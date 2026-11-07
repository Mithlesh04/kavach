//! Lowering `for tick in time.every_ms(ms)` (spec 14.4): a loop whose
//! rounds each wait for their start.

use crate::ir::{self, BinOp, Block, ExprKind as K, Stmt, Type};
use crate::lower::Lowerer;
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// `for tick in time.every_ms(ms) { … }` (spec 14.4): each round waits
    /// until its start, which is `ms` after the one before started.
    pub(crate) fn every_loop(
        &mut self,
        tick: Option<&'a ast::Ident>,
        period: ir::Expr,
        body: ir::Block,
        line: u32,
    ) -> Vec<Stmt> {
        let int = Type::I64;
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let (ms, next, now, count) = (
            self.new_local(int),
            self.new_local(int),
            self.new_local(int),
            self.new_local(int),
        );
        let clock = at(K::Rt(RtOp::TimeMonotonicMs, vec![at(K::Int(0), int)]), int);
        let plus = |s: &Self, a: ir::Local, b: ir::Local| {
            at(
                K::Binary(
                    BinOp::Add,
                    Box::new(s.local_expr(a, line)),
                    Box::new(s.local_expr(b, line)),
                ),
                int,
            )
        };
        let mut round = vec![
            Stmt::Let(
                now,
                at(
                    K::Rt(RtOp::TimeWaitUntil, vec![self.local_expr(next, line)]),
                    int,
                ),
            ),
            Stmt::Expr(at(
                K::Assign(next, Box::new(plus(self, now, ms))),
                Type::Unit,
            )),
        ];
        let tick_local = self.new_local(int);
        if let Some(id) = tick {
            self.bind(id, tick_local);
        }
        round.push(Stmt::Let(tick_local, self.local_expr(count, line)));
        let one = at(K::Int(1), int);
        let counted = at(
            K::Binary(
                BinOp::Add,
                Box::new(self.local_expr(count, line)),
                Box::new(one),
            ),
            int,
        );
        round.push(Stmt::Expr(at(
            K::Assign(count, Box::new(counted)),
            Type::Unit,
        )));
        round.extend(body.stmts);
        if let Some(v) = body.value {
            round.push(Stmt::Expr(*v));
        }
        vec![
            Stmt::Let(ms, period),
            Stmt::Let(next, clock),
            Stmt::Expr(at(
                K::Assign(next, Box::new(plus(self, next, ms))),
                Type::Unit,
            )),
            Stmt::Let(count, at(K::Int(0), int)),
            Stmt::While {
                cond: at(K::Bool(true), Type::Bool),
                body: Block {
                    stmts: round,
                    value: None,
                },
            },
        ]
    }
}
