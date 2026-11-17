//! Which assignment targets may change (spec 1.4, 2.4, 3.1, 4.4): only
//! `let` bindings, with advice that fits what was named.

use crate::DefKind;
use crate::resolver::Resolver;
use kavach_ast::*;

impl Resolver {
    /// Only `let` bindings can change (spec 1.4, 2.4, 3.1).
    pub(crate) fn check_assignable(&mut self, target: &Expr) {
        // `p[i] = v` writes through a pointer, not to `p`: inside `unsafe`
        // the checker, which knows the types, decides (spec 6.4).
        if self.in_unsafe > 0 && matches!(target.kind, ExprKind::Index { .. }) {
            return;
        }
        let mut root = target;
        while let ExprKind::Field { base, .. }
        | ExprKind::TupleField { base, .. }
        | ExprKind::Index { base, .. } = &root.kind
        {
            root = base;
        }
        let message = match &root.kind {
            ExprKind::SelfValue => {
                "`self` can't change: return an updated copy with `with` instead".to_string()
            }
            ExprKind::Name(ident) => match self.res.def_at(ident.span.start).map(|d| d.kind) {
                Some(DefKind::Local { mutable: true }) | None => return,
                Some(DefKind::Local { .. })
                    if (self.res.uses.get(&ident.span.start))
                        .is_some_and(|d| self.taken.contains(d)) =>
                {
                    format!(
                        "`{}` is a name a `for` or `match` takes, so it can't change: work out the new value in a `let` of its own (spec 4.4)",
                        ident.name
                    )
                }
                Some(DefKind::Local { mutable: false }) => {
                    format!(
                        "`{}` is a `const`, so it can't change: declare it with `let`",
                        ident.name
                    )
                }
                Some(DefKind::Param) => format!(
                    "`{}` is a parameter, so it can't change: copy it into a `let` first",
                    ident.name
                ),
                Some(_) => format!("`{}` can't be assigned to", ident.name),
            },
            _ => return,
        };
        self.error(root.span, message);
    }
}
