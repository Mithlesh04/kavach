//! Where a task cancelled with `task.cancel()` stops even when it only
//! computes (spec 9.6, decision 0075): the start of each function and the
//! start of each loop's round (which is where the last round ended, after
//! a `continue` too). A stop point is a runtime op that, for a task that
//! must stop, sets the cancelling flag as a cancelled wait does, so
//! `cancel.rs` puts its return-at-once check after it and after the calls
//! that reach it.
//!
//! A task given a time limit (`spawn … within d`) checks its deadline at
//! the same points. Only a program that cancels this way, or gives a time
//! limit, gets them: whether one does is known when it is compiled, so
//! other programs pay nothing. A counted
//! loop whose body runs straight through (no call, no inner loop) gets
//! none: it ends in time anyway, and stays one LLVM can vectorize or the
//! compiler can run on all cores.

use crate::ir::*;
use crate::rt_op::RtOp;
use crate::walk::{any_in_block, blocks_mut};

pub fn add_stop_points(functions: &mut [Function]) {
    let cancels = |e: &Expr| matches!(e.kind, ExprKind::Rt(RtOp::TaskCancel | RtOp::TaskWithin, _));
    if !functions
        .iter()
        .any(|f| any_in_block(&f.body, &mut { cancels }))
    {
        return;
    }
    for f in functions.iter_mut().filter(|f| f.external.is_none()) {
        let line = f.line;
        blocks_mut(&mut f.body, &mut |b| {
            for s in &mut b.stmts {
                let body = match s {
                    Stmt::While { body, .. } | Stmt::ForEach { body, .. } => body,
                    Stmt::ForRange { body, .. } if !straight(body) => body,
                    _ => continue,
                };
                let line = body.stmts.first().map_or(line, stmt_line);
                body.stmts.insert(0, stop_point(line));
            }
        });
        f.body.stmts.insert(0, stop_point(line));
    }
}

fn stop_point(line: u32) -> Stmt {
    Stmt::Expr(Expr {
        kind: ExprKind::Rt(RtOp::TaskStopPoint, Vec::new()),
        ty: Type::Unit,
        line,
    })
}

fn stmt_line(s: &Stmt) -> u32 {
    match s {
        Stmt::Expr(e) | Stmt::Let(_, e) => e.line,
        _ => 0,
    }
}

/// Whether a counted loop's `body` runs straight through.
fn straight(body: &Block) -> bool {
    let calls = any_in_block(body, &mut |e| {
        matches!(e.kind, ExprKind::Call(..) | ExprKind::CallValue(..))
    });
    let loops = body.stmts.iter().any(|s| {
        matches!(
            s,
            Stmt::While { .. } | Stmt::ForRange { .. } | Stmt::ForEach { .. }
        )
    });
    !calls && !loops
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(kind: ExprKind, ty: Type) -> Expr {
        Expr { kind, ty, line: 1 }
    }

    fn function(stmts: Vec<Stmt>) -> Function {
        Function {
            name: "f".into(),
            params: Vec::new(),
            ret: Type::Unit,
            locals: vec![Type::I64],
            body: Block { stmts, value: None },
            line: 1,
            fails: false,
            external: None,
        }
    }

    fn stops(b: &Block) -> usize {
        let mut n = 0;
        any_in_block(b, &mut |x| {
            n += usize::from(matches!(x.kind, ExprKind::Rt(RtOp::TaskStopPoint, _)));
            false
        });
        n
    }

    fn counted(body: Vec<Stmt>) -> Stmt {
        Stmt::ForRange {
            var: 0,
            start: e(ExprKind::Int(0), Type::I64),
            end: e(ExprKind::Int(10), Type::I64),
            inclusive: false,
            body: Block {
                stmts: body,
                value: None,
            },
        }
    }

    /// A `while` and a counted loop with a call get a stop point each, a
    /// straight counted loop none, and the function one at its start.
    #[test]
    fn loops_that_may_run_long_and_function_starts_get_stop_points() {
        let call = Stmt::Expr(e(ExprKind::Call(0, Vec::new()), Type::Unit));
        let forever = Stmt::While {
            cond: e(ExprKind::Bool(true), Type::Bool),
            body: Block::default(),
        };
        let cancel = Stmt::Expr(e(
            ExprKind::Rt(RtOp::TaskCancel, vec![e(ExprKind::None, Type::I64)]),
            Type::Unit,
        ));
        let f = function(vec![
            cancel,
            forever,
            counted(vec![call]),
            counted(Vec::new()),
        ]);
        let mut functions = vec![f];
        add_stop_points(&mut functions);
        assert_eq!(stops(&functions[0].body), 3);
    }

    /// A program that never calls `cancel()` gets none.
    #[test]
    fn programs_without_cancel_get_none() {
        let forever = Stmt::While {
            cond: e(ExprKind::Bool(true), Type::Bool),
            body: Block::default(),
        };
        let mut functions = vec![function(vec![forever])];
        add_stop_points(&mut functions);
        assert_eq!(stops(&functions[0].body), 0);
    }
}
