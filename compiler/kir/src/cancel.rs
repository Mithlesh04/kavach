//! Cancellation in native code (spec 9.6). A cancelled task's waits end
//! at once and set the runtime's *cancelling* flag
//! (`runtime/native/src/rt_tasks.rs`). After each wait, and after each
//! call that can wait, a function that can wait checks the flag and, when
//! it is set, returns at once: its locals are released as on any return,
//! and its caller does the same, up to the task's thunk, which ends the
//! task. Nothing else changes: functions keep their signatures, `catch`
//! never sees cancelling (it isn't an error), and code that never waits
//! has no checks.
//!
//! A function can wait when it waits itself, calls a function that can
//! wait, or calls a function value while some function used as a value
//! can wait.

use crate::ir::*;
use crate::rt_op::RtOp;
use crate::walk::{any_in_block, blocks_mut, exprs_mut};

/// Waits that a cancelled task stops at. (A function waiting for the
/// tasks it spawned, `TaskJoin`, waits for them even then: spec 9.2.)
fn waits(op: RtOp) -> bool {
    matches!(
        op,
        RtOp::TaskWait
            | RtOp::TaskWaitFor
            | RtOp::ChanPut
            | RtOp::ChanPutFor
            | RtOp::ChanTake
            | RtOp::ChanTakeFor
            | RtOp::TimeSleepMs
            | RtOp::TimeWaitUntil
            | RtOp::AiAsk
            | RtOp::AgentDecide
            | RtOp::AgentReplayWait
            | RtOp::NetConnectQuic
            | RtOp::NetOpenStream
            | RtOp::NetWriteStream
            | RtOp::NetReadStream
            | RtOp::NetConnect
            | RtOp::NetConnectTls
            | RtOp::NetSend
            | RtOp::NetReceive
            | RtOp::NetAccept
            | RtOp::NetRead
            | RtOp::NetWrite
            | RtOp::TaskStopPoint
    )
}

