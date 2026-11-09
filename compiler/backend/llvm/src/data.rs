//! Optionals, unions, errors and decimals, as in the Cranelift backend
//! (`cranelift/src/data.rs`, `cranelift/src/errors.rs`). `none` is 0; a
//! present optional is the value itself, or a box for a number. A union
//! value is a record that knows its variant. A function that can fail
//! returns its error after its value: 0 when it succeeded. Decimals are
//! objects the runtime computes with. A closure is an object holding its
//! function's address, then the values it captured; calling it passes
//! the closure itself first.

use crate::func::{FnEmit, fn_name};
use crate::module::llvm_type;
use kavach_kir::{BinOp, Expr, ExprKind, Local, Shape, Type};
use kavach_runtime_native::heap::{FIELDS, FIXED, KIND, TAG};

impl FnEmit<'_> {
    /// Whether values of type `t` are unions or errors, whose fields the
    /// runtime reads (an error may be wrapped in `context`).
    pub(crate) fn variants(&self, t: Type) -> bool {
        matches!(t, Type::Ref(id) if matches!(
            self.shapes[id as usize],
            Shape::Union { .. } | Shape::Error
        ))
    }

    /// Whether values of type `t` are unions (not errors).
    fn is_union(&self, t: Type) -> bool {
        matches!(t, Type::Ref(id) if matches!(self.shapes[id as usize], Shape::Union { .. }))
    }

    /// Whether union value `obj` is a fixed object, with its variant in
    /// its header and its fields after it. Only an error with `context`
    /// added (spec 5.7) is not: the runtime looks inside it.
    fn is_fixed_obj(&mut self, obj: &str) -> String {
        let word = self.load_at(obj, TAG);
        let kind = self.val(format!("lshr i64 {word}, {}", 8 * (KIND - TAG)));
        let kind = self.val(format!("and i64 {kind}, 65535"));
        self.val(format!("icmp eq i64 {kind}, {FIXED}"))
    }

    /// The variant number of union value `obj` (borrowed).
    fn variant_tag(&mut self, obj: &str) -> String {
        let result = self.slot(Type::I64);
        let (fast, slow, done) = (self.label(), self.label(), self.label());
        let fixed = self.is_fixed_obj(obj);
        self.cond_br(&fixed, &fast, &slow);
        self.start(&fast);
        let word = self.load_at(obj, TAG);
        let tag = self.val(format!("and i64 {word}, 4294967295"));
        self.ins(format!("store i64 {tag}, ptr {result}"));
        self.br(&done);
        self.start(&slow);
        let tag = self.call_rt("kv_variant_of", "i64", &[("i64", obj.to_string())], false);
        self.ins(format!("store i64 {tag}, ptr {result}"));
        self.br(&done);
        self.start(&done);
        self.val(format!("load i64, ptr {result}"))
    }

    /// Field `i` (of type `ty`) of union value `obj` (borrowed), as an
    /// owned slot.
    fn variant_field(&mut self, obj: &str, i: u32, ty: Type) -> String {
        let result = self.slot(Type::I64);
        let (fast, slow, done) = (self.label(), self.label(), self.label());
        let fixed = self.is_fixed_obj(obj);
        self.cond_br(&fixed, &fast, &slow);
        self.start(&fast);
        let slot = self.field(obj, i);
        if ty.is_ref() {
            self.retain(&slot);
        }
        self.ins(format!("store i64 {slot}, ptr {result}"));
        self.br(&done);
        self.start(&slow);
        let args = [("i64", obj.to_string()), ("i64", i.to_string())];
        let slot = self.call_rt("kv_field_of", "i64", &args, false);
        self.ins(format!("store i64 {slot}, ptr {result}"));
        self.br(&done);
        self.start(&done);
        self.val(format!("load i64, ptr {result}"))
    }

    fn is_decimal(&self, t: Type) -> bool {
        matches!(t, Type::Ref(id) if matches!(self.shapes[id as usize], Shape::Decimal | Shape::BigInt))
    }

    /// Optionals, unions, errors and decimals; `None` for any other
    /// expression.
    pub(crate) fn data_expr(&mut self, e: &Expr) -> Option<Option<String>> {
        let arith = [BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div, BinOp::Rem];
        let v = match &e.kind {
            // A callback's bare address, which C calls (spec 10.7).
            ExprKind::Closure {
                func,
                env: Type::Unit,
                ..
            } => Some(self.val(format!("ptrtoint ptr @{} to i64", fn_name(*func as usize)))),
            ExprKind::Closure {
                func,
                env,
                captures,
            } => {
                let n = (captures.len() + 1).to_string();
                let args = [("i64", Self::code(*env)), ("i64", n)];
                let obj = self.call_rt("kv_list", "i64", &args, false);
                let addr = format!("ptrtoint (ptr @{} to i64)", fn_name(*func as usize));
                self.store_item_at(&addr, &obj, FIELDS);
                for (i, c) in captures.iter().enumerate() {
                    let slot = self.slot_of(c);
                    self.store_item_at(&slot, &obj, FIELDS + 8 * (i as i32 + 1));
                }
                Some(obj)
            }
            ExprKind::CallValue(callee, args) => self.call_value(callee, args, e.ty, false, e.line),
            ExprKind::Decimal(text) => {
                let (ptr, len) = self.m.text(text);
                let args = [
                    ("i64", ptr),
                    ("i64", len.to_string()),
                    ("i64", Self::code(e.ty)),
                ];
                Some(self.call_rt("kv_dec_new", "i64", &args, false))
            }
            ExprKind::Binary(op, l, r) if self.is_decimal(e.ty) && arith.contains(op) => {
                let i = arith.iter().position(|o| o == op).unwrap_or(0);
                let (a, b) = (self.expr(l)?, self.expr(r)?);
                let args = [
                    ("i64", i.to_string()),
                    ("i64", a),
                    ("i64", b),
                    ("i64", Self::code(e.ty)),
                    ("i32", e.line.to_string()),
                ];
                Some(self.call_rt("kv_dec_arith", "i64", &args, false))
            }
            ExprKind::Unary(_, x) if self.is_decimal(x.ty) => {
                let v = self.expr(x)?;
                let args = [("i64", v), ("i64", Self::code(e.ty))];
                Some(self.call_rt("kv_dec_neg", "i64", &args, false))
            }
            // To and from `Decimal`, the only heap type that converts.
            ExprKind::Convert(x, _) if x.ty != e.ty && self.is_decimal(e.ty) => {
                let v = self.expr(x)?;
                let args = [
                    ("i64", self.pack(&v, x.ty)),
                    ("i64", Self::code(x.ty)),
                    ("i64", Self::code(e.ty)),
                    ("i32", e.line.to_string()),
                ];
                Some(self.call_rt("kv_dec_from", "i64", &args, false))
            }
            ExprKind::Convert(x, _) if x.ty != e.ty && self.is_decimal(x.ty) => {
                let v = self.expr(x)?;
                let args = [
                    ("i64", v),
                    ("i64", Self::code(e.ty)),
                    ("i32", e.line.to_string()),
                ];
                let slot = self.call_rt("kv_dec_to", "i64", &args, false);
                Some(self.unpack(&slot, e.ty))
            }
            ExprKind::None => Some("0".to_string()),
            ExprKind::Some(x) => {
                let v = self.expr(x)?;
                if x.ty.is_ref() {
                    Some(v)
                } else {
                    let args = [("i64", self.pack(&v, x.ty)), ("i64", Self::code(e.ty))];
                    Some(self.call_rt("kv_some", "i64", &args, false))
                }
            }
            ExprKind::IsSome(x) => {
                let p = self.expr(x)?;
                let yes = self.call_rt("kv_is_some", "i8", &[("i64", p)], false);
                Some(self.val(format!("icmp ne i8 {yes}, 0")))
            }
            ExprKind::Unwrap(x) => {
                let p = self.expr(x)?;
                let args = [
                    ("i64", p),
                    ("i64", Self::code(x.ty)),
                    ("i32", e.line.to_string()),
                ];
                let slot = self.call_rt("kv_unwrap", "i64", &args, false);
                Some(self.unpack(&slot, e.ty))
            }
            ExprKind::HasVariant(x, i) if self.is_union(x.ty) => {
                let (obj, owned) = self.borrow(x, true)?;
                let tag = self.variant_tag(&obj);
                if owned {
                    self.release(&obj);
                }
                Some(self.val(format!("icmp eq i64 {tag}, {i}")))
            }
            ExprKind::HasVariant(x, i) => {
                let obj = self.expr(x)?;
                let args = [("i64", obj), ("i64", i.to_string())];
                let yes = self.call_rt("kv_has_variant", "i8", &args, false);
                Some(self.val(format!("icmp ne i8 {yes}, 0")))
            }
            ExprKind::Record { variant, fields } if self.variants(e.ty) => {
                let slots: Vec<String> = fields.iter().map(|f| self.slot_of(f)).collect();
                Some(self.new_record(e.ty, *variant, &slots))
            }
            ExprKind::GetField(x, i) if self.is_union(x.ty) => {
                let (obj, owned) = self.borrow(x, true)?;
                let slot = self.variant_field(&obj, *i, e.ty);
                if owned {
                    self.release(&obj);
                }
                Some(self.unpack(&slot, e.ty))
            }
            ExprKind::GetField(x, i) if self.variants(x.ty) => {
                let obj = self.expr(x)?;
                let args = [("i64", obj), ("i64", i.to_string())];
                let slot = self.call_rt("kv_get", "i64", &args, false);
                Some(self.unpack(&slot, e.ty))
            }
            ExprKind::Context(text, error) => {
                let (t, err) = (self.expr(text)?, self.expr(error)?);
                let args = [("i64", t), ("i64", err), ("i64", Self::code(e.ty))];
                Some(self.call_rt("kv_context", "i64", &args, false))
            }
            ExprKind::Fail(x) => {
                if let Some(err) = self.expr(x) {
                    self.leave(None, Some(err));
                }
                Self::zero(e.ty)
            }
            _ => return None,
        };
        Some(v)
    }

    /// `value, error = call`: something that can fail, run for its value
    /// and its error (0 when it succeeded).
    pub(crate) fn let_result(&mut self, value: Local, error: Local, call: &Expr) {
        let (v, err) = match &call.kind {
            ExprKind::Call(f, args) if self.m.fails[*f as usize] => {
                let Some(result) = self.call(*f as usize, args, call.ty, call.line) else {
                    return;
                };
                if call.ty == Type::Unit {
                    (None, result)
                } else {
                    let ty = format!("{{ {}, i64 }}", llvm_type(call.ty));
                    let v = self.val(format!("extractvalue {ty} {result}, 0"));
                    (Some(v), self.val(format!("extractvalue {ty} {result}, 1")))
                }
            }
            // Through a function value: in a `LetResult`, a failing one (an
            // interface's failing method, `kir/src/lower_iface.rs`).
            ExprKind::CallValue(callee, args) => {
                let Some(result) = self.call_value(callee, args, call.ty, true, call.line) else {
                    return;
                };
                if call.ty == Type::Unit {
                    (None, result)
                } else {
                    let ty = format!("{{ {}, i64 }}", llvm_type(call.ty));
                    let v = self.val(format!("extractvalue {ty} {result}, 0"));
                    (Some(v), self.val(format!("extractvalue {ty} {result}, 1")))
                }
            }
            ExprKind::Parse(text) => {
                let Some(t) = self.expr(text) else { return };
                let args = [
                    ("i64", t),
                    ("i64", Self::code(call.ty)),
                    ("i64", Self::code(text.ty)),
                ];
                let slot = self.call_rt("kv_parse", "i64", &args, false);
                let err = self.call_rt("kv_take_error", "i64", &[], false);
                (Some(self.unpack(&slot, call.ty)), err)
            }
            ExprKind::Rt(op, args) if op.can_fail() => {
                let mut slots: Vec<String> = args.iter().map(|a| self.slot_of(a)).collect();
                slots.resize(3, "0".into());
                let slot = self.rt_op(*op, &slots[0], &slots[1], &slots[2], call.ty, call.line);
                let err = self.call_rt("kv_take_error", "i64", &[], false);
                let v = (call.ty != Type::Unit).then(|| self.unpack(&slot, call.ty));
                (v, err)
            }
            // A call that cannot fail after all: it always succeeds.
            _ => (self.expr(call), "0".to_string()),
        };
        if let Some(v) = v {
            self.set_local(value, &v);
        }
        self.set_local(error, &err);
    }
}
