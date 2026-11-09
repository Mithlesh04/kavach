//! Which statements of a loop can run later (spec 15.2).

use super::{defined_in, mention_counts, one_statement, take_out_list};
use crate::gpu_stay;
use crate::ir::{Block, Expr, ExprKind as K, Local, Stmt};
use crate::rt_op::RtOp;
use crate::walk;
use std::collections::HashSet;

/// The locals set by a statement of this loop (not of a loop inside it)
/// from a sum on the GPU.
pub(super) fn accumulators(b: &Block, out: &mut Vec<Local>) {
    for s in &b.stmts {
        let Stmt::Expr(e) = s else { continue };
        match &e.kind {
            K::Assign(total, value) if holds_one_reduce(value) => out.push(*total),
            K::If(_, then, otherwise) => {
                accumulators(then, out);
                accumulators(otherwise, out);
            }
            K::Block(inner) => accumulators(inner, out),
            _ => {}
        }
    }
}

fn holds_one_reduce(e: &Expr) -> bool {
    let mut n = 0;
    walk::any_expr(e, &mut |x| {
        n += usize::from(reduce_of(x).is_some());
        false
    });
    n == 1
}

/// The op of `e`, when it is a `sum`, `min` or `max` that may run on the
/// GPU, as `lower_gpu.rs` writes it.
pub(super) fn reduce_of(e: &Expr) -> Option<RtOp> {
    let K::Block(b) = &e.kind else { return None };
    let (2, Some(Stmt::Let(..)), Some(Stmt::Let(_, gpu))) =
        (b.stmts.len(), b.stmts.first(), b.stmts.get(1))
    else {
        return None;
    };
    match gpu.kind {
        K::Rt(op @ (RtOp::GpuSum | RtOp::GpuMin | RtOp::GpuMax), _) => Some(op),
        _ => None,
    }
}

/// Whether the statement that sets `total` in the loop of `body` can run
/// later: it is the only mention of `total` in the loop, what it reads
/// doesn't change in the loop, it does nothing but work out a number, and
/// its list is on the GPU. If not, why, when its list is on the GPU.
pub(super) fn fits(
    body: &Block,
    total: Local,
    on_gpu: &HashSet<Local>,
) -> Result<(), Option<&'static str>> {
    let statement = find(body, total).ok_or(None)?;
    let K::Assign(_, value) = &statement.kind else {
        return Err(None);
    };
    let mut work = statement.clone();
    let sum = take_out_list(&mut work).ok_or(None)?;
    let list_on_gpu = gpu_stay::is_gpu_list(&sum.list)
        || matches!(sum.list.kind, K::Local(l) | K::Move(l) if on_gpu.contains(&l));
    let mentions = |b: &Block| mention_counts(b).get(&total).copied().unwrap_or(0);
    if !list_on_gpu {
        return Err(None);
    }
    if mentions(body) != mentions(&one_statement(statement)) {
        return Err(Some("the loop also reads what it adds up to"));
    }
    if !plain(value) {
        return Err(Some("the statement does more than work out numbers"));
    }
    if !exits_are_statements(body) {
        return Err(Some("the loop can leave in the middle of an expression"));
    }
    // What runs later reads these. The list and closure are worked out in
    // the loop, as they are now.
    let block = one_statement(&work);
    let mut defined = defined_in(&block);
    defined.extend([sum.held, sum.value]);
    let mut reads = HashSet::new();
    walk::locals_mut(&mut block.clone(), &mut |l| {
        reads.insert(*l);
    });
    reads.remove(&total);
    let unchanged = (reads.iter()).all(|l| defined.contains(l) || !walk::sets_local(body, *l));
    if !unchanged || has_exit(&block) {
        return Err(Some("the statement reads a value the loop changes"));
    }
    Ok(())
}

/// Only numbers worked out, around the reduction.
pub(super) fn plain(e: &Expr) -> bool {
    if reduce_of(e).is_some() {
        return true;
    }
    let block = |b: &Block| {
        b.stmts
            .iter()
            .all(|s| matches!(s, Stmt::Let(_, x) if plain(x)))
            && b.value.as_ref().is_none_or(|v| plain(v))
    };
    match &e.kind {
        K::Int(_) | K::Float(_) | K::Bool(_) | K::Local(_) | K::None => true,
        K::Unary(_, x) | K::Convert(x, _) | K::Some(x) | K::IsSome(x) | K::Unwrap(x) => plain(x),
        K::Binary(_, a, b) | K::Fast(_, a, b) | K::And(a, b) | K::Or(a, b) => plain(a) && plain(b),
        K::If(c, a, b) => plain(c) && block(a) && block(b),
        K::Block(b) => block(b),
        _ => false,
    }
}

/// Whether `block` can leave the function.
fn has_exit(block: &Block) -> bool {
    let mut returns = false;
    walk::blocks_mut(&mut block.clone(), &mut |b| {
        returns |= b.stmts.iter().any(|s| matches!(s, Stmt::Return(_)));
    });
    returns || walk::any_in_block(block, &mut |x| matches!(x.kind, K::Fail(_)))
}

/// Whether every `fail` in `body` is a statement, where drains can go
/// ahead of it.
fn exits_are_statements(body: &Block) -> bool {
    let mut all = 0;
    walk::any_in_block(body, &mut |x| {
        all += usize::from(matches!(x.kind, K::Fail(_)));
        false
    });
    let mut statements = 0;
    walk::blocks_mut(&mut body.clone(), &mut |b| {
        statements += b.stmts.iter().filter(|s| is_fail(s)).count();
    });
    all == statements
}

pub(super) fn is_fail(s: &Stmt) -> bool {
    matches!(
        s,
        Stmt::Expr(Expr {
            kind: K::Fail(_),
            ..
        })
    )
}

/// The line of the assignment to `total` in `b`.
pub(super) fn line_of(b: &Block, total: Local) -> Option<u32> {
    find(b, total).map(|e| e.line)
}

/// The assignment to `total` in `b`.
fn find(b: &Block, total: Local) -> Option<&Expr> {
    b.stmts.iter().find_map(|s| {
        let Stmt::Expr(e) = s else { return None };
        match &e.kind {
            K::Assign(l, _) if *l == total => Some(e),
            K::If(_, then, otherwise) => find(then, total).or_else(|| find(otherwise, total)),
            K::Block(inner) => find(inner, total),
            _ => None,
        }
    })
}
