// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A record's invariants checked after a change (spec 7.3). A field set
//! on a variable is stored first and then checked where it lies, so the
//! record is changed in place, never copied for the check; and only the
//! clauses that read the changed field are checked again, as the others
//! read what was true before (Sakshi's S18: 20–40 ns a change, from a
//! copy of the record each time).

use crate::ir::{self, Local, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast::visit::{VisitMut, walk_expr};
use kavach_ast::{self as ast, ExprKind};
use kavach_resolver::DefId;
use std::collections::HashMap;

/// Whether invariant clause `inv` may read field `field`: it names it,
/// or it uses `self`, which may read any field.
pub(crate) fn may_read(inv: &ast::Expr, field: &str) -> bool {
    struct Reads<'f>(&'f str, bool);
    impl VisitMut for Reads<'_> {
        fn expr(&mut self, e: &mut ast::Expr) {
            match &e.kind {
                ExprKind::Name(id) if id.name == self.0 => self.1 = true,
                ExprKind::SelfValue => self.1 = true,
                _ => walk_expr(self, e),
            }
        }
    }
    let mut reads = Reads(field, false);
    reads.expr(&mut inv.clone());
    reads.1
}

/// What `enter_invariant` replaced, for `leave_invariant` to put back.
pub(crate) struct OuterInvariant {
    of: Option<(Local, Type)>,
    subst: HashMap<DefId, Type>,
}

impl<'a> Lowerer<'a> {
    /// Starts lowering an invariant of record type `ty` on the record held
    /// in `local`: its fields read that record, and the type's generic
    /// parameters stand for its type arguments, so an invariant of
    /// `Range<T>` is lowered for each type it is made with (`Range<Int>`).
    pub(crate) fn enter_invariant(&mut self, local: Local, ty: Type) -> OuterInvariant {
        let mut subst = self.subst.clone();
        if let Some(def) = self.shapes.def_of(ty) {
            let args = self.shapes.args_of(ty).unwrap_or_default().to_vec();
            subst.extend(self.type_generics(def).into_iter().zip(args));
        }
        OuterInvariant {
            of: self.invariant_of.replace((local, ty)),
            subst: std::mem::replace(&mut self.subst, subst),
        }
    }

    /// Ends what `enter_invariant` started.
    pub(crate) fn leave_invariant(&mut self, outer: OuterInvariant) {
        self.invariant_of = outer.of;
        self.subst = outer.subst;
    }

    /// The checks of the invariants of `ty`, reading the record held in
    /// `local`. With `changed`, only the clauses that may read that field.
    pub(crate) fn checks_on_change(
        &mut self,
        local: Local,
        ty: Type,
        changed: Option<&str>,
        line: u32,
    ) -> Lowered<Vec<Stmt>> {
        if self.emptying {
            return Ok(Vec::new());
        }
        let invariants = self.invariants_of(ty);
        let mut checks = Vec::new();
        let outer = self.enter_invariant(local, ty);
        for inv in invariants {
            if changed.is_some_and(|f| !may_read(inv, f)) {
                continue;
            }
            let cond = match self.expr(inv) {
                Ok(c) => c,
                Err(e) => {
                    self.leave_invariant(outer);
                    return Err(e);
                }
            };
            checks.push(Stmt::Check {
                cond,
                message: format!("contract failed: invariant {}", self.text(inv.span)),
                line,
            });
        }
        self.leave_invariant(outer);
        Ok(checks)
    }

    /// `value` (a new or changed record), after checking its type's
    /// invariants (spec 7.3): a false one panics with the contract. With
    /// `changed`, only the clauses that may read that field.
    pub(crate) fn with_invariants_on(
        &mut self,
        value: ir::Expr,
        changed: Option<&str>,
    ) -> Lowered<ir::Expr> {
        if self.emptying || self.invariants_of(value.ty).is_empty() {
            return Ok(value);
        }
        let (ty, line) = (value.ty, value.line);
        let tmp = self.new_local(ty);
        let checks = self.checks_on_change(tmp, ty, changed, line)?;
        if checks.is_empty() {
            return Ok(value);
        }
        let mut stmts = vec![Stmt::Let(tmp, value)];
        stmts.extend(checks);
        Ok(ir::Expr {
            kind: ir::ExprKind::Block(ir::Block {
                stmts,
                value: Some(Box::new(self.local_expr(tmp, line))),
            }),
            ty,
            line,
        })
    }

    /// The variable `base` names, when a field set on it can be stored in
    /// place and checked there.
    pub(crate) fn plain_local(&self, base: &ast::Expr) -> Option<Local> {
        match &base.kind {
            ExprKind::Name(id) if !self.emptying => self.local(id).ok(),
            _ => None,
        }
    }

    /// `x.f = v` on the variable `local` (`changed` is the record with `f`
    /// set): stored in place, then the clauses that read `f` checked on
    /// the variable. With no such clause, the plain store is all.
    pub(crate) fn store_then_check(
        &mut self,
        base: &'a ast::Expr,
        local: Local,
        changed: ir::Expr,
        field: &str,
    ) -> Lowered<Stmt> {
        let (ty, line) = (changed.ty, changed.line);
        let store = self.assign_path(base, changed)?;
        let mut stmts = self.checks_on_change(local, ty, Some(field), line)?;
        if stmts.is_empty() {
            return Ok(store);
        }
        stmts.insert(0, store);
        Ok(Stmt::Expr(ir::Expr {
            kind: ir::ExprKind::Block(ir::Block { stmts, value: None }),
            ty: Type::Unit,
            line,
        }))
    }
}
