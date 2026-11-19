// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Module constants in native code (spec 1.13): each becomes a function
//! that works its value out, made when first read. A read calls it, and
//! the program's start calls those something reads, in the order they are
//! written, so a panic in one comes before `main`; one nothing reads is
//! left out. A constant needs no capability and
//! can't fail, so each call gives the same value; LLVM folds the small
//! ones into the code that reads them. (Keeping a big one made once per
//! thread is noted for later in the to-do.)

use crate::ir::{self, Block, ExprKind as K, FuncId, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast as ast;
use kavach_resolver::DefId;
use std::collections::HashMap;

/// The module constants' values, and the functions made for them.
#[derive(Default)]
pub(crate) struct Consts<'a> {
    values: HashMap<DefId, &'a ast::Expr>,
    made: HashMap<DefId, FuncId>,
}

impl<'a> Lowerer<'a> {
    /// Notes top-level statement `s` when it is a module constant: then it
    /// isn't a statement of `main`.
    pub(crate) fn note_const(&mut self, s: &'a ast::Stmt) -> bool {
        let (Some(id), ast::StmtKind::Let { value, .. }) =
            (kavach_resolver::module_const(s), &s.kind)
        else {
            return false;
        };
        let Some(&def) = self.decl.get(&id.span.start) else {
            return false;
        };
        if !self.c.resolution.module_consts.contains(&def) {
            return false;
        }
        self.consts.values.insert(def, value);
        true
    }

    /// The name `id` of type `ty`: a local, or a module constant's value.
    pub(crate) fn local_or_const(
        &mut self,
        id: &'a ast::Ident,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let def = self.c.resolution.uses.get(&id.span.start).copied();
        if let Some(d) = def.filter(|d| self.consts.values.contains_key(d))
            && !self.local_ids.contains_key(&d)
        {
            let f = self.const_fn(d, id.span)?;
            return Ok(ir::Expr {
                kind: K::Call(f, Vec::new()),
                ty,
                line,
            });
        }
        Ok(ir::Expr {
            kind: K::Local(self.local(id)?),
            ty,
            line,
        })
    }

    /// The function that works out constant `def`, made now if not yet.
    fn const_fn(&mut self, def: DefId, span: ast::Span) -> Lowered<FuncId> {
        if let Some(&f) = self.consts.made.get(&def) {
            return Ok(f);
        }
        let value = self.consts.values[&def];
        let ty = self.ty_of(value)?;
        let line = self.line(span);
        let saved = self.save_function(ty);
        let lowered = self.expr(value);
        let locals = self.locals.clone();
        self.restore_function(saved);
        let name = self.c.resolution.defs[def].name.clone();
        let f = self.add_function(ir::Function {
            name: format!("const {name}"),
            params: Vec::new(),
            ret: ty,
            locals,
            body: Block {
                stmts: Vec::new(),
                value: Some(Box::new(lowered?)),
            },
            line,
            fails: false,
            external: None,
            variadic: false,
        });
        self.consts.made.insert(def, f);
        Ok(f)
    }

    /// Calls of every module constant's function that something reads, in
    /// the order they are written, for the program's start.
    pub(crate) fn const_starts(&mut self) -> Lowered<Vec<Stmt>> {
        // Only the constants something reads: one nothing reads is left
        // out, with no work at start (spec 1.13).
        let read: std::collections::HashSet<DefId> =
            self.c.resolution.uses.values().copied().collect();
        let mut defs: Vec<DefId> = (self.consts.values.keys().copied())
            .filter(|d| read.contains(d))
            .collect();
        defs.sort_by_key(|d| self.c.resolution.defs[*d].span.map(|s| s.start));
        let mut out = Vec::new();
        for d in defs {
            let span = self.c.resolution.defs[d]
                .span
                .unwrap_or(ast::Span { start: 0, end: 0 });
            let f = self.const_fn(d, span)?;
            let ty = self.ty_of(self.consts.values[&d])?;
            let line = self.line(span);
            out.push(Stmt::Expr(ir::Expr {
                kind: K::Call(f, Vec::new()),
                ty,
                line,
            }));
        }
        Ok(out)
    }
}
