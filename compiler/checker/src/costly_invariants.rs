// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A change to a record whose `invariant` walks a collection, made in a
//! loop (spec 7.3): the invariant is checked after every change, so the
//! loop walks the collection each round, which grows with the square of
//! its size. A warning says so, and how to build it instead.

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::visit::{VisitMut, walk_expr};
use kavach_ast::*;

/// Methods that walk a whole collection.
const WALKS: &[&str] = &[
    "all", "any", "sum", "count", "contains", "filter", "map", "min", "max", "sorted", "index_of",
    "find", "fold", "unique", "join", "reversed",
];

/// Whether invariant `e` calls a method that walks a collection.
pub(crate) fn walks(e: &Expr) -> bool {
    struct Walks(bool);
    impl VisitMut for Walks {
        fn expr(&mut self, e: &mut Expr) {
            if let ExprKind::Call { callee, .. } = &e.kind
                && let ExprKind::Field { name, .. } = &callee.kind
                && WALKS.contains(&name.name.as_str())
            {
                self.0 = true;
            }
            walk_expr(self, e);
        }
    }
    let mut found = Walks(false);
    found.expr(&mut e.clone());
    found.0
}

impl Checker<'_> {
    /// `target` changes: inside a loop, a warning when a record on its
    /// path has an invariant that walks a collection.
    pub(crate) fn check_invariant_in_loop(&mut self, target: &Expr) {
        if self.loops.is_empty() {
            return;
        }
        let mut at = target;
        loop {
            let base = match &at.kind {
                ExprKind::Field { base, .. }
                | ExprKind::Index { base, .. }
                | ExprKind::TupleField { base, .. } => base,
                _ => return,
            };
            if let Some(Ty::Adt(def, _)) = self.type_at(base.span).map(|t| self.inf.resolve(&t))
                && self.costly_invariants.contains(&def)
            {
                let name = self.def_name(def).to_string();
                self.warn(target.span, &format!("a `{name}` has an invariant that walks a collection, checked after every change (spec 7.3): changing one in a loop walks the collection each round; build the collection first, then make the `{name}` once"));
                return;
            }
            at = base;
        }
    }
}
