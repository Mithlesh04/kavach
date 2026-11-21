// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The order things are worked out in (spec 3.10): left to right, as
//! written. Arguments named out of their parameters' order, and record
//! fields written out of their declared order, are worked out as written
//! into locals first; a place's keys are worked out once.

use crate::ir::{self, Block, ExprKind as K, Stmt};
use crate::lower::{Lowered, Lowerer};
use crate::walk::any_expr;
use kavach_ast::{self as ast, ExprKind};

/// Whether working out `e` could be seen: it calls something, runs a
/// runtime operation, fails or panics. Reading locals and arithmetic
/// can't be.
pub(crate) fn may_act(e: &ir::Expr) -> bool {
    any_expr(e, &mut |x| {
        matches!(
            x.kind,
            K::Call(..)
                | K::CallValue(..)
                | K::Rt(..)
                | K::Panic(_)
                | K::Fail(_)
                | K::Assign(..)
                | K::Block(_)
                | K::If(..)
                | K::Fallback(..)
        )
    })
}

impl<'a> Lowerer<'a> {
    /// `values`, in the order they are used: `written[i]` is where value
    /// `i` was written among the call's arguments, `None` for a default,
    /// worked out after every written one, and `acts[i]` whether working
    /// it out could be seen (`may_act`). When the order used could be
    /// seen to differ from the order written, the written values are
    /// worked out first, as written, into locals that the values read.
    pub(crate) fn in_written_order(
        &mut self,
        values: &mut [ir::Expr],
        written: &[Option<usize>],
        acts: &[bool],
    ) {
        let n = values.len().min(written.len()).min(acts.len());
        let swapped = |i: usize, j: usize| match (written[i], written[j]) {
            (Some(a), Some(b)) => a > b,
            (None, Some(_)) => true,
            _ => false,
        };
        let reorder = (0..n).any(|j| (0..j).any(|i| acts[i] && acts[j] && swapped(i, j)));
        if !reorder {
            return;
        }
        let mut order: Vec<usize> = (0..n).filter(|&i| written[i].is_some()).collect();
        order.sort_by_key(|&i| written[i]);
        let mut stmts = Vec::new();
        for i in order {
            let line = values[i].line;
            let local = self.new_local(values[i].ty);
            let read = self.local_expr(local, line);
            stmts.push(Stmt::Let(local, std::mem::replace(&mut values[i], read)));
        }
        // The locals are set before the first value is read.
        let first = &mut values[0];
        let (ty, line) = (first.ty, first.line);
        let value = std::mem::replace(
            first,
            ir::Expr {
                kind: K::None,
                ty,
                line,
            },
        );
        *first = ir::Expr {
            kind: K::Block(Block {
                stmts,
                value: Some(Box::new(value)),
            }),
            ty,
            line,
        };
    }

    /// Works out the keys of place `target` that could be seen working
    /// out (a call), once, as written, into locals, before the statement
    /// that changes the place: `xs[pick()] += 1` calls `pick` once. Gives
    /// the keys held; `release` lets go of them after the statement.
    pub(crate) fn hold_keys(
        &mut self,
        target: &'a ast::Expr,
        out: &mut Vec<Stmt>,
    ) -> Lowered<Vec<(u32, u32)>> {
        let mut keys = Vec::new();
        let mut root = target;
        loop {
            match &root.kind {
                ExprKind::Field { base, .. } | ExprKind::TupleField { base, .. } => root = base,
                ExprKind::Index { base, index } => {
                    keys.push(&**index);
                    root = base;
                }
                _ => break,
            }
        }
        let mut held = Vec::new();
        for key in keys.into_iter().rev() {
            if matches!(key.kind, ExprKind::FromEnd(_) | ExprKind::Part { .. }) || key.is_plain() {
                continue;
            }
            let value = self.expr(key)?;
            let local = self.new_local(value.ty);
            out.push(Stmt::Let(local, value));
            let span = (key.span.start, key.span.end);
            self.held_parts.insert(span, (local, false));
            held.push(span);
        }
        Ok(held)
    }

    /// `target = value` and `target op= value`, with `target`'s keys held.
    pub(crate) fn assign_stmt(
        &mut self,
        target: &'a ast::Expr,
        op: ast::AssignOp,
        value: &'a ast::Expr,
        line: u32,
        span: ast::Span,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        let mut value = self.expr(value)?;
        // `log += ["step"]`: pushes onto the variable (spec 2.2).
        if op == ast::AssignOp::Add
            && let ExprKind::Name(id) = &target.kind
            && let Ok(local) = self.local(id)
            && self.is_list_type(self.locals[local as usize])
        {
            out.extend(self.list_extend(local, value, line)?);
            return Ok(());
        }
        if let Some(bin) = op.binary() {
            value = self.compound(bin, target, value, line, span)?;
        }
        out.push(self.assign_path(target, value)?);
        Ok(())
    }

    /// Lets go of the keys `hold_keys` held.
    pub(crate) fn release(&mut self, held: Vec<(u32, u32)>) {
        for span in held {
            self.held_parts.remove(&span);
        }
    }
}
