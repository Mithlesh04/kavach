//! What may leave an `arena` (spec 6.3): only its block's value. A value
//! that holds memory and is made from a variable declared in the arena
//! can't be kept outside: assigned or stored into something declared
//! before the arena, given to an `inout` parameter, captured by a lambda
//! or task kept outside, or passed out with `return` or `fail`.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;

impl Checker<'_> {
    /// `target = value`, or `value` stored into `target` (`push`, `update`).
    pub(crate) fn check_arena_store(&mut self, target: &Expr, value: &Expr) {
        for arena in self.arenas.clone() {
            if self.declared_in(root_of(target), arena) == Some(false) && self.made_in(value, arena)
            {
                self.error(value.span, "this value was made inside the `arena` and can't be stored outside it: give it back as the arena's value instead (spec 6.3)");
                return;
            }
        }
    }

    /// `return value` or `fail value` inside an arena.
    pub(crate) fn check_arena_leave(&mut self, value: &Expr, how: &str) {
        if self.ctx.as_ref().is_some_and(|c| c.name == "this lambda") {
            return;
        }
        if let Some(_arena) = (self.arenas.clone().into_iter()).find(|a| self.made_in(value, *a)) {
            self.error(value.span, format!("this value was made inside the `arena` and can't leave it with `{how}`: give it back as the arena's value instead (spec 6.3)"));
        }
    }

    /// A call that takes `target` as `inout` and `value` as another
    /// argument could keep `value` in `target`.
    pub(crate) fn check_arena_inout(&mut self, target: &Expr, value: &Expr) {
        for arena in self.arenas.clone() {
            if self.declared_in(root_of(target), arena) == Some(false) && self.made_in(value, arena)
            {
                self.error(value.span, "this value was made inside the `arena`, and the `inout` argument could keep it outside: give it back as the arena's value instead (spec 6.3)");
                return;
            }
        }
    }

    /// Whether the variable `e` names was declared inside `arena`; `None`
    /// when `e` isn't a variable.
    fn declared_in(&self, e: &Expr, arena: Span) -> Option<bool> {
        let ExprKind::Name(id) = &e.kind else {
            return None;
        };
        let def = self.res.uses.get(&id.span.start)?;
        let span = self.res.defs[*def].span?;
        Some(span.start >= arena.start && span.start < arena.end)
    }

    /// Whether `e` holds memory and is made from a variable declared in
    /// `arena`: it names one anywhere inside, lambdas and tasks included.
    fn made_in(&self, e: &Expr, arena: Span) -> bool {
        if self.copied(e) {
            return false;
        }
        struct Find<'c, 'r> {
            c: &'c Checker<'r>,
            arena: Span,
            found: bool,
            /// Inside a lambda or task, which keeps what it names.
            capturing: bool,
        }
        impl kavach_ast::visit::VisitMut for Find<'_, '_> {
            fn expr(&mut self, e: &mut Expr) {
                // A copied part (`words.len()`) holds nothing of the arena,
                // unless a lambda or task computes it later from what it
                // keeps (`() => words.len()` holds `words`).
                if self.c.copied(e) && !self.capturing {
                    return;
                }
                if let ExprKind::Name(_) = &e.kind {
                    self.found |=
                        self.c.declared_in(e, self.arena) == Some(true) && !self.c.copied(e);
                }
                let was = self.capturing;
                self.capturing |= matches!(e.kind, ExprKind::Lambda { .. } | ExprKind::Spawn(..));
                kavach_ast::visit::walk_expr(self, e);
                self.capturing = was;
            }
        }
        let mut find = Find {
            c: self,
            arena,
            found: false,
            capturing: false,
        };
        kavach_ast::visit::VisitMut::expr(&mut find, &mut e.clone());
        find.found
    }
}

impl Checker<'_> {
    /// Whether `e`'s value is copied, holding no memory: a number (not a
    /// `Decimal` or `BigInt`), a `Bool` or a `Char`.
    fn copied(&self, e: &Expr) -> bool {
        let recorded = (self.expr_types.iter().rev()).find(|(sp, _)| *sp == e.span);
        match recorded.map(|(_, t)| self.inf.resolve(t)) {
            Some(Ty::Prim(p)) => {
                p.is_numeric() && p != Prim::Decimal && p != Prim::BigInt
                    || matches!(p, Prim::Bool | Prim::Char)
            }
            Some(Ty::Unit | Ty::Never | Ty::Error) => true,
            _ => false,
        }
    }
}

/// The variable at the root of `x`, `x.f`, `x[i]` or `x.0`.
fn root_of(mut e: &Expr) -> &Expr {
    while let ExprKind::Field { base, .. }
    | ExprKind::Index { base, .. }
    | ExprKind::TupleField { base, .. } = &e.kind
    {
        e = base;
    }
    e
}
