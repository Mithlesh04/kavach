//! What changes a value in place (spec 2.2): `items.push(x)` and the like
//! need `items` to be a `let`, as assignment does.

use crate::checker::Checker;
use kavach_ast::*;
use kavach_resolver::DefKind;

impl Checker<'_> {
    /// `items.push(x)` changes `items`, so it must be a `const` (spec 2.2).
    pub(crate) fn require_var(&mut self, base: &Expr, method: &str) {
        self.check_round_change(base, method);
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

    /// `count += 1` (or `log.push(x)`) inside a lambda or `spawn` block,
    /// where `count` is a variable from outside: only the copy changes
    /// (spec 3.3).
    pub(crate) fn check_captured_change(&mut self, target: &Expr) {
        let Some(&(body, what)) = self.copies.last() else {
            return;
        };
        let mut root = target;
        while let ExprKind::Field { base, .. }
        | ExprKind::TupleField { base, .. }
        | ExprKind::Index { base, .. } = &root.kind
        {
            root = base;
        }
        let ExprKind::Name(id) = &root.kind else {
            return;
        };
        let Some(&def) = self.res.uses.get(&id.span.start) else {
            return;
        };
        let d = &self.res.defs[def];
        let outside = matches!(d.kind, kavach_resolver::DefKind::Local { .. })
            && d.span.is_some_and(|s| s.start < body);
        if outside {
            let message = format!(
                "`{}` is a copy inside this {what}: the change doesn't reach the outside. Give the value back instead, or use `Shared` to share it (spec 3.3)",
                id.name
            );
            self.warn(target.span, &message);
        }
    }
}
