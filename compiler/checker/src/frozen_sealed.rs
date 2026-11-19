// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Where a sealed value (spec 2.20), `(value, key)` while running, is
//! read as its value: code generation takes the value out there
//! (`compiler/driver/src/frozen.rs`).

use crate::checker::Checker;
use crate::frozen_reads::Types;
use crate::ty::Ty;
use kavach_ast::visit::{VisitMut, walk_expr, walk_stmt};
use kavach_ast::*;
use kavach_resolver::DefKind;
use std::collections::HashSet;

/// The places a sealed value is read as its `T` (spec 2.20): code
/// generation takes the value out of its `(value, key)` there.
pub(crate) fn sealed_reads(c: &Checker, file: &File, types: &Types) -> HashSet<(u32, u32)> {
    let mut v = Sealed {
        c,
        types,
        found: HashSet::new(),
        callees: HashSet::new(),
    };
    kavach_ast::visit::walk_file(&mut v, &mut file.clone());
    v.found
}

struct Sealed<'c, 'a> {
    c: &'c Checker<'a>,
    types: &'c Types,
    found: HashSet<(u32, u32)>,
    /// The `x.m` of each method call: its `x` is read, or not, by the call.
    callees: HashSet<(u32, u32)>,
}

impl Sealed<'_, '_> {
    fn read(&mut self, e: &Expr) {
        let key = (e.span.start, e.span.end);
        if let Some(t) = self.types.get(&key)
            && let Ty::Frozen(_, _, Some(_)) = self.c.inf.shallow_keep(t)
        {
            self.found.insert(key);
        }
    }
}

impl VisitMut for Sealed<'_, '_> {
    fn expr(&mut self, e: &mut Expr) {
        let span = (e.span.start, e.span.end);
        match &e.kind {
            ExprKind::Field { .. } if self.callees.contains(&span) => {}
            ExprKind::Field { base, .. }
            | ExprKind::TupleField { base, .. }
            | ExprKind::With { base, .. } => self.read(base),
            ExprKind::Index { base, index } => {
                self.read(base);
                self.read(index);
            }
            ExprKind::Unary(_, x) => self.read(x),
            ExprKind::Binary(op, l, r) if *op != BinaryOp::Default => {
                self.read(l);
                self.read(r);
            }
            ExprKind::Compare(first, links) => {
                self.read(first);
                links.iter().for_each(|(_, x)| self.read(x));
            }
            ExprKind::Str(parts) => {
                for p in parts {
                    if let StrPart::Expr(x) = p {
                        self.read(x);
                    }
                }
            }
            ExprKind::Match { scrutinee, arms }
                if !arms
                    .iter()
                    .any(|a| matches!(a.pattern.kind, PatternKind::Binding(_))) =>
            {
                self.read(scrutinee)
            }
            ExprKind::Call { callee, args, .. } => match &callee.kind {
                ExprKind::Field { base, .. } => {
                    self.callees.insert((callee.span.start, callee.span.end));
                    if self.c.frozen.calls.get(&span)
                        != Some(&crate::frozen::FrozenCall::Unfreeze(true))
                    {
                        self.read(base);
                    }
                }
                ExprKind::Name(id)
                    if (self.c.res.uses.get(&id.span.start))
                        .is_some_and(|d| self.c.res.defs[*d].kind == DefKind::Builtin) =>
                {
                    args.iter().for_each(|a| self.read(&a.value));
                }
                _ => {}
            },
            _ => {}
        }
        walk_expr(self, e);
    }

    fn stmt(&mut self, s: &mut Stmt) {
        match &s.kind {
            StmtKind::For { iter, .. } => self.read(iter),
            StmtKind::Let { pattern, value, .. }
                if !matches!(pattern.kind, PatternKind::Binding(_)) =>
            {
                self.read(value)
            }
            _ => {}
        }
        walk_stmt(self, s);
    }
}
