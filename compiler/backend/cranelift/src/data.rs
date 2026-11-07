//! Code for heap values: text, lists, tuples, records, unions and
//! optionals. Values cross into the runtime as 64-bit slots.

use crate::abi::cl_type;
use crate::codegen::FnGen;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};
use kavach_kir::{BinOp, Expr, ExprKind, RtOp, Shape, Type};
use kavach_runtime_native::heap::{LEN, PTR};

impl FnGen<'_, '_> {
    /// A value as a 64-bit slot.
    pub(crate) fn pack(&mut self, v: Value, t: Type) -> Value {
        match t {
            Type::F64 => self.b.ins().bitcast(types::I64, MemFlags::new(), v),
            Type::F32 => {
                let bits = self.b.ins().bitcast(types::I32, MemFlags::new(), v);
                self.b.ins().uextend(types::I64, bits)
            }
            Type::Bool | Type::Char => self.b.ins().uextend(types::I64, v),
            Type::Ref(_) => v,
            t => self.widen(v, t),
        }
    }

    /// A slot as a value of type `t`.
    pub(crate) fn unpack(&mut self, s: Value, t: Type) -> Value {
        match (t, cl_type(t)) {
            (Type::F64, _) => self.b.ins().bitcast(types::F64, MemFlags::new(), s),
            (Type::F32, _) => {
                let bits = self.b.ins().ireduce(types::I32, s);
                self.b.ins().bitcast(types::F32, MemFlags::new(), bits)
            }
            (_, Some(ct)) if ct.bits() < 64 => self.b.ins().ireduce(ct, s),
            _ => s,
        }
    }

    pub(crate) fn slot_of(&mut self, e: &Expr) -> Value {
        match self.expr(e) {
            Some(v) => self.pack(v, e.ty),
            None => self.int(types::I64, 0),
        }
    }

