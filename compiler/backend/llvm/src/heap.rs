//! Heap values in generated code, as in the Cranelift backend
//! (`cranelift/src/inline.rs`): the same object layout, the same
//! reference counting, so both backends behave the same.
//!
//! An object pointer is an `i64`, as the runtime passes it; reading a
//! field turns it into a pointer first.

use crate::func::FnEmit;
use crate::module::llvm_type;
use kavach_kir::{Expr, ExprKind, Local, Shape, Type};
use kavach_runtime_native::heap::RC;

/// The alias tags of header words and item words (`module.rs`).
pub(crate) const HEADER: &str = "!tbaa !3";
pub(crate) const ITEM: &str = "!tbaa !5";

impl FnEmit<'_> {
    /// The address `p + offset` as a pointer.
    pub(crate) fn addr(&mut self, p: &str, offset: i32) -> String {
        let base = self.val(format!("inttoptr i64 {p} to ptr"));
        if offset == 0 {
            return base;
        }
        self.val(format!("getelementptr i8, ptr {base}, i64 {offset}"))
    }

    /// The header word at `p + offset` (reference count, length, buffer…).
    /// Headers and items are tagged apart (`HEADER`, `ITEM`), so LLVM
    /// knows a store into a list's buffer doesn't change its length.
    pub(crate) fn load_at(&mut self, p: &str, offset: i32) -> String {
        let a = self.addr(p, offset);
        self.val(format!("load i64, ptr {a}, {HEADER}"))
    }

    pub(crate) fn store_at(&mut self, v: &str, p: &str, offset: i32) {
        let a = self.addr(p, offset);
        self.ins(format!("store i64 {v}, ptr {a}, {HEADER}"));
    }

    /// The item word at `p + offset`: a list item or a record field.
    pub(crate) fn load_item_at(&mut self, p: &str, offset: i32) -> String {
        let a = self.addr(p, offset);
        self.val(format!("load i64, ptr {a}, {ITEM}"))
    }

    pub(crate) fn store_item_at(&mut self, v: &str, p: &str, offset: i32) {
        let a = self.addr(p, offset);
        self.ins(format!("store i64 {v}, ptr {a}, {ITEM}"));
    }

    /// Takes a new reference to `p` (0 means `none`: nothing to count).
    pub(crate) fn retain(&mut self, p: &str) {
        let (count, done) = (self.label(), self.label());
        let set = self.val(format!("icmp ne i64 {p}, 0"));
        self.cond_br(&set, &count, &done);
        self.start(&count);
        let rc = self.load_at(p, RC);
        let more = self.val(format!("add i64 {rc}, 1"));
        self.store_at(&more, p, RC);
        self.br(&done);
        self.start(&done);
    }

    /// Drops a reference to `p`, freeing it when it was the last one.
    pub(crate) fn release(&mut self, p: &str) {
        let (count, free, done) = (self.label(), self.label(), self.label());
        let set = self.val(format!("icmp ne i64 {p}, 0"));
        self.cond_br(&set, &count, &done);
        self.start(&count);
        let rc = self.load_at(p, RC);
        let less = self.val(format!("sub i64 {rc}, 1"));
        self.store_at(&less, p, RC);
        let last = self.val(format!("icmp eq i64 {less}, 0"));
        self.cond_br(&last, &free, &done);
        self.start(&free);
        self.m.declare("kv_free", "void", &["i64"], false);
        self.ins(format!("call void @kv_free(i64 {p})"));
        self.br(&done);
        self.start(&done);
    }

    /// The value of `e`, borrowed when it is a place and `may_borrow` (no
    /// new reference): the value and whether the caller owns it.
    pub(crate) fn borrow(&mut self, e: &Expr, may_borrow: bool) -> Option<(String, bool)> {
        if may_borrow && self.is_place(e) {
            return Some((self.place(e), false));
        }
        Some((self.expr(e)?, true))
    }

    /// Whether `e` is a local, or an item of a list in one, reached without
    /// changing any local: the local keeps it alive, so it can be read
    /// without taking a reference.
    pub(crate) fn is_place(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Local(_) => true,
            // Moving out of a borrowing local only reads it (`for_each`
            // allows that only for reading a field).
            ExprKind::Move(l) => self.borrowed[*l as usize],
            ExprKind::Index(base, i) | ExprKind::InRange(base, i) => {
                self.plain_list(base.ty) && plain(i) && self.is_place(base)
            }
            ExprKind::GetField(base, _) => {
                (self.fixed(base.ty) && (self.is_place(base) || self.is_flat_item(base)))
                    || self.borrowed_closure(base)
            }
            _ => false,
        }
    }

    /// Whether `e` is a closure a closure's function borrows: its fields
    /// are read in place, the closure kept alive by the caller.
    fn borrowed_closure(&self, e: &Expr) -> bool {
        let closure = |t: Type| {
            matches!(t, Type::Ref(id)
                if matches!(self.shapes[id as usize], kavach_kir::Shape::Closure(_)))
        };
        matches!(e.kind, ExprKind::Local(l) | ExprKind::Move(l) if self.borrowed[l as usize])
            && closure(e.ty)
    }

    /// Whether `e` is an item of a flat list that is a place: its fields
    /// are read in the list's buffer, through a view (`records.rs`).
    fn is_flat_item(&self, e: &Expr) -> bool {
        matches!(&e.kind, ExprKind::Index(base, i)
            if self.flat_width(base.ty) > 0 && plain(i) && self.is_place(base))
    }

    /// A view of `e`, an item of a flat list (see `is_flat_item`).
    fn flat_item_view(&mut self, e: &Expr) -> String {
        let ExprKind::Index(base, i) = &e.kind else {
            unreachable!("an item of a flat list")
        };
        let checked = !self.known_in_range(base, i);
        let list = self.place(base);
        let (ptr, len) = self.view(base, &list, false);
        let index = self.expr(i).expect("an index");
        if checked {
            self.check_index(&index, &len, e.line);
        }
        let width = self.flat_width(base.ty);
        let addr = self.flat_addr(&ptr, &index, width);
        self.flat_view(&addr)
    }

    /// The value of a place (see `is_place`), borrowed.
    pub(crate) fn place(&mut self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Index(base, i) | ExprKind::InRange(base, i) => {
                let checked =
                    matches!(e.kind, ExprKind::Index(..)) && !self.known_in_range(base, i);
                let list = self.place(base);
                let view = self.view(base, &list, false);
                let index = self.expr(i).expect("an index");
                self.item(view, &index, e.ty, e.line, checked)
            }
            ExprKind::GetField(base, f) => {
                let obj = if self.is_flat_item(base) {
                    self.flat_item_view(base)
                } else {
                    self.place(base)
                };
                let slot = self.field(&obj, *f);
                self.unpack(&slot, e.ty)
            }
            ExprKind::Local(l) | ExprKind::Move(l) => self.load_local(*l).expect("a value"),
            _ => unreachable!("a place"),
        }
    }

    /// A 64-bit slot as a value of type `ty`.
    pub(crate) fn unpack(&mut self, slot: &str, ty: Type) -> String {
        match ty {
            Type::F64 => self.val(format!("bitcast i64 {slot} to double")),
            Type::F32 => {
                let bits = self.val(format!("trunc i64 {slot} to i32"));
                self.val(format!("bitcast i32 {bits} to float"))
            }
            t if t.is_ref() || t.bits() == 64 => slot.to_string(),
            t => self.val(format!("trunc i64 {slot} to {}", llvm_type(t))),
        }
    }

    /// `v` as a 64-bit slot, as lists and the runtime keep values.
    pub(crate) fn pack(&mut self, v: &str, ty: Type) -> String {
        match ty {
            Type::F64 => self.val(format!("bitcast double {v} to i64")),
            Type::F32 => {
                let bits = self.val(format!("bitcast float {v} to i32"));
                self.val(format!("zext i32 {bits} to i64"))
            }
            Type::Unit => "0".into(),
            t if t.is_ref() || t.bits() == 64 => v.to_string(),
            t if t.is_signed() => self.val(format!("sext {} {v} to i64", llvm_type(t))),
            t => self.val(format!("zext {} {v} to i64", llvm_type(t))),
        }
    }

    /// A built-in method, through its group's runtime entry (`RtOp::entry`).
    pub(crate) fn rt_op(
        &mut self,
        op: kavach_kir::RtOp,
        a: &str,
        b: &str,
        c: &str,
        ty: Type,
        line: u32,
    ) -> String {
        let args = [
            ("i64", op.code().to_string()),
            ("i64", a.to_string()),
            ("i64", b.to_string()),
            ("i64", c.to_string()),
            ("i64", Self::code(ty)),
            ("i32", line.to_string()),
        ];
        self.call_rt(op.entry(), "i64", &args, false)
    }

    /// Whether `t` is a map whose keys are integers, booleans or
    /// characters (compared as slots).
    pub(crate) fn int_keys(&self, t: Type) -> bool {
        matches!(t, Type::Ref(id) if matches!(
            self.shapes[id as usize],
            Shape::Map(k, _) if !k.is_ref() && !k.is_float()
        ))
    }

    /// Whether `t` is a map with text keys and values held by reference
    /// (not optionals): `m[k]` is then the value or 0, read in place.
    pub(crate) fn text_keys(&self, t: Type) -> bool {
        let shape = |t: Type| match t {
            Type::Ref(id) => Some(&self.shapes[id as usize]),
            _ => None,
        };
        match shape(t) {
            Some(Shape::Map(k, v)) => {
                matches!(shape(*k), Some(Shape::Str))
                    && v.is_ref()
                    && !matches!(shape(*v), Some(Shape::Optional(_)))
            }
            _ => false,
        }
    }

    /// Releases every heap local (the function is returning).
    pub(crate) fn release_locals(&mut self) {
        let refs: Vec<Local> = (0..self.f.locals.len() as Local)
            .filter(|l| self.f.locals[*l as usize].is_ref() && !self.borrowed[*l as usize])
            .collect();
        for l in refs {
            let v = self.load_local(l).expect("a heap value");
            self.release(&v);
        }
    }
}

/// Whether evaluating `e` cannot change any local (as in the Cranelift
/// backend's `data::plain`).
pub(crate) fn plain(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Local(_) => true,
        ExprKind::Binary(_, a, b) | ExprKind::Fast(_, a, b) => plain(a) && plain(b),
        ExprKind::Unary(_, a) | ExprKind::Convert(a, _) => plain(a),
        _ => false,
    }
}