pub fn cancel_checks(functions: &mut [Function]) {
    let n = functions.len();
    let mut values = vec![false; n];
    for f in functions.iter() {
        any_in_block(&f.body, &mut |e| {
            if let ExprKind::Closure { func, .. } = e.kind {
                values[func as usize] = true;
            }
            false
        });
    }
    let mut can_wait = vec![false; n];
    loop {
        let values_wait = (0..n).any(|i| values[i] && can_wait[i]);
        let mut changed = false;
        for (i, f) in functions.iter().enumerate() {
            if !can_wait[i] && any_in_block(&f.body, &mut |e| waiting(e, &can_wait, values_wait)) {
                can_wait[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let values_wait = (0..n).any(|i| values[i] && can_wait[i]);
    for (i, f) in functions.iter_mut().enumerate() {
        if can_wait[i] {
            // A printed line comes out whole: its waits first (spec 9.8).
            crate::whole_lines::print_whole_lines(f, &mut |e| waiting(e, &can_wait, values_wait));
            add_checks(f, &can_wait, values_wait);
        }
    }
}

/// Whether `e` itself can wait.
fn waiting(e: &Expr, can_wait: &[bool], values_wait: bool) -> bool {
    match &e.kind {
        ExprKind::Rt(op, _) => waits(*op),
        ExprKind::Call(g, _) => can_wait[*g as usize],
        ExprKind::CallValue(..) => values_wait,
        _ => false,
    }
}

fn add_checks(f: &mut Function, can_wait: &[bool], values_wait: bool) {
    let (ret, locals) = (f.ret, &mut f.locals);
    exprs_mut(&mut f.body, &mut |e| {
        if !waiting(e, can_wait, values_wait) {
            return;
        }
        let (ty, line) = (e.ty, e.line);
        let inner = std::mem::replace(
            e,
            Expr {
                kind: ExprKind::None,
                ty,
                line,
            },
        );
        let mut stmts = Vec::new();
        let value = if ty == Type::Unit {
            stmts.push(Stmt::Expr(inner));
            None
        } else {
            locals.push(ty);
            let t = (locals.len() - 1) as Local;
            stmts.push(Stmt::Let(t, inner));
            Some(Box::new(Expr {
                kind: ExprKind::Local(t),
                ty,
                line,
            }))
        };
        stmts.push(check(ret, line));
        e.kind = ExprKind::Block(Block { stmts, value });
    });
    // A call that can fail stays as it is in its `LetResult`: its check
    // goes after it.
    blocks_mut(&mut f.body, &mut |b| {
        let mut i = 0;
        while i < b.stmts.len() {
            if let Stmt::LetResult { call, .. } = &mut b.stmts[i]
                && let Some(after) = unwrap_check(call)
            {
                b.stmts.insert(i + 1, after);
                i += 1;
            }
            i += 1;
        }
    });
}

/// `if cancelling { return }`, with a placeholder value for a function
/// that gives `ret` (nothing will use it).
fn check(ret: Type, line: u32) -> Stmt {
    let at = |kind, ty| Expr { kind, ty, line };
    let placeholder = match ret {
        Type::Unit => None,
        Type::F64 | Type::F32 => Some(at(ExprKind::Float(0.0), ret)),
        Type::Bool => Some(at(ExprKind::Bool(false), ret)),
        Type::Char => Some(at(ExprKind::Char('\0'), ret)),
        t if t.is_int() => Some(at(ExprKind::Int(0), ret)),
        // A heap value: the null pointer, which releasing skips.
        _ => Some(at(ExprKind::None, ret)),
    };
    let cancelling = at(ExprKind::Rt(RtOp::TaskCancelling, Vec::new()), Type::Bool);
    let leave = Block {
        stmts: vec![Stmt::Return(placeholder)],
        value: None,
    };
    Stmt::Expr(at(
        ExprKind::If(Box::new(cancelling), leave, Block::default()),
        Type::Unit,
    ))
}

/// For a call that `add_checks` wrapped: puts the call back, and gives
/// its check.
fn unwrap_check(call: &mut Expr) -> Option<Stmt> {
    let ExprKind::Block(b) = &mut call.kind else {
        return None;
    };
    if b.stmts.len() != 2
        || !matches!(&b.stmts[1], Stmt::Expr(Expr { kind: ExprKind::If(c, ..), .. })
        if matches!(c.kind, ExprKind::Rt(RtOp::TaskCancelling, _)))
    {
        return None;
    }
    let after = b.stmts.pop().expect("a check");
    let inner = match b.stmts.pop().expect("the call") {
        Stmt::Let(_, inner) | Stmt::Expr(inner) => inner,
        _ => return None,
    };
    *call = inner;
    Some(after)
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
            locals: Vec::new(),
            body: Block { stmts, value: None },
            line: 1,
            fails: false,
            external: None,
        }
    }

    fn checks(f: &Function) -> usize {
        let mut n = 0;
        any_in_block(&f.body, &mut |x| {
            n += usize::from(matches!(x.kind, ExprKind::Rt(RtOp::TaskCancelling, _)));
            false
        });
        n
    }

    #[test]
    fn waits_and_calls_that_can_wait_get_a_check_and_others_none() {
        let sleep = e(
            ExprKind::Rt(RtOp::TimeSleepMs, vec![e(ExprKind::Int(5), Type::I64)]),
            Type::Unit,
        );
        let sleeper = function(vec![Stmt::Expr(sleep)]);
        let caller = function(vec![Stmt::Expr(e(
            ExprKind::Call(0, Vec::new()),
            Type::Unit,
        ))]);
        let adder = e(
            ExprKind::Binary(
                BinOp::Add,
                Box::new(e(ExprKind::Int(1), Type::I64)),
                Box::new(e(ExprKind::Int(2), Type::I64)),
            ),
            Type::I64,
        );
        let computer = function(vec![Stmt::Expr(adder)]);
        let mut functions = vec![sleeper, caller, computer];
        cancel_checks(&mut functions);
        let counts: Vec<usize> = functions.iter().map(checks).collect();
        assert_eq!(counts, [1, 1, 0]);
    }
}