    pub(crate) fn data_expr(&mut self, e: &Expr) -> Option<Value> {
        let line = self.i32(e.line as i64);
        let v = match &e.kind {
            ExprKind::Str(text) => {
                let (ptr, len) = self.constant_text(text);
                let code = self.code(e.ty);
                self.call_rt("kv_str_new", &[ptr, len, code])[0]
            }
            ExprKind::Format(parts) => {
                let mut acc = self.int(types::I64, 0);
                let str_code = self.code(e.ty);
                for p in parts {
                    if let ExprKind::Str(text) = &p.kind {
                        let (ptr, len) = self.constant_text(text);
                        acc = self.call_rt("kv_format_text", &[acc, ptr, len, str_code])[0];
                        continue;
                    }
                    let slot = self.slot_of(p);
                    let code = self.code(p.ty);
                    acc = self.call_rt("kv_format", &[acc, slot, code, str_code])[0];
                }
                acc
            }
            ExprKind::Decimal(text) => {
                let (ptr, len) = self.constant_text(text);
                let code = self.code(e.ty);
                self.call_rt("kv_dec_new", &[ptr, len, code])[0]
            }
            ExprKind::Context(text, error) => {
                let (t, err) = (self.expr(text)?, self.expr(error)?);
                let code = self.code(e.ty);
                self.call_rt("kv_context", &[t, err, code])[0]
            }
            ExprKind::None => self.int(types::I64, 0),
            ExprKind::Some(x) => {
                let v = self.expr(x)?;
                if x.ty.is_ref() {
                    v
                } else {
                    let slot = self.pack(v, x.ty);
                    let code = self.code(e.ty);
                    self.call_rt("kv_some", &[slot, code])[0]
                }
            }
            ExprKind::IsSome(x) => {
                let p = self.expr(x)?;
                self.call_rt("kv_is_some", &[p])[0]
            }
            ExprKind::Unwrap(x) => {
                let p = self.expr(x)?;
                let code = self.code(x.ty);
                let slot = self.call_rt("kv_unwrap", &[p, code, line])[0];
                self.unpack(slot, e.ty)
            }
            ExprKind::Record { variant, fields } => {
                let obj = match self.alloc_fixed(e.ty, *variant, fields.len()) {
                    Some(obj) => obj,
                    None => {
                        let (code, variant, n) = (
                            self.code(e.ty),
                            self.int(types::I64, *variant as i64),
                            self.int(types::I64, fields.len() as i64),
                        );
                        self.call_rt("kv_record", &[code, variant, n])[0]
                    }
                };
                self.fill(obj, fields, false);
                obj
            }
            // Each item of a list of text is stored by the runtime.
            ExprKind::List(items) if self.text_list(e.ty) => {
                let (code, n) = (self.code(e.ty), self.int(types::I64, items.len() as i64));
                let obj = self.call_rt("kv_list", &[code, n])[0];
                for (i, item) in items.iter().enumerate() {
                    let (index, slot) = (self.int(types::I64, i as i64), self.slot_of(item));
                    self.call_rt("kv_init", &[obj, index, slot]);
                }
                obj
            }
            ExprKind::List(items) if self.flat_width(e.ty) > 0 => {
                let (code, n) = (self.code(e.ty), self.int(types::I64, items.len() as i64));
                let obj = self.call_rt("kv_list", &[code, n])[0];
                let width = self.flat_width(e.ty);
                for (i, item) in items.iter().enumerate() {
                    self.flat_init(obj, i, item, width);
                }
                obj
            }
            ExprKind::List(items) | ExprKind::Tuple(items) => {
                let list = self.is_list(e.ty);
                // A packed list (decisions 0030 and 0045): the runtime
                // packs each item.
                if self.item_bytes_of(e.ty) > 0 {
                    let (code, n) = (self.code(e.ty), self.int(types::I64, items.len() as i64));
                    let obj = self.call_rt("kv_list", &[code, n])[0];
                    for (i, item) in items.iter().enumerate() {
                        let slot = self.slot_of(item);
                        let at = self.int(types::I64, i as i64);
                        self.call_rt("kv_init", &[obj, at, slot]);
                    }
                    return Some(obj);
                }
                let obj = match self.alloc_fixed(e.ty, 0, items.len()) {
                    Some(obj) => obj,
                    None => {
                        let (code, n) = (self.code(e.ty), self.int(types::I64, items.len() as i64));
                        self.call_rt("kv_list", &[code, n])[0]
                    }
                };
                self.fill(obj, items, list);
                obj
            }
            ExprKind::GetField(x, i) => {
                let (obj, owned) = self.borrow(x, true)?;
                self.read_field(obj, owned, x.ty, *i, e.ty)
            }
            ExprKind::SetField(x, i, value) => {
                let obj = self.expr(x)?;
                let slot = self.slot_of(value);
                let index = self.int(types::I64, *i as i64);
                self.call_rt("kv_set", &[obj, index, slot])[0]
            }
            ExprKind::HasVariant(x, i) => {
                let obj = self.expr(x)?;
                let index = self.int(types::I64, *i as i64);
                self.call_rt("kv_has_variant", &[obj, index])[0]
            }
            ExprKind::Index(x, i) if self.is_list(x.ty) => {
                let (obj, owned) = self.borrow(x, plain(i))?;
                let view = if owned {
                    (self.load(obj, PTR), self.load(obj, LEN))
                } else {
                    self.list_view(x, obj)
                };
                let index = self.slot_of(i);
                if self.text_list(x.ty) {
                    // An item of a list of text, as a text object.
                    if owned || !self.known_in_range(x, i) {
                        self.check_index(view, index, e.line);
                    }
                    let addr = self.text_addr(view.0, index);
                    let item = self.text_box(addr, e.ty);
                    if owned {
                        self.release(obj);
                    }
                    return Some(item);
                }
                if let width @ 1.. = self.flat_width(x.ty) {
                    // An item of a flat list, used whole: boxed.
                    if owned || !self.known_in_range(x, i) {
                        self.check_index(view, index, e.line);
                    }
                    let addr = self.flat_addr(view.0, index, width);
                    let item = self.flat_box(addr, e.ty, width);
                    if owned {
                        self.release(obj);
                    }
                    return Some(item);
                }
                if !owned && self.known_in_range(x, i) {
                    let item = self.item_at(view.0, index, e.ty);
                    if e.ty.is_ref() {
                        self.retain(item);
                    }
                    return Some(item);
                }
                self.read_item(obj, owned, view, index, e.ty, e.line)
            }
            // Element `n` of a tuple.
            ExprKind::Index(x, i) if matches!(i.kind, ExprKind::Int(_)) => {
                let ExprKind::Int(n) = i.kind else {
                    unreachable!("a constant index")
                };
                let (obj, owned) = self.borrow(x, true)?;
                self.read_field(obj, owned, x.ty, n as u32, e.ty)
            }
            ExprKind::Index(x, i) => {
                let obj = self.expr(x)?;
                let index = self.slot_of(i);
                let slot = self.call_rt("kv_index", &[obj, index, line])[0];
                self.unpack(slot, e.ty)
            }
            ExprKind::Rt(RtOp::MapGetOr, args) if self.int_keys(args[0].ty) => {
                let (map, owned) = self.borrow(&args[0], true)?;
                let (key, fallback) = (self.slot_of(&args[1]), self.slot_of(&args[2]));
                let slot = self.call_rt("kv_map_get_int", &[map, key, fallback])[0];
                if owned {
                    self.release(map);
                }
                self.unpack(slot, e.ty)
            }
            ExprKind::Rt(RtOp::StrLen, args) if self.is_text_view(&args[0]) => {
                let addr = self.text_view(&args[0])?;
                self.text_len(addr)
            }
            ExprKind::Rt(RtOp::StrLen, args) => {
                let (text, owned) = self.borrow(&args[0], true)?;
                let n = self.call_rt("kv_str_len", &[text])[0];
                if owned {
                    self.release(text);
                }
                n
            }
            // Only a hint (`kir/src/prefetch.rs`): Cranelift has no prefetch.
            ExprKind::Rt(RtOp::ListPrefetch | RtOp::MapPrefetch, _) => return None,
            ExprKind::Rt(RtOp::ListLen, args) if self.is_list(args[0].ty) => {
                let (obj, owned) = self.borrow(&args[0], true)?;
                self.list_len(obj, owned)
            }
            ExprKind::Rt(RtOp::ListPush, args) if self.text_list(args[0].ty) => {
                let (list, item) = (self.slot_of(&args[0]), self.slot_of(&args[1]));
                self.text_push(list, item, e.ty, e.line)
            }
            ExprKind::Rt(RtOp::ListPush, args) if self.flat_width(args[0].ty) > 0 => {
                let list = self.slot_of(&args[0]);
                let item = self.flat_item(&args[1]);
                self.flat_push(list, item, e.ty, e.line)
            }
            ExprKind::Rt(RtOp::ListPush, args) if self.is_list(args[0].ty) => {
                let (list, item) = (self.slot_of(&args[0]), self.slot_of(&args[1]));
                self.list_push(list, item, e.ty, e.line)
            }
            ExprKind::Rt(RtOp::PtrRead | RtOp::PtrWrite, args) => {
                return self.ptr_access(e.ty, args);
            }
            // `math.sqrt`, `floor` and `ceil`: one exactly rounded
            // instruction, not a runtime call. `round` stays a call:
            // Cranelift's `nearest` rounds ties to even, and Kavach
            // rounds them away from zero.
            ExprKind::Rt(op @ (RtOp::MathSqrt | RtOp::MathFloor | RtOp::MathCeil), args) => {
                let x = self.expr(&args[0])?;
                return Some(match op {
                    RtOp::MathSqrt => self.b.ins().sqrt(x),
                    RtOp::MathFloor => self.b.ins().floor(x),
                    _ => self.b.ins().ceil(x),
                });
            }
            ExprKind::Rt(op, args) => {
                // An op without operands gets 0 for its first.
                let a = match args.first() {
                    Some(x) => self.slot_of(x),
                    None => self.int(types::I64, 0),
                };
                let rest = args.get(1..).unwrap_or(&[]);
                let v = self.rt_call(*op, a, rest, e.ty, e.line);
                return (e.ty != Type::Unit).then_some(v);
            }
            // A callback's bare address, which C calls (spec 10.7).
            ExprKind::Closure {
                func,
                env: Type::Unit,
                ..
            } => {
                let fref = self
                    .module
                    .declare_func_in_func(self.funcs[*func as usize], self.b.func);
                self.b.ins().func_addr(types::I64, fref)
            }
            ExprKind::Closure {
                func,
                env,
                captures,
            } => {
                let (code, n) = (
                    self.code(*env),
                    self.int(types::I64, captures.len() as i64 + 1),
                );
                let obj = self.call_rt("kv_list", &[code, n])[0];
                let fref = self
                    .module
                    .declare_func_in_func(self.funcs[*func as usize], self.b.func);
                let addr = self.b.ins().func_addr(types::I64, fref);
                self.init_field(obj, 0, addr);
                for (i, c) in captures.iter().enumerate() {
                    let slot = self.slot_of(c);
                    self.init_field(obj, i + 1, slot);
                }
                obj
            }
            ExprKind::CallValue(callee, args) => {
                let results = self.call_value(callee, args, e.ty, false, e.line)?;
                return results.first().copied();
            }
            ExprKind::Map(pairs) => {
                let code = self.code(e.ty);
                let mut map = self.call_rt("kv_map_new", &[code])[0];
                for (k, v) in pairs {
                    let (ks, vs) = (self.slot_of(k), self.slot_of(v));
                    map = self.rt_slots(RtOp::MapInsert, map, ks, vs, e.ty, e.line);
                }
                map
            }
            _ => return None,
        };
        Some(v)
    }

