// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The operator interfaces' methods on numbers and text (spec 2.16): a
//! generic `T: Add`'s `a + b` is `a.add(b)`, which for a number is its own
//! `+`, so each copy of the function runs it as one instruction.
//! `compare` gives -1, 0 or 1, and 0 when neither comes first (a `NaN`).

use crate::ir::{self, BinOp, Block, ExprKind as K, Stmt, Type, UnOp};
use crate::lower::Lowerer;

impl Lowerer<'_> {
    /// `a.name(b)` (or `a.neg()`) on a number or text, with `values` the
    /// receiver then the argument; `None` when `name` isn't one of these.
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
            "neg" => {
                let x = values.pop()?;
                return Some(self.at(K::Unary(UnOp::Neg, Box::new(x)), ty, line));
            }
            "compare" => return self.compare(values, line),
            _ => return None,
        };
        let r = values.pop()?;
        let l = values.pop()?;
        Some(self.at(K::Binary(op, Box::new(l), Box::new(r)), ty, line))
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
}
