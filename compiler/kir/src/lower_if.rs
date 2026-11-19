// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Lowering `if`, `if let`, `match` and blocks: as values, or with
//! statements for branches. Split from `lower_expr.rs`.

use crate::ir::{self, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::lower_expr::wrap;
use kavach_ast::{self as ast, ExprKind};

impl<'a> Lowerer<'a> {
    /// `if`, `if let`, `match` and blocks. Without `want_value` their
    /// branches are statements.
    pub(crate) fn branching(&mut self, e: &'a ast::Expr, want_value: bool) -> Lowered<ir::Expr> {
        let line = self.line(e.span);
        let ty = if want_value {
            self.ty_of(e)?
        } else {
            Type::Unit
        };
        let kind = match &e.kind {
            // `unsafe` (spec 6.4) only lets the checker allow more inside;
            // the code is a block like any other.
            ExprKind::Arena(b) => return self.arena(b, want_value, ty, line),
            ExprKind::Block(b) | ExprKind::Unsafe(b) => {
                K::Block(self.branch_block(b, want_value, ty)?)
            }
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => {
                let cond = self.expr(cond)?;
                let then_block = self.branch_block(then, want_value, ty)?;
                let else_block = self.else_block(otherwise.as_deref(), want_value, ty)?;
                K::If(Box::new(cond), then_block, else_block)
            }
            ExprKind::IfLet {
                name,
                value,
                then,
                otherwise,
            } => {
                let value = self.expr(value)?;
                let Some(Shape::Optional(inner)) = self.shapes.shape(value.ty).cloned() else {
                    return Err(self.unsupported("this `if let`", e.span));
                };
                let tmp = self.new_local(value.ty);
                let bound = self.new_local(inner);
                self.bind(name, bound);
                let mut then_block = self.branch_block(then, want_value, ty)?;
                let unwrapped = ir::Expr {
                    kind: K::Unwrap(Box::new(self.local_expr(tmp, line))),
                    ty: inner,
                    line,
                };
                then_block.stmts.insert(0, Stmt::Let(bound, unwrapped));
                let else_block = self.else_block(otherwise.as_deref(), want_value, ty)?;
                let present = ir::Expr {
                    kind: K::IsSome(Box::new(self.local_expr(tmp, line))),
                    ty: Type::Bool,
                    line,
                };
                let choice = ir::Expr {
                    kind: K::If(Box::new(present), then_block, else_block),
                    ty,
                    line,
                };
                K::Block(Block {
                    stmts: vec![Stmt::Let(tmp, value)],
                    value: Some(Box::new(choice)),
                })
            }
            ExprKind::Match { scrutinee, arms } => {
                return self.match_expr(scrutinee, arms, ty, want_value, line);
            }
            _ => return Err(self.unsupported("this expression", e.span)),
        };
        Ok(ir::Expr { kind, ty, line })
    }

    fn else_block(
        &mut self,
        otherwise: Option<&'a ast::Expr>,
        want_value: bool,
        ty: Type,
    ) -> Lowered<Block> {
        let Some(o) = otherwise else {
            return Ok(Block::default());
        };
        let other = self.branching(o, want_value)?;
        let other = self.coerce(other, ty)?;
        Ok(wrap(other, want_value))
    }

    pub(crate) fn branch_block(
        &mut self,
        b: &'a ast::Block,
        want_value: bool,
        ty: Type,
    ) -> Lowered<Block> {
        let mut block = Block::default();
        self.block_into(b, want_value, &mut block)?;
        if let Some(v) = block.value.take() {
            block.value = Some(Box::new(self.coerce(*v, ty)?));
        }
        Ok(block)
    }
}
