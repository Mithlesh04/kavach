//! Places: a local, or an item or field inside one, reached without
//! changing any local. Its value can be read without taking a reference,
//! because the local keeps it alive; so can a field of the closure a
//! closure's function borrows (`kavach_kir::borrows`).

use crate::codegen::FnGen;
use cranelift_codegen::ir::Value;
use kavach_kir::{Expr, ExprKind};

impl FnGen<'_, '_> {
    /// The value of `e`, borrowed when it is a place and `may_borrow` (no
    /// new reference): the value and whether the caller owns it.
    pub(crate) fn borrow(&mut self, e: &Expr, may_borrow: bool) -> Option<(Value, bool)> {
        if may_borrow && self.is_place(e) {
            return Some((self.place(e), false));
        }
        Some((self.expr(e)?, true))
    }

    /// Whether `e` is a local, or an item or field inside one, reached
    /// without changing any local: then it can be read without taking a
    /// reference, because the local keeps it alive.
    pub(crate) fn is_place(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Local(_) => true,
            // Moving out of a borrowing local only reads it (`for_each`
            // allows that only for reading a field).
            ExprKind::Move(l) => self.borrowed[*l as usize],
            ExprKind::Index(base, i) => {
                self.is_list(base.ty)
                    && self.flat_width(base.ty) == 0
                    && !self.text_list(base.ty)
                    && crate::data::plain(i)
                    && self.is_place(base)
            }
            ExprKind::GetField(base, _) => {
                (self.has_fixed_fields(base.ty) && (self.is_place(base) || self.is_flat_item(base)))
                    || self.borrowed_closure(base)
            }
            _ => false,
        }
    }

    /// Whether `e` is a closure a closure's function borrows: its fields
    /// are read in place, the closure kept alive by the caller.
    fn borrowed_closure(&self, e: &Expr) -> bool {
        let closure = |t: kavach_kir::Type| {
            matches!(t, kavach_kir::Type::Ref(id)
                if matches!(self.shapes[id as usize], kavach_kir::Shape::Closure(_)))
        };
        matches!(e.kind, ExprKind::Local(l) | ExprKind::Move(l) if self.borrowed[l as usize])
            && closure(e.ty)
    }

    /// Whether `e` is an item of a flat list that is a place: its fields
    /// are read in the list's buffer, through a view (`flat.rs`).
    fn is_flat_item(&self, e: &Expr) -> bool {
        matches!(&e.kind, ExprKind::Index(base, i)
            if self.flat_width(base.ty) > 0 && crate::data::plain(i) && self.is_place(base))
    }

    /// A view of `e`, an item of a flat list (see `is_flat_item`).
    fn flat_item_view(&mut self, e: &Expr) -> Value {
        let ExprKind::Index(base, i) = &e.kind else {
            unreachable!("an item of a flat list")
        };
        let obj = self.place(base);
        let view = self.list_view(base, obj);
        let index = self.slot_of(i);
        if !self.known_in_range(base, i) {
            self.check_index(view, index, e.line);
        }
        let width = self.flat_width(base.ty);
        let addr = self.flat_addr(view.0, index, width);
        self.flat_view(addr)
    }

    /// The value of a place (see `is_place`), borrowed.
    pub(crate) fn place(&mut self, e: &Expr) -> Value {
        match &e.kind {
            ExprKind::Index(base, i) => {
                let obj = self.place(base);
                let view = self.list_view(base, obj);
                let index = self.slot_of(i);
                if self.known_in_range(base, i) {
                    return self.item_at(view.0, index, e.ty);
                }
                self.checked_item(view, index, e.ty, e.line)
            }
            ExprKind::GetField(base, f) => {
                let obj = if self.is_flat_item(base) {
                    self.flat_item_view(base)
                } else {
                    self.place(base)
                };
                self.field_value(obj, *f, e.ty)
            }
            ExprKind::Local(l) | ExprKind::Move(l) => self.b.use_var(self.vars[*l as usize]),
            _ => unreachable!("a place"),
        }
    }

    /// `Move(l)`, the last read of local `l`: its value handed over and the
    /// variable emptied; out of a borrowing local, a new reference instead
    /// (`is_ref`: a heap value), since it holds none of its own.
    pub(crate) fn moved(&mut self, l: kavach_kir::Local, is_ref: bool) -> Value {
        let var = self.vars[l as usize];
        let v = self.b.use_var(var);
        if self.borrowed[l as usize] {
            if is_ref {
                self.retain(v);
            }
            return v;
        }
        // Only a heap reference is emptied: a plain value (a `Bool`, a
        // `Float`) has nothing to hand over, and its variable may not even
        // be 64 bits wide.
        if is_ref {
            let empty = self.int(cranelift_codegen::ir::types::I64, 0);
            self.b.def_var(var, empty);
        }
        v
    }
}
