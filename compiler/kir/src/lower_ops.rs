// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The operator interfaces' methods on numbers and text (spec 2.16): a
//! generic `T: Add`'s `a + b` is `a.add(b)`, which for a number is its own
//! `+`, so each copy of the function runs it as one instruction.
//! `compare` gives -1, 0 or 1, and 0 when neither comes first (a `NaN`).
//! And compound assignment, `x op= v`.

use crate::ir::{self, BinOp, Block, ExprKind as K, Shape, Stmt, Type, UnOp};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// `a.name(b)` (or `a.neg()`) on a number or text, with `values` the
    /// receiver then the argument; `None` when `name` isn't one of these.
    /// `bit_and`, `bit_or` and `bit_not` are a whole number's `&`, `|` and
    /// `~`, and a `Bool`'s `and`, `or` and `not` (`bool_op`).
    pub(crate) fn op_method(
        &mut self,
        name: &str,
        mut values: Vec<ir::Expr>,
        ty: Type,
        line: u32,
    ) -> Option<ir::Expr> {
        let op = match name {
            "add" => BinOp::Add,
            "sub" => BinOp::Sub,
            "mul" => BinOp::Mul,
            "div" => BinOp::Div,
            "rem" => BinOp::Rem,
            "bit_and" if values[0].ty.is_int() => BinOp::BitAnd,
            "bit_or" if values[0].ty.is_int() => BinOp::BitOr,
            "neg" => {
                let x = values.pop()?;
                return Some(self.at(K::Unary(UnOp::Neg, Box::new(x)), ty, line));
            }
            "bit_not" if values[0].ty.is_int() => {
                let x = values.pop()?;
                return Some(self.at(K::Unary(UnOp::BitNot, Box::new(x)), ty, line));
            }
            "compare" => return self.compare(values, line),
            _ => return None,
        };
        let r = values.pop()?;
        let l = values.pop()?;
        Some(self.at(K::Binary(op, Box::new(l), Box::new(r)), ty, line))
    }

    /// `a.bit_and(b)`, `a.bit_or(b)` or `a.bit_not()` on `Bool`s: `and`,
    /// `or` and `not`, with both sides worked out first (spec 2.16).
    pub(crate) fn bool_op(
        &mut self,
        name: &str,
        mut values: Vec<ir::Expr>,
        line: u32,
    ) -> Option<ir::Expr> {
        if name == "bit_not" {
            let x = values.pop()?;
            return Some(self.at(K::Unary(UnOp::Not, Box::new(x)), Type::Bool, line));
        }
        let b = values.pop()?;
        let a = values.pop()?;
        let (la, lb) = (self.new_local(a.ty), self.new_local(b.ty));
        let (x, y) = (self.local_expr(la, line), self.local_expr(lb, line));
        let both = match name {
            "bit_and" => K::And(Box::new(x), Box::new(y)),
            "bit_or" => K::Or(Box::new(x), Box::new(y)),
            _ => return None,
        };
        let both = self.at(both, Type::Bool, line);
        Some(self.at(
            K::Block(Block {
                stmts: vec![Stmt::Let(la, a), Stmt::Let(lb, b)],
                value: Some(Box::new(both)),
            }),
            Type::Bool,
            line,
        ))
    }

    /// `a.compare(b)`: -1 when `a` comes first, 1 when `b` does, else 0.
    fn compare(&mut self, mut values: Vec<ir::Expr>, line: u32) -> Option<ir::Expr> {
        let b = values.pop()?;
        let a = values.pop()?;
        let (la, lb) = (self.new_local(a.ty), self.new_local(b.ty));
        let test = |me: &Self, op| {
            let (x, y) = (me.local_expr(la, line), me.local_expr(lb, line));
            me.at(K::Binary(op, Box::new(x), Box::new(y)), Type::Bool, line)
        };
        let int = |me: &Self, n| me.at(K::Int(n), Type::I64, line);
        let value = |e| Block {
            stmts: Vec::new(),
            value: Some(Box::new(e)),
        };
        let after = self.at(
            K::If(
                Box::new(test(self, BinOp::Gt)),
                value(int(self, 1)),
                value(int(self, 0)),
            ),
            Type::I64,
            line,
        );
        let whole = self.at(
            K::If(
                Box::new(test(self, BinOp::Lt)),
                value(int(self, -1)),
                value(after),
            ),
            Type::I64,
            line,
        );
        Some(self.at(
            K::Block(Block {
                stmts: vec![Stmt::Let(la, a), Stmt::Let(lb, b)],
                value: Some(Box::new(whole)),
            }),
            Type::I64,
            line,
        ))
    }

    /// The new value of `target op= value` (spec 1.8): `ages[name] += 1`
    /// changes the value under that key, and a list grows by another.
    pub(crate) fn compound(
        &mut self,
        bin: ast::BinaryOp,
        target: &'a ast::Expr,
        value: ir::Expr,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let mut current = self.expr(target)?;
        if let Some(Shape::Optional(inner)) = self.shapes.shape(current.ty).cloned() {
            // `ages[name] += 1` changes the value under that key;
            // a missing key panics, naming it.
            current = match current.kind {
                ir::ExprKind::Rt(RtOp::MapGet, _) => self.dict_entry(ir::Expr {
                    ty: inner,
                    ..current
                }),
                _ => ir::Expr {
                    kind: ir::ExprKind::Unwrap(Box::new(current)),
                    ty: inner,
                    line,
                },
            };
        }
        let ty = current.ty;
        let whole = self.whole_of(target);
        Ok(match crate::lower_expr::binary_op(bin) {
            // `x += 1` on an `Int<bits: n>` or a range (spec 2.1).
            _ if let Some(p) = whole => self.whole_binary(bin, current, value, p, line),
            _ if bin == ast::BinaryOp::Pow => self.power(current, value, line),
            _ if self.is_list_type(ty) => self.list_concat(current, value, line, true)?,
            Some(bin) => ir::Expr {
                kind: ir::ExprKind::Binary(bin, Box::new(current), Box::new(value)),
                ty,
                line,
            },
            None => {
                return Err(self.unsupported(format!("`{}=`", bin.as_str()), span));
            }
        })
    }
}