    /// A built-in method on slots `a` and `b`, giving a value of type `ty`.
    /// A built-in method on slot `a` and the values of `rest` (up to two
    /// more), giving a value of type `ty`.
    pub(crate) fn rt_call(
        &mut self,
        op: RtOp,
        a: Value,
        rest: &[Expr],
        ty: Type,
        line: u32,
    ) -> Value {
        let mut slots = rest.iter().map(|x| self.slot_of(x)).collect::<Vec<_>>();
        while slots.len() < 2 {
            slots.push(self.int(types::I64, 0));
        }
        self.rt_slots(op, a, slots[0], slots[1], ty, line)
    }

    /// A built-in method on three slots.
    pub(crate) fn rt_slots(
        &mut self,
        op: RtOp,
        a: Value,
        b: Value,
        c: Value,
        ty: Type,
        line: u32,
    ) -> Value {
        let entry = op.entry();
        let (op, code) = (self.int(types::I64, op.code()), self.code(ty));
        let line = self.i32(line as i64);
        let slot = self.call_rt(entry, &[op, a, b, c, code, line])[0];
        self.unpack(slot, ty)
    }

    /// Fills the slots of a new list or fixed object, in order.
    fn fill(&mut self, obj: Value, items: &[Expr], list: bool) {
        for (i, item) in items.iter().enumerate() {
            let slot = self.slot_of(item);
            if list {
                let slots = self.load(obj, PTR);
                let offset = 8 * i as i32;
                self.b.ins().store(MemFlags::trusted(), slot, slots, offset);
            } else {
                self.init_field(obj, i, slot);
            }
        }
    }
}

