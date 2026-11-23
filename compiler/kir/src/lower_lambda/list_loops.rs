// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A list's methods that take a lambda, as one loop over the list (spec
//! 2.2, 14.2), for `lower_lambda.rs`. A sum of floats is exact: the
//! runtime keeps it (`exact_sums.rs`).

use crate::ir::{self, BinOp, Block, ExprKind as K, Local, Shape, Stmt, Type, UnOp};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// `filter`, `map`, `flat_map`, `any`, `all` and `sum` with lambda `f`,
    /// as a loop over the list.
    pub(crate) fn list_loop(
        &mut self,
        method: &str,
        list: ir::Expr,
        f: &'a ast::Expr,
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let Some(Shape::List(item_ty)) = self.shapes.shape(list.ty).cloned() else {
            return Err(self.unsupported(format!("`{method}` here"), span));
        };
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let src = self.new_local(list.ty);
        let (item, index, acc) = (
            self.new_local(item_ty),
            self.new_local(Type::I64),
            self.new_local(ty),
        );
        let start = match method {
            "filter" | "map" | "flat_map" => K::List(Vec::new()),
            "find" | "find_index" | "find_last_index" => K::None,
            "for_each" => K::Block(Block::default()),
            "any" => K::Bool(false),
            "all" => K::Bool(true),
            _ if ty.is_float() => K::Float(0.0),
            _ if matches!(self.shapes.shape(ty), Some(Shape::Decimal)) => K::Decimal("0".into()),
            _ => K::Int(0),
        };
        let mut stmts = vec![Stmt::Let(src, list), Stmt::Let(acc, at(start, ty))];
        // A sum of floats is exact, rounded once (spec 14.2): the runtime
        // keeps it, and is given the values a few thousand at a time.
        let exact = (method == "sum" && ty.is_float()).then(|| self.exact_sum(&mut stmts, line));
        let item_value = self.local_expr(item, line);
        // A function that can fail is called, not inlined: its first
        // error ends the loop (spec 5.2).
        let (value, round) = match self.failing_rounds(f, item_value.clone())? {
            Some((setup, round, value)) => {
                stmts.extend(setup);
                (value, round)
            }
            None => (self.inline_lambda(f, vec![item_value], line)?, Vec::new()),
        };
        let body = match method {
            "filter" => {
                let keep = self.push_to(acc, self.local_expr(item, line));
                at(K::If(Box::new(value), keep, Block::default()), Type::Unit)
            }
            "map" => {
                let pushed = self.push_to(acc, value);
                at(K::Block(pushed), Type::Unit)
            }
            "any" | "all" => {
                // Every item is visited, like the interpreter, so a lambda's
                // side effects happen the same way.
                let flip = method == "all";
                let cond = if flip {
                    at(K::Unary(UnOp::Not, Box::new(value)), Type::Bool)
                } else {
                    value
                };
                let flag = at(K::Bool(!flip), Type::Bool);
                let set = self.assign(acc, flag);
                at(K::If(Box::new(cond), set, Block::default()), Type::Unit)
            }
            "count" => {
                let one = at(K::Int(1), ty);
                let total = self.local_expr(acc, line);
                let added = at(K::Binary(BinOp::Add, Box::new(total), Box::new(one)), ty);
                at(
                    K::If(Box::new(value), self.assign(acc, added), Block::default()),
                    Type::Unit,
                )
            }
            "find" => {
                // The first match is kept; every item is still visited, like
                // the interpreter, so a lambda's side effects match.
                let present = at(K::IsSome(Box::new(self.local_expr(acc, line))), Type::Bool);
                let first = at(K::Unary(UnOp::Not, Box::new(present)), Type::Bool);
                let cond = at(K::And(Box::new(value), Box::new(first)), Type::Bool);
                let found = self.coerce(self.local_expr(item, line), ty)?;
                at(
                    K::If(Box::new(cond), self.assign(acc, found), Block::default()),
                    Type::Unit,
                )
            }
            "for_each" => value,
            "find_index" | "find_last_index" => {
                // The first match's index, or the last's; every item is
                // still visited.
                let cond = match method {
                    "find_index" => {
                        let acc_now = Box::new(self.local_expr(acc, line));
                        let present = at(K::IsSome(acc_now), Type::Bool);
                        let first = at(K::Unary(UnOp::Not, Box::new(present)), Type::Bool);
                        at(K::And(Box::new(value), Box::new(first)), Type::Bool)
                    }
                    _ => value,
                };
                let found = self.coerce(self.local_expr(index, line), ty)?;
                at(
                    K::If(Box::new(cond), self.assign(acc, found), Block::default()),
                    Type::Unit,
                )
            }
            "sum" if let Some(sum) = exact => self.exact_add(sum, value, line),
            "sum" => {
                let total = self.local_expr(acc, line);
                let added = match self.whole_at(span) {
                    // A sum of `Int<bits: n>`s is checked at `n` bits (2.1).
                    Some(p) => self.whole_binary(ast::BinaryOp::Add, total, value, p, line),
                    None => at(K::Binary(BinOp::Add, Box::new(total), Box::new(value)), ty),
                };
                at(K::Block(self.assign(acc, added)), Type::Unit)
            }
            _ => self.flat_push(acc, value, line)?,
        };
        let mut each = round;
        each.push(Stmt::Expr(body));
        stmts.push(Stmt::ForEach {
            var: item,
            index,
            list: src,
            body: Block {
                stmts: each,
                value: None,
            },
        });
        let result = match exact {
            Some(sum) => self.exact_end(sum, ty, line),
            None => self.local_expr(acc, line),
        };
        Ok(at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(result)),
            }),
            ty,
        ))
    }

    /// A block that sets local `acc` to `value`.
    pub(crate) fn assign(&self, acc: Local, value: ir::Expr) -> Block {
        let line = value.line;
        Block {
            stmts: vec![Stmt::Expr(ir::Expr {
                kind: K::Assign(acc, Box::new(value)),
                ty: Type::Unit,
                line,
            })],
            value: None,
        }
    }

    /// A block that pushes `value` onto the list in local `acc`.
    pub(crate) fn push_to(&self, acc: Local, value: ir::Expr) -> Block {
        let line = value.line;
        let list_ty = self.locals[acc as usize];
        let pushed = ir::Expr {
            kind: K::Rt(RtOp::ListPush, vec![self.local_expr(acc, line), value]),
            ty: list_ty,
            line,
        };
        self.assign(acc, pushed)
    }
}
