//! C's pointer syntax on `Ptr<T>` (spec 6.4): `*p`, `p[i]` and `p + n`,
//! the same operations as `p.read(0)`, `p.read(i)` and an address `n`
//! values on. Only inside `unsafe`.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;

impl Checker<'_> {
    /// `*p`: the value pointer `p` points at.
    pub(crate) fn deref(&mut self, x: &Expr) -> Ty {
        let t = self.value(x, None);
        match self.inf.resolve(&t) {
            Ty::Builtin("Ptr", args) => {
                self.need_unsafe("`*p`", x.span);
                args.first().cloned().unwrap_or(Ty::Error)
            }
            Ty::Error => Ty::Error,
            other => {
                let shown = self.show(&other);
                self.error(
                    x.span,
                    format!("`*` reads through a pointer, and this is `{shown}`"),
                );
                Ty::Error
            }
        }
    }

    /// `p[i]` on pointer type `args` (`Ptr<T>`): the `i`th `T`.
    pub(crate) fn ptr_item(&mut self, args: &[Ty], index: &Expr, span: Span) -> Ty {
        self.need_unsafe("`p[i]`", span);
        self.check_expect(index, &Ty::Prim(Prim::Int));
        args.first().cloned().unwrap_or(Ty::Error)
    }

    /// `p + n` and `p - n` when `lt`, the left side's type, is a pointer:
    /// a pointer of the same type. `None` when it isn't one.
    pub(crate) fn ptr_offset(&mut self, op: BinaryOp, lt: &Ty, r: &Expr, span: Span) -> Option<Ty> {
        let Ty::Builtin("Ptr", _) = self.inf.resolve(lt) else {
            return None;
        };
        if !matches!(op, BinaryOp::Add | BinaryOp::Sub) {
            return None;
        }
        self.need_unsafe("`p + n`", span);
        self.check_expect(r, &Ty::Prim(Prim::Int));
        Some(self.inf.resolve(lt))
    }

    /// Inside `unsafe` the resolver leaves `a[i] = v` to here: through a
    /// `Ptr` it writes memory, so `p` itself needn't change; any other
    /// index changes its variable, which must be a `let` (spec 1.4, 6.4).
    pub(crate) fn check_unsafe_index_target(&mut self, target: &Expr) {
        let ExprKind::Index { base, .. } = &target.kind else {
            return;
        };
        if self.in_unsafe == 0 {
            return;
        }
        let base_ty = self
            .expr_types
            .iter()
            .rev()
            .find(|(s, _)| *s == base.span)
            .map(|(_, t)| t.clone());
        if matches!(
            base_ty.map(|t| self.inf.resolve(&t)),
            Some(Ty::Builtin("Ptr", _))
        ) {
            return;
        }
        let mut root = &**base;
        while let ExprKind::Field { base, .. }
        | ExprKind::TupleField { base, .. }
        | ExprKind::Index { base, .. } = &root.kind
        {
            root = base;
        }
        let ExprKind::Name(id) = &root.kind else {
            return;
        };
        let kind = self
            .res
            .uses
            .get(&id.span.start)
            .map(|d| self.res.defs[*d].kind);
        let why = match kind {
            Some(kavach_resolver::DefKind::Local { mutable: false }) => {
                "is a `const`, so it can't change: declare it with `let`"
            }
            Some(kavach_resolver::DefKind::Param) => {
                "is a parameter, so it can't change: copy it into a `let` first"
            }
            _ => return,
        };
        self.error(root.span, format!("`{}` {why}", id.name));
    }
}
