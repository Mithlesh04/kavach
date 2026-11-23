// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `xs.group_by(key).map_values(g => g.sum(f))` (spec 14.2, decision
//! 0135) as one loop over the items, with a running sum for each key in
//! a dict, so no groups are made. Whole numbers add as `sum` adds them,
//! checked; floats into an exact sum for each key (`ExactNew`), each
//! rounded once at the end. The checker's `grouped_sum` says which calls
//! are one; a sum of `Decimal`s, or an `f` that can fail, is left to the
//! methods themselves.

use crate::ir::{self, BinOp, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// `e`, giving `ty`, as a sum per key, or `None` when it isn't one.
    pub(crate) fn grouped_sum(&mut self, e: &'a ast::Expr, ty: Type) -> Option<Lowered<ir::Expr>> {
        let parts = kavach_checker::grouped_sum(e, &self.c.expr_types, &self.c.resolution)?;
        let Some(Shape::Map(key_ty, sum_ty)) = self.shapes.shape(ty).cloned() else {
            return None;
        };
        if sum_ty.is_ref() || sum_ty == Type::Bool || sum_ty == Type::Unit {
            return None;
        }
        let failing = self.ty_of(parts.value).ok().and_then(|t| self.fn_error(t));
        if failing.is_some() {
            return None;
        }
        Some(self.sum_per_key(&parts, ty, key_ty, sum_ty, e.span))
    }

    fn sum_per_key(
        &mut self,
        parts: &kavach_checker::GroupedSum<'a>,
        ty: Type,
        key_ty: Type,
        sum_ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let mut list = self.expr(parts.items)?;
        // A table's rows are its field `kept` (`stdlib/table/table.kav`).
        if let Some(Shape::Record { fields, .. }) = self.shapes.shape(list.ty).cloned()
            && let Some(at) = fields.iter().position(|f| f.name == "kept")
        {
            let kept = fields[at].ty;
            list = self.at(K::GetField(Box::new(list), at as u32), kept, line);
        }
        let Some(Shape::List(item_ty)) = self.shapes.shape(list.ty).cloned() else {
            return Err(self.unsupported("`group_by` here", span));
        };
        let float = sum_ty.is_float();
        // Floats: each key's exact sum, by its address; else the sums.
        let held_ty = if float {
            self.map_type(key_ty, Type::I64)
        } else {
            ty
        };
        let (src, held, out) = (
            self.new_local(list.ty),
            self.new_local(held_ty),
            self.new_local(ty),
        );
        let (item, index) = (self.new_local(item_ty), self.new_local(Type::I64));
        let (k, v) = (self.new_local(key_ty), self.new_local(sum_ty));
        let item_value = self.local_expr(item, line);
        let key = self.inline_lambda(parts.key, vec![item_value.clone()], line)?;
        let value = self.inline_lambda(parts.value, vec![item_value], line)?;
        let mut body = vec![Stmt::Let(k, key), Stmt::Let(v, value)];
        let get_or = |this: &Self, map: ir::Local, k: ir::Local, value_ty: Type| {
            let args = vec![
                this.local_expr(map, line),
                this.local_expr(k, line),
                this.at(K::Int(0), value_ty, line),
            ];
            this.at(K::Rt(RtOp::MapGetOr, args), value_ty, line)
        };
        let insert = |this: &Self, map: ir::Local, map_ty: Type, k: ir::Local, value: ir::Expr| {
            let args = vec![this.local_expr(map, line), this.local_expr(k, line), value];
            let put = this.at(K::Rt(RtOp::MapInsert, args), map_ty, line);
            Stmt::Expr(this.at(K::Assign(map, Box::new(put)), Type::Unit, line))
        };
        if float {
            // The key's sum, made the first time the key is seen.
            let at = self.new_local(Type::I64);
            body.push(Stmt::Let(at, get_or(self, held, k, Type::I64)));
            let compact = vec![self.at(K::Int(1), Type::I64, line)];
            let made = self.at(K::Rt(RtOp::ExactNew, compact), Type::I64, line);
            let new = Block {
                stmts: vec![
                    Stmt::Expr(self.at(K::Assign(at, Box::new(made)), Type::Unit, line)),
                    insert(self, held, held_ty, k, self.local_expr(at, line)),
                ],
                value: None,
            };
            let zero = self.at(K::Int(0), Type::I64, line);
            let cond = self.at(
                K::Binary(
                    BinOp::Eq,
                    Box::new(self.local_expr(at, line)),
                    Box::new(zero),
                ),
                Type::Bool,
                line,
            );
            body.push(Stmt::Expr(self.at(
                K::If(Box::new(cond), new, Block::default()),
                Type::Unit,
                line,
            )));
            let wide = match sum_ty {
                Type::F64 => self.local_expr(v, line),
                _ => self.at(
                    K::Convert(Box::new(self.local_expr(v, line)), false),
                    Type::F64,
                    line,
                ),
            };
            let add = vec![self.local_expr(at, line), wide];
            body.push(Stmt::Expr(self.at(
                K::Rt(RtOp::ExactAdd, add),
                Type::Unit,
                line,
            )));
        } else {
            // A whole-number sum is checked as `sum` checks it (2.1).
            let before = get_or(self, held, k, sum_ty);
            let now = self.local_expr(v, line);
            let added = match self.whole_at(parts.sum.span) {
                Some(p) => self.whole_binary(ast::BinaryOp::Add, before, now, p, line),
                None => self.at(
                    K::Binary(BinOp::Add, Box::new(before), Box::new(now)),
                    sum_ty,
                    line,
                ),
            };
            body.push(insert(self, held, held_ty, k, added));
        }
        let empty = |this: &Self, t| this.at(K::Map(Vec::new()), t, line);
        let mut stmts = vec![
            Stmt::Let(src, list),
            Stmt::Let(held, empty(self, held_ty)),
            Stmt::ForEach {
                var: item,
                index,
                list: src,
                body: Block {
                    stmts: body,
                    value: None,
                },
            },
        ];
        if float {
            // Each key's sum, rounded once, in the order first seen.
            let keys_ty = self.list_type(key_ty);
            let keys = self.new_local(keys_ty);
            let (key, i) = (self.new_local(key_ty), self.new_local(Type::I64));
            let all = self.at(
                K::Rt(RtOp::MapKeys, vec![self.local_expr(held, line)]),
                keys_ty,
                line,
            );
            let at = get_or(self, held, key, Type::I64);
            let sum = self.at(K::Rt(RtOp::ExactEnd, vec![at]), sum_ty, line);
            stmts.push(Stmt::Let(out, empty(self, ty)));
            stmts.push(Stmt::Let(keys, all));
            stmts.push(Stmt::ForEach {
                var: key,
                index: i,
                list: keys,
                body: Block {
                    stmts: vec![insert(self, out, ty, key, sum)],
                    value: None,
                },
            });
        } else {
            stmts.push(Stmt::Let(out, self.local_expr(held, line)));
        }
        let block = Block {
            stmts,
            value: Some(Box::new(self.local_expr(out, line))),
        };
        Ok(self.at(K::Block(block), ty, line))
    }
}
