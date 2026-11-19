// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
    // Code that runs on the GPU is never cancelled there, and WGSL has no
    // stop point to write.
    let gpu = gpu_functions(functions);
    for (_, f) in
        (functions.iter_mut().enumerate()).filter(|(i, f)| f.external.is_none() && !gpu.contains(i))
    {
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

/// The functions that run on the GPU: those a GPU `map`, `filter` or
/// reduction, or a draw pipeline, is given, and the ones they call.
fn gpu_functions(functions: &[Function]) -> std::collections::HashSet<usize> {
    let mut found: Vec<usize> = Vec::new();
    for f in functions {
        any_in_block(&f.body, &mut |e| {
            if let ExprKind::Rt(op, args) = &e.kind
                && on_gpu(*op)
            {
                for a in args {
                    if let ExprKind::Closure { func, .. } = &a.kind {
                        found.push(*func as usize);
                    }
                }
            }
            false
        });
    }
    let mut seen = std::collections::HashSet::new();
    while let Some(i) = found.pop() {
        if !seen.insert(i) {
            continue;
        }
        if let Some(f) = functions.get(i) {
            any_in_block(&f.body, &mut |e| {
                if let ExprKind::Call(callee, _) = &e.kind {
                    found.push(*callee as usize);
                }
                false
            });
        }
    }
    seen
}

/// Ops whose closures run on the GPU.
fn on_gpu(op: RtOp) -> bool {
    matches!(
        op,
        RtOp::GpuMap
            | RtOp::GpuMapStay
            | RtOp::GpuFilter
            | RtOp::GpuFilterStay
            | RtOp::GpuSum
            | RtOp::GpuMin
            | RtOp::GpuMax
            | RtOp::GpuSumLater
            | RtOp::GpuMinLater
            | RtOp::GpuMaxLater
            | RtOp::GpuWarm
            | RtOp::DrawPipeline
    )
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
            variadic: false,
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
