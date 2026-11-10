//! Accumulators in a counted loop: `total += x` in `for i in 0..n`.
//!
//! The fixed point widens a local that grows every round to its type's
//! limits, so `total + x` keeps its overflow check. But a loop that runs at
//! most `n` rounds, and adds at most `d` a round, keeps `total` within its
//! start plus `n × d`. When that range fits the type, no add can overflow,
//! and spec 5.6 still holds: the check that is left out could never fail.
//!
//! The local qualifies only when it is safe to reason so: every set of the
//! local inside the loop is `t = t + d` or `t = t - d`, with `d` a constant
//! or a local that isn't `t`, and none is inside a loop within it; and
//! besides those, the local is set once, by a `let` before the loop in the
//! same block. So each time the block runs, `t` starts again from the
//! `let`, even when the block is itself inside another loop.

use super::interval::Range;
use crate::ir::*;
use crate::walk;

/// What one round may add: `t = t + d` (or `- d`), with `d`.
fn step(l: Local, value: &Expr) -> Option<(bool, &Expr)> {
    let ExprKind::Binary(op @ (BinOp::Add | BinOp::Sub), a, b) = &value.kind else {
        return None;
    };
    let is_l = |e: &Expr| matches!(e.kind, ExprKind::Local(x) | ExprKind::Move(x) if x == l);
    let reads_l = |e: &Expr| walk::any_expr(e, &mut |x| is_l(x));
    match (op, is_l(a), is_l(b)) {
        (_, true, false) if !reads_l(b) => Some((*op == BinOp::Sub, b)),
        (BinOp::Add, false, true) if !reads_l(a) => Some((false, a)),
        _ => None,
    }
}

/// The range of a constant, a local, or a sum, difference, product or
/// mask of those, or a remainder by a constant, from the ranges so far:
/// `q.x + q.z` in `total += q.x + q.z`. `None` when it may leave its type,
/// or for anything else.
fn simple(e: &Expr, ranges: &[Option<Range>]) -> Option<Range> {
    let r = match &e.kind {
        ExprKind::Int(n) => Range { lo: *n, hi: *n },
        ExprKind::Local(x) | ExprKind::Move(x) => ranges.get(*x as usize).copied().flatten()?,
        // `a % c` (a constant `c > 0`) is within `c` of 0, whatever `a` is:
        // `i * k % 1000` with `k` unknown.
        ExprKind::Binary(BinOp::Rem, a, c) => {
            let ExprKind::Int(c) = c.kind else {
                return None;
            };
            if c <= 0 {
                return None;
            }
            let lo = match simple(a, ranges) {
                Some(a) if a.lo >= 0 => 0,
                _ => -(c - 1),
            };
            Range { lo, hi: c - 1 }
        }
        ExprKind::Binary(op, a, b) => {
            let (a, b) = (simple(a, ranges)?, simple(b, ranges)?);
            let corners = |f: fn(i128, i128) -> Option<i128>| {
                let c = [
                    f(a.lo, b.lo)?,
                    f(a.lo, b.hi)?,
                    f(a.hi, b.lo)?,
                    f(a.hi, b.hi)?,
                ];
                Some(Range {
                    lo: *c.iter().min()?,
                    hi: *c.iter().max()?,
                })
            };
            match op {
                BinOp::Add => corners(i128::checked_add)?,
                BinOp::Sub => corners(i128::checked_sub)?,
                BinOp::Mul => corners(i128::checked_mul)?,
                BinOp::BitAnd if b.lo >= 0 => Range { lo: 0, hi: b.hi },
                _ => return None,
            }
        }
        _ => return None,
    };
    r.within(e.ty).then_some(r)
}

/// Every `Assign` to `l` in `b`, with what it assigns.
fn assigns(b: &mut Block, l: Local) -> Vec<Expr> {
    let mut found = Vec::new();
    walk::exprs_mut(b, &mut |e| {
        if let ExprKind::Assign(x, value) = &e.kind
            && *x == l
        {
            found.push((**value).clone());
        }
    });
    found
}

/// Whether `l` is set in `b` other than by an `Assign`: by a `let`
/// (counted in `lets`), a loop's own variable or a call's result; and,
/// with `nested`, by anything in a loop inside `b`.
fn set_otherwise(b: &mut Block, l: Local, nested: bool, lets: &mut usize) -> bool {
    let mut bad = false;
    walk::blocks_mut(b, &mut |inner| {
        for s in &inner.stmts {
            match s {
                Stmt::While { body, .. } | Stmt::ForEach { body, .. } => {
                    bad |= nested && walk::sets_local(body, l);
                }
                Stmt::ForRange { var, body, .. } => {
                    bad |= *var == l || (nested && walk::sets_local(body, l));
                }
                Stmt::LetResult { value, error, .. } => bad |= *value == l || *error == l,
                Stmt::Let(x, _) if *x == l => *lets += 1,
                _ => {}
            }
        }
    });
    bad
}