impl FnGen<'_, '_> {
    /// Whether `t` is a map whose keys are integers, booleans or
    /// characters (compared as slots).
    fn int_keys(&self, t: Type) -> bool {
        matches!(t, Type::Ref(id) if matches!(
            self.shapes[id as usize],
            Shape::Map(k, _) if !k.is_ref() && !k.is_float()
        ))
    }
}

/// Whether evaluating `e` cannot change any local: then a local list can be
/// read while `e` is computed without taking a reference to it.
pub(crate) fn plain(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Local(_) => true,
        ExprKind::Binary(_, a, b) => plain(a) && plain(b),
        ExprKind::Unary(_, a) | ExprKind::Convert(a, _) => plain(a),
        _ => false,
    }
}

impl FnGen<'_, '_> {
    /// Operators on heap values: `==`, `!=`, text `+`, ordering of text
    /// and decimals, and decimal arithmetic.
    pub(crate) fn ref_binary(
        &mut self,
        op: BinOp,
        ty: Type,
        a: Value,
        b: Value,
        line: u32,
    ) -> Value {
        let arith = [BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div, BinOp::Rem];
        // `BigInt` goes through the same calls; the runtime tells them apart.
        let is_decimal = matches!(ty, Type::Ref(id) if matches!(self.shapes[id as usize], Shape::Decimal | Shape::BigInt));
        if let (true, Some(i)) = (is_decimal, arith.iter().position(|o| *o == op)) {
            let (op, code, line) = (
                self.int(types::I64, i as i64),
                self.code(ty),
                self.i32(line as i64),
            );
            return self.call_rt("kv_dec_arith", &[op, a, b, code, line])[0];
        }
        match op {
            BinOp::Eq | BinOp::Ne => {
                let code = self.code(ty);
                let same = self.call_rt("kv_eq", &[a, b, code])[0];
                if op == BinOp::Eq {
                    same
                } else {
                    self.b.ins().bxor_imm(same, 1)
                }
            }
            BinOp::Add => self.call_rt("kv_concat", &[a, b])[0],
            _ => {
                let order = self.call_rt("kv_cmp", &[a, b])[0];
                let cc = match op {
                    BinOp::Lt => IntCC::SignedLessThan,
                    BinOp::Le => IntCC::SignedLessThanOrEqual,
                    BinOp::Gt => IntCC::SignedGreaterThan,
                    _ => IntCC::SignedGreaterThanOrEqual,
                };
                self.b.ins().icmp_imm(cc, order, 0)
            }
        }
    }
}
