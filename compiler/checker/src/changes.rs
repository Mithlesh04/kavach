// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What changes a value in place (spec 2.2): `items.push(x)` and the like
//! need `items` to be a `let`, as assignment does.

use crate::checker::Checker;
use kavach_ast::*;
use kavach_resolver::DefKind;

impl Checker<'_> {
    /// `items.push(x)` changes `items`, so it must be a `const` (spec 2.2).
    pub(crate) fn require_var(&mut self, base: &Expr, method: &str) {
        self.check_round_change(base, method);
        self.check_invariant_in_loop(base);
        self.check_read_only_target(base, &format!("changed with `{method}`"));
        self.check_frozen_target(base, &format!("changed with `{method}`"), false, false);
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
        // `x?.push(6)`: the method runs on `?`, a copy of what is inside.
        if id.name == "?" {
            self.error(
                root.span,
                format!("`?.{method}(…)` would change a copy of the value inside, not the value itself (spec 2.7): to change it only when it is there, test first, as `if d.contains_key(k) {{ d[k].{method}(…) }}` for a dict entry"),
            );
            return;
        }
        let name = &id.name;
        let message = match kind {
            Some(DefKind::Local { mutable: false }) => {
                format!("`{method}` changes `{name}`, but it is a `const`: declare it with `let`")
            }
            // A parameter is the caller's value, given as a copy (6.1).
            Some(DefKind::Param) => format!(
                "`{method}` changes `{name}`, but it is a parameter: copy it into a `let` first (`let items = {name}`), or make it `inout` to change the caller's value (spec 3.9)"
            ),
            _ => return,
        };
        self.error(root.span, message);
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
