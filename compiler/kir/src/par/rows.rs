//! A grid's rows on all cores (spec 4.9): `out[i * w + j]` in an inner
//! `for j in 0..w`, which `analyze.rs` lets a loop's rounds write, as each
//! round's row is its own.

use crate::ir::{BinOp, Block, Expr, ExprKind, FastOp, Local, Stmt, Type};
use crate::par::analyze::{capture, store_ok, value_ok};
use crate::rt_op::RtOp;

/// The lists `b` writes, at any depth: in it, its loops and its `if`s.
pub(super) fn stores(b: &Block, outs: &mut Vec<Local>) {
    for s in &b.stmts {
        match s {
            Stmt::Expr(e) => match &e.kind {
                ExprKind::Assign(out, v) if matches!(v.kind, ExprKind::Rt(RtOp::ListSet, _)) => {
                    outs.push(*out);
                }
                ExprKind::If(_, a, b) => {
                    stores(a, outs);
                    stores(b, outs);
                }
                _ => {}
            },
            Stmt::ForRange { body, .. } => stores(body, outs),
            _ => {}
        }
    }
}

/// `for j in 0..w` writing a grid's rows, `out[i * w + j]`, with `i` the
/// outer loop's `var`: its `w`, once its lets and stores check out.
pub(super) fn row_loop(
    s: &Stmt,
    var: Local,
    temps: &mut Vec<Local>,
    outs: &[Local],
    fresh: &[Local],
    locals: &[Type],
    caps: &mut Vec<Local>,
) -> Option<Expr> {
    let Stmt::ForRange {
        var: j,
        start,
        end,
        inclusive: false,
        body,
    } = s
    else {
        return None;
    };
    if !matches!(start.kind, ExprKind::Int(0)) || body.value.is_some() {
        return None;
    }
    match &end.kind {
        ExprKind::Int(n) if *n >= 0 => {}
        ExprKind::Local(w) if locals[*w as usize] == Type::I64 => capture(*w, locals, caps)?,
        _ => return None,
    }
    temps.push(*j);
    for s in &body.stmts {
        match s {
            Stmt::Let(l, e) => {
                if e.ty.is_ref() || !value_ok(e, temps, outs, fresh, locals, caps) {
                    return None;
                }
                temps.push(*l);
            }
            Stmt::Expr(e) => {
                let is_row = |i: &Expr| row_index(i, var, *j, end);
                store_ok(e, is_row, (temps, outs, fresh, locals), caps)?;
            }
            _ => return None,
        }
    }
    Some(end.clone())
}

/// `w` when it is the width seen before, or the first.
pub(super) fn same_width(before: Option<Expr>, w: Expr) -> Option<Expr> {
    let same = match (before.as_ref().map(|b| &b.kind), &w.kind) {
        (None, _) => true,
        (Some(ExprKind::Int(a)), ExprKind::Int(b)) => a == b,
        (Some(ExprKind::Local(a)), ExprKind::Local(b)) => a == b,
        _ => false,
    };
    same.then_some(w)
}

/// `i * w + j`, the parts in any order, with `w` the inner loop's end.
fn row_index(index: &Expr, i: Local, j: Local, w: &Expr) -> bool {
    let is = |e: &Expr, l: Local| matches!(e.kind, ExprKind::Local(x) if x == l);
    let same_w = |e: &Expr| match (&e.kind, &w.kind) {
        (ExprKind::Int(a), ExprKind::Int(b)) => a == b,
        (ExprKind::Local(a), ExprKind::Local(b)) => a == b,
        _ => false,
    };
    let product = |e: &Expr| match &e.kind {
        ExprKind::Binary(BinOp::Mul, x, y) | ExprKind::Fast(FastOp::Mul, x, y) => {
            (is(x, i) && same_w(y)) || (is(y, i) && same_w(x))
        }
        _ => false,
    };
    match &index.kind {
        ExprKind::Binary(BinOp::Add, a, b) | ExprKind::Fast(FastOp::Add, a, b) => {
            (product(a) && is(b, j)) || (product(b) && is(a, j))
        }
        _ => false,
    }
}
