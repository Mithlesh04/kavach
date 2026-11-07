//! How much work a kernel does per item, roughly: the runtime sends a
//! `map` to the GPU only when the work is worth starting it for (spec
//! 15.2). One unit is one operation; a loop counts its body once per
//! round, and a call counts the function it calls.

use kavach_kir::{Block, Expr, ExprKind, FuncId, Program, Stmt, walk};
use std::collections::HashMap;

/// The rounds a loop is taken to run when they aren't known.
const UNKNOWN_ROUNDS: u64 = 100;

/// The work per item of function `fid` (a closure's function, or a
/// `gpu fn`).
pub fn work(program: &Program, fid: FuncId) -> u64 {
    Cost {
        program,
        done: HashMap::new(),
    }
    .function(fid)
}

struct Cost<'a> {
    program: &'a Program,
    /// The work of each function already counted; 1 while it is being
    /// counted, so a cycle can't loop (a `gpu fn` has none).
    done: HashMap<FuncId, u64>,
}

impl Cost<'_> {
    fn function(&mut self, fid: FuncId) -> u64 {
        if let Some(w) = self.done.get(&fid) {
            return *w;
        }
        self.done.insert(fid, 1);
        let w = match self.program.functions.get(fid as usize) {
            Some(f) => self.block(&f.body),
            None => 1,
        };
        self.done.insert(fid, w);
        w
    }

    fn block(&mut self, b: &Block) -> u64 {
        let stmts: u64 = b.stmts.iter().map(|s| self.stmt(s)).sum();
        stmts + b.value.as_ref().map_or(0, |v| self.expr(v))
    }

    fn stmt(&mut self, s: &Stmt) -> u64 {
        match s {
            Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => self.expr(e),
            Stmt::While { cond, body } => {
                UNKNOWN_ROUNDS.saturating_mul(self.expr(cond) + self.block(body))
            }
            Stmt::ForRange {
                start,
                end,
                inclusive,
                body,
                ..
            } => {
                let rounds = match (&start.kind, &end.kind) {
                    (ExprKind::Int(a), ExprKind::Int(b)) => {
                        (b - a + i128::from(*inclusive)).max(0) as u64
                    }
                    _ => UNKNOWN_ROUNDS,
                };
                let each = self.block(body) + 1;
                self.expr(start) + self.expr(end) + rounds.saturating_mul(each)
            }
            _ => 1,
        }
    }

    fn expr(&mut self, e: &Expr) -> u64 {
        match &e.kind {
            // A branch counts its longer side.
            ExprKind::If(cond, then, otherwise) => {
                let sides = self.block(then).max(self.block(otherwise));
                self.expr(cond) + sides + 1
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::Call(fid, args) => {
                let args: u64 = args.iter().map(|a| self.expr(a)).sum();
                args + self.function(*fid) + 1
            }
            _ => {
                // Every operation inside, and the functions it calls.
                let mut calls = Vec::new();
                let mut n = 0u64;
                walk::any_expr(e, &mut |x| {
                    n += 1;
                    if let ExprKind::Call(fid, _) = x.kind {
                        calls.push(fid);
                    }
                    false
                });
                n + calls.into_iter().map(|f| self.function(f)).sum::<u64>()
            }
        }
    }
}
