// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What a lambda's body reads and changes, found before it is lowered.

use kavach_ast::{self as ast, ExprKind};

/// The methods that change the value they are called on (as the checker
/// has them, `compiler/checker/src/methods.rs`).
const CHANGING: [&str; 3] = ["push", "remove", "add"];

/// The variable at the root of `x`, `x.f`, `x.0` or `x[i]`.
fn root_name(mut e: &ast::Expr) -> Option<&ast::Ident> {
    while let ExprKind::Field { base, .. }
    | ExprKind::TupleField { base, .. }
    | ExprKind::Index { base, .. } = &e.kind
    {
        e = base;
    }
    match &e.kind {
        ExprKind::Name(id) => Some(id),
        _ => None,
    }
}

/// The variables `e` changes: `x = …`, `x += …`, `x.f = …` or `x[i] = …`,
/// `x.push(…)` and the other changing methods, and `f(x)` where `f`'s
/// parameter is `inout`: `inout(def)` gives a function's `inout`
/// parameters, by position and name.
pub(super) fn assigned(
    e: &ast::Expr,
    res: &kavach_resolver::Resolution,
    inout: &dyn Fn(kavach_resolver::DefId) -> Vec<(usize, String)>,
) -> Vec<kavach_resolver::DefId> {
    struct Find<'r> {
        res: &'r kavach_resolver::Resolution,
        inout: &'r dyn Fn(kavach_resolver::DefId) -> Vec<(usize, String)>,
        found: Vec<kavach_resolver::DefId>,
    }
    impl Find<'_> {
        fn note(&mut self, target: &ast::Expr) {
            let Some(id) = root_name(target) else { return };
            let def = self.res.uses.get(&id.span.start).copied();
            if let Some(def) = def.filter(|d| !self.found.contains(d)) {
                self.found.push(def);
            }
        }
    }
    impl kavach_ast::visit::VisitMut for Find<'_> {
        fn stmt(&mut self, s: &mut ast::Stmt) {
            if let ast::StmtKind::Assign { target, .. } = &s.kind {
                self.note(target);
            }
            kavach_ast::visit::walk_stmt(self, s);
        }
        fn expr(&mut self, e: &mut ast::Expr) {
            if let ExprKind::Call { callee, .. } = &e.kind
                && let ExprKind::Field { base, name, .. } = &callee.kind
                && CHANGING.contains(&name.name.as_str())
            {
                self.note(base);
            }
            // `bump(count)` with `fn bump(inout n: Int)` (spec 3.9).
            if let ExprKind::Call { callee, args, .. } = &e.kind
                && let ExprKind::Name(f) = &callee.kind
                && let Some(&def) = self.res.uses.get(&f.span.start)
            {
                for (pos, name) in (self.inout)(def) {
                    let given = match args
                        .iter()
                        .find(|a| a.name.as_ref().is_some_and(|n| n.name == name))
                    {
                        Some(a) => Some(a),
                        None => args.get(pos).filter(|a| a.name.is_none()),
                    };
                    if let Some(a) = given {
                        self.note(&a.value);
                    }
                }
            }
            kavach_ast::visit::walk_expr(self, e);
        }
    }
    let mut find = Find {
        res,
        inout,
        found: Vec::new(),
    };
    kavach_ast::visit::VisitMut::expr(&mut find, &mut e.clone());
    find.found
}

/// Whether `e` reads the variable `def`.
pub(super) fn mentions(
    e: &ast::Expr,
    def: Option<kavach_resolver::DefId>,
    res: &kavach_resolver::Resolution,
) -> bool {
    struct Find<'r> {
        def: Option<kavach_resolver::DefId>,
        res: &'r kavach_resolver::Resolution,
        found: bool,
    }
    impl kavach_ast::visit::VisitMut for Find<'_> {
        fn expr(&mut self, e: &mut ast::Expr) {
            if let ExprKind::Name(id) = &e.kind {
                self.found |= self.res.uses.get(&id.span.start).copied() == self.def;
            }
            kavach_ast::visit::walk_expr(self, e);
        }
    }
    let mut find = Find {
        def,
        res,
        found: false,
    };
    kavach_ast::visit::VisitMut::expr(&mut find, &mut e.clone());
    find.found
}