/// A counted loop, with the `let`s before it in its block.
struct Counted {
    var: Local,
    inclusive: bool,
    body: Block,
    lets: Vec<(Local, Expr)>,
}

/// Every counted loop in `b`, at any depth.
fn counted_loops(b: &mut Block) -> Vec<Counted> {
    let mut found = Vec::new();
    walk::blocks_mut(b, &mut |inner| {
        let mut lets = Vec::new();
        for s in &inner.stmts {
            match s {
                Stmt::Let(l, value) => lets.push((*l, value.clone())),
                // A loop `version.rs` copied: one of the two runs, so both
                // bodies' steps together bound what a round adds.
                Stmt::Expr(Expr {
                    kind: ExprKind::If(_, fast, written),
                    ..
                }) => {
                    if let (Some(a), Some(b)) = (only_loop(fast), only_loop(written))
                        && a.var == b.var
                        && a.inclusive == b.inclusive
                    {
                        let mut body = a.body;
                        body.stmts.extend(b.body.stmts);
                        found.push(Counted {
                            body,
                            lets: lets.clone(),
                            ..a
                        });
                    }
                }
                Stmt::ForRange {
                    var,
                    inclusive,
                    body,
                    ..
                } => found.push(Counted {
                    var: *var,
                    inclusive: *inclusive,
                    body: body.clone(),
                    lets: lets.clone(),
                }),
                _ => {}
            }
        }
    });
    found
}

/// The loop that is all of block `b`.
fn only_loop(b: &Block) -> Option<Counted> {
    match (&b.stmts[..], &b.value) {
        (
            [
                Stmt::ForRange {
                    var,
                    inclusive,
                    body,
                    ..
                },
            ],
            None,
        ) => Some(Counted {
            var: *var,
            inclusive: *inclusive,
            body: body.clone(),
            lets: Vec::new(),
        }),
        _ => None,
    }
}

/// The ranges the accumulators of `f`'s counted loops stay in, worked out
/// from `ranges`, the pass's sound ranges so far, and `bounds`, the ranges
/// of each loop's start and end where it starts.
pub(super) fn caps(
    f: &mut Function,
    ranges: &[Option<Range>],
    bounds: &[Option<(Range, Range)>],
) -> Vec<(Local, Range)> {
    let mut caps = Vec::new();
    for Counted {
        var,
        inclusive,
        mut body,
        lets,
    } in counted_loops(&mut f.body)
    {
        let Some((s, e)) = bounds.get(var as usize).copied().flatten() else {
            continue;
        };
        let rounds = (e.hi - s.lo + i128::from(inclusive)).max(0);
        let candidates: Vec<Local> = {
            let mut ls = Vec::new();
            walk::exprs_mut(&mut body, &mut |e| {
                if let ExprKind::Assign(x, _) = e.kind
                    && !ls.contains(&x)
                {
                    ls.push(x);
                }
            });
            ls
        };
        for l in candidates {
            if !f.locals[l as usize].is_int() {
                continue;
            }
            let inside = assigns(&mut body, l);
            // Every set of `l` in the function is one of these, and its one
            // `let` is before the loop in its block.
            let (mut in_loop, mut all) = (0, 0);
            if assigns(&mut f.body, l).len() != inside.len()
                || set_otherwise(&mut body, l, true, &mut in_loop)
                || set_otherwise(&mut f.body, l, false, &mut all)
                || in_loop != 0
                || all != 1
            {
                continue;
            }
            let mine: Vec<&Expr> = (lets.iter())
                .filter_map(|(x, value)| (*x == l).then_some(value))
                .collect();
            let [init] = mine[..] else { continue };
            let Some(start) = simple(init, ranges) else {
                continue;
            };
            let (mut up, mut down) = (0i128, 0i128);
            let mut ok = true;
            for value in &inside {
                match step(l, value).and_then(|(minus, d)| Some((minus, simple(d, ranges)?))) {
                    Some((false, d)) => {
                        up += d.hi.max(0);
                        down += d.lo.min(0);
                    }
                    Some((true, d)) => {
                        up += (-d.lo).max(0);
                        down += (-d.hi).min(0);
                    }
                    None => ok = false,
                }
            }
            let cap = (ok)
                .then(|| {
                    let lo = start.lo.checked_add(rounds.checked_mul(down)?)?;
                    let hi = start.hi.checked_add(rounds.checked_mul(up)?)?;
                    Some(Range { lo, hi })
                })
                .flatten();
            if let Some(cap) = cap.filter(|c| c.within(f.locals[l as usize])) {
                caps.push((l, cap));
            }
        }
    }
    caps
}

#[cfg(test)]
mod tests;
