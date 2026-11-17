//! What changes a value in place (spec 2.2): `items.push(x)` and the like
//! need `items` to be a `let`, as assignment does.

use crate::checker::Checker;
use kavach_ast::*;
use kavach_resolver::DefKind;

impl Checker<'_> {
    /// `items.push(x)` changes `items`, so it must be a `const` (spec 2.2).
    pub(crate) fn require_var(&mut self, base: &Expr, method: &str) {
        let mut root = base;
        while let ExprKind::Field { base, .. }
        | ExprKind::TupleField { base, .. }
        | ExprKind::Index { base, .. } = &root.kind
        {
            root = base;
        }
        let ExprKind::Name(id) = &root.kind else {
            return;
        };
        if self.in_contract {
            self.error(
                root.span,
                format!("a contract can't change `{}` with `{method}`: contracts must be pure, and a check that changes what it checks hides the change (spec 7.5)", id.name),
            );
            return;
        }
        let kind = self
            .res
            .uses
            .get(&id.span.start)
            .map(|d| self.res.defs[*d].kind);
        let what = match kind {
            Some(DefKind::Local { mutable: false }) => "a `const`",
            Some(DefKind::Param) => "a parameter",
            _ => return,
        };
        self.error(
            root.span,
            format!(
                "`{method}` changes `{}`, but it is {what}: declare it with `let`",
                id.name
            ),
        );
    }
}
