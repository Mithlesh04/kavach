// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A list's `sum(f)` of floats (spec 14.2, decision 0135): an exact sum,
//! which the runtime keeps (`ExactNew`). The loop pushes each value onto
//! a list of its own and hands the runtime a few thousand at a time
//! (`ExactAddList`), so it calls the runtime once for each batch, not
//! once for each item; `ExactEnd` rounds the sum once.

use crate::ir::{self, BinOp, Block, ExprKind as K, Local, Stmt, Type};
use crate::lower::Lowerer;
use crate::rt_op::RtOp;

/// How many values the loop gathers before it hands them over.
const BATCH: i128 = 4096;

/// The locals of a float sum: the runtime's sum, the values gathered
/// and how many.
#[derive(Clone, Copy)]
pub(crate) struct ExactSum {
    sum: Local,
    batch: Local,
    count: Local,
}

impl Lowerer<'_> {
    /// Starts a float sum, its statements pushed onto `stmts`.
    pub(crate) fn exact_sum(&mut self, stmts: &mut Vec<Stmt>, line: u32) -> ExactSum {
        let list = self.list_type(Type::F64);
        let s = ExactSum {
            sum: self.new_local(Type::I64),
            batch: self.new_local(list),
            count: self.new_local(Type::I64),
        };
        let made = self.at(K::Rt(RtOp::ExactNew, Vec::new()), Type::I64, line);
        stmts.push(Stmt::Let(s.sum, made));
        stmts.push(Stmt::Let(s.batch, self.at(K::List(Vec::new()), list, line)));
        stmts.push(Stmt::Let(s.count, self.at(K::Int(0), Type::I64, line)));
        s
    }

    /// Adds `value`, a `Float` or `Float32`, to the sum.
    pub(crate) fn exact_add(&mut self, s: ExactSum, value: ir::Expr, line: u32) -> ir::Expr {
        let list = self.list_type(Type::F64);
        let wide = match value.ty {
            Type::F64 => value,
            _ => self.at(K::Convert(Box::new(value), false), Type::F64, line),
        };
        let int = |this: &Self, n: i128| this.at(K::Int(n), Type::I64, line);
        let pushed = K::Rt(RtOp::ListPush, vec![self.local_expr(s.batch, line), wide]);
        let count = || Box::new(self.local_expr(s.count, line));
        let more = K::Binary(BinOp::Add, count(), Box::new(int(self, 1)));
        let full = K::Binary(BinOp::Eq, count(), Box::new(int(self, BATCH)));
        let emptied = K::Rt(
            RtOp::ListTruncate,
            vec![self.local_expr(s.batch, line), int(self, 0)],
        );
        let hand_over = Block {
            stmts: vec![
                Stmt::Expr(self.handed_over(s, line)),
                self.set(s.batch, self.at(emptied, list, line)),
                self.set(s.count, int(self, 0)),
            ],
            value: None,
        };
        let full = Box::new(self.at(full, Type::Bool, line));
        let stmts = vec![
            self.set(s.batch, self.at(pushed, list, line)),
            self.set(s.count, self.at(more, Type::I64, line)),
            Stmt::Expr(self.at(K::If(full, hand_over, Block::default()), Type::Unit, line)),
        ];
        self.at(K::Block(Block { stmts, value: None }), Type::Unit, line)
    }

    /// The sum, of type `ty`, rounded once, with the values left over.
    pub(crate) fn exact_end(&mut self, s: ExactSum, ty: Type, line: u32) -> ir::Expr {
        let end = K::Rt(RtOp::ExactEnd, vec![self.local_expr(s.sum, line)]);
        let block = Block {
            stmts: vec![Stmt::Expr(self.handed_over(s, line))],
            value: Some(Box::new(self.at(end, ty, line))),
        };
        self.at(K::Block(block), ty, line)
    }

    /// The values gathered, handed to the runtime's sum.
    fn handed_over(&self, s: ExactSum, line: u32) -> ir::Expr {
        let args = vec![self.local_expr(s.sum, line), self.local_expr(s.batch, line)];
        self.at(K::Rt(RtOp::ExactAddList, args), Type::Unit, line)
    }

    /// `local = value`, as a statement.
    fn set(&self, local: Local, value: ir::Expr) -> Stmt {
        let line = value.line;
        Stmt::Expr(self.at(K::Assign(local, Box::new(value)), Type::Unit, line))
    }
}
