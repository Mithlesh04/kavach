// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What lowering every body uses: source lines and text, locals and the
//! names bound to them, a function's start, coercion to a wanted type
//! (optional wrapping, interface boxes), blocks, and `ensure` checks.

use crate::Unsupported;
use crate::ir::{self, FuncId, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast::{self as ast, ExprKind, StmtKind};
use kavach_resolver::DefKind;

impl<'a> Lowerer<'a> {
    pub(crate) fn line(&self, span: ast::Span) -> u32 {
        self.c.line_of(span.start) as u32
    }

    pub(crate) fn unsupported(&self, feature: impl Into<String>, span: ast::Span) -> Unsupported {
        Unsupported {
            feature: feature.into(),
            line: self.line(span),
        }
    }

    pub(crate) fn text(&self, span: ast::Span) -> &'a str {
        span.text(&self.c.src)
    }

    pub(crate) fn new_local(&mut self, ty: Type) -> Local {
        self.locals.push(ty);
        (self.locals.len() - 1) as Local
    }

    pub(crate) fn bind(&mut self, id: &ast::Ident, local: Local) {
        if let Some(&def) = self.decl.get(&id.span.start) {
            self.local_ids.insert(def, local);
        }
    }

    pub(crate) fn start_function(&mut self, ret: Type) {
        self.locals.clear();
        self.local_ids.clear();
        self.ret = ret;
        self.result = None;
        self.ensures = &[];
        self.self_local = None;
    }

    /// The declaration of program function `fid`.
    pub(crate) fn decl_of(&self, fid: FuncId) -> &'a ast::FnDecl {
        self.fns[fid as usize].0.expect("a declared function")
    }

    pub(crate) fn local_expr(&self, local: Local, line: u32) -> ir::Expr {
        ir::Expr {
            kind: ir::ExprKind::Local(local),
            ty: self.locals[local as usize],
            line,
        }
    }

    /// `expr` as type `want`: a `T` where a `T?` is expected becomes a
    /// present optional (spec 2.7), and a value where an interface is
    /// expected becomes an interface box (spec 2.15).
    pub(crate) fn coerce(&mut self, expr: ir::Expr, want: Type) -> Lowered<ir::Expr> {
        match self.shapes.shape(want) {
            Some(Shape::Optional(inner)) if *inner == expr.ty => {
                let line = expr.line;
                Ok(ir::Expr {
                    kind: ir::ExprKind::Some(Box::new(expr)),
                    ty: want,
                    line,
                })
            }
            Some(Shape::Iface { .. }) if expr.ty != want => self.iface_value(expr, want),
            // A value where an optional interface is: boxed, then present.
            Some(Shape::Optional(inner))
                if expr.ty != want
                    && matches!(self.shapes.shape(*inner), Some(Shape::Iface { .. }))
                    && expr.ty != *inner =>
            {
                let inner = *inner;
                let boxed = self.iface_value(expr, inner)?;
                self.coerce(boxed, want)
            }
            // A function that can't fail where one that can is wanted.
            Some(Shape::Func(_, _, Some(_))) if self.fn_error(expr.ty).is_none() => {
                self.never_failing(expr, want)
            }
            _ => Ok(expr),
        }
    }

    /// Lowers `b` into `out`. With `want_value`, the last expression becomes
    /// the block's value.
    pub(crate) fn block_into(
        &mut self,
        b: &'a ast::Block,
        want_value: bool,
        out: &mut ir::Block,
    ) -> Lowered<()> {
        for (i, s) in b.stmts.iter().enumerate() {
            match (&s.kind, want_value && i + 1 == b.stmts.len()) {
                // `return`, `break` and `continue` leave instead of giving a
                // value (a `catch` handler that returns early).
                (StmtKind::Expr(e), true) if !leaves(e) => {
                    out.value = Some(Box::new(self.expr(e)?))
                }
                _ => self.stmt(s, &mut out.stmts)?,
            }
        }
        Ok(())
    }

    /// The function's `ensure` checks, on `result` (spec 7.2).
    pub(crate) fn ensure_checks(&mut self, out: &mut Vec<Stmt>) -> Lowered<()> {
        for e in self.ensures {
            let cond = self.expr(e)?;
            out.push(Stmt::Check {
                cond,
                message: format!("contract failed: ensure {}", self.text(e.span)),
                line: self.line(e.span),
            });
        }
        Ok(())
    }

    pub(crate) fn is_builtin(&self, callee: &ast::Expr, name: &str) -> bool {
        let ExprKind::Name(id) = &callee.kind else {
            return false;
        };
        let res = &self.c.resolution;
        id.name == name
            && res
                .uses
                .get(&id.span.start)
                .is_some_and(|d| res.defs[*d].kind == DefKind::Builtin)
    }

    pub(crate) fn local(&self, id: &ast::Ident) -> Lowered<Local> {
        let def = self.c.resolution.uses.get(&id.span.start);
        match def.map(|d| (*d, self.c.resolution.defs[*d].kind)) {
            Some((_, DefKind::Result)) => self
                .result
                .ok_or_else(|| self.unsupported("`result` here", id.span)),
            Some((d, _)) => self
                .local_ids
                .get(&d)
                .copied()
                .ok_or_else(|| self.unsupported(format!("the value `{}`", id.name), id.span)),
            None => Err(self.unsupported(format!("the name `{}`", id.name), id.span)),
        }
    }
}

/// Whether `e` leaves the block instead of giving it a value.
pub(crate) fn leaves(e: &ast::Expr) -> bool {
    matches!(
        e.kind,
        ExprKind::Return(_) | ExprKind::Break(_) | ExprKind::Continue(_)
    )
}
