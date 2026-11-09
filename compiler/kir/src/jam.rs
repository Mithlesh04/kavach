//! Unroll and jam: a loop over `j` whose body sums an inner loop into
//! accumulators,
//!
//! ```text
//! for j in 0..n { let cell = 0.0; for k in 0..m { cell += row[k] * b[k][j] }; total += cell }
//! ```
//!
//! computes eight values of `j` at once: one inner loop updates eight
//! accumulators, so each `b[k]` is looked up once for eight cells and the
//! eight independent sums can share vector instructions. Each sum still
//! adds its terms in the same order, so the results are exactly the same.
//!
//! Only safe loops are jammed: the inner loop may only do float arithmetic
//! on its accumulators and read lists by the loop variables, so it can't
//! panic or change anything else; the statements after it run in order
//! for each `j`. Before the loop, a check covers every index of a group
//! of eight (the shortest row is found once), so the inner loop reads
//! with `InRange` and checks nothing; a group that wouldn't pass, and the
//! last few values of `j`, run the original loop.

use crate::ir::*;
use crate::walk::{any_in_block, sets_local};

/// Values of `j` computed together.
const FACTOR: i64 = 8;

pub fn jam_loops(f: &mut Function) {
    let mut locals = std::mem::take(&mut f.locals);
    jam_block(&mut f.body, &mut locals);
    f.locals = locals;
}

fn jam_block(b: &mut Block, locals: &mut Vec<Type>) {
    for s in std::mem::take(&mut b.stmts) {
        match s {
            Stmt::ForRange {
                var,
                start,
                end,
                inclusive: false,
                body,
            } if plan(var, &body, locals).is_some() => {
                let p = plan(var, &body, locals).expect("a plan");
                b.stmts.extend(jammed(var, start, end, body, p, locals));
            }
            mut s => {
                if let Stmt::While { body, .. }
                | Stmt::ForRange { body, .. }
                | Stmt::ForEach { body, .. } = &mut s
                {
                    jam_block(body, locals);
                }
                b.stmts.push(s);
            }
        }
    }
}

/// What a jammable loop over `j` is made of.
struct Plan {
    /// The `let c = …` before the inner loop.
    accs: Vec<Local>,
    /// The inner loop's position in the body.
    inner: usize,
    /// Lists read as `l[k]` (or `l[k][j]`), as `l[k][j]` (with the type
    /// of a row), and as `l[j]`.
    by_k: Vec<Local>,
    rows: Vec<(Local, Type)>,
    by_j: Vec<Local>,
}

fn plan(j: Local, body: &Block, locals: &[Type]) -> Option<Plan> {
    if locals[j as usize] != Type::I64 || body.value.is_some() {
        return None;
    }
    let is_loop = |s: &Stmt| {
        matches!(
            s,
            Stmt::While { .. } | Stmt::ForRange { .. } | Stmt::ForEach { .. }
        )
    };
    let loops: Vec<usize> = (0..body.stmts.len())
        .filter(|i| is_loop(&body.stmts[*i]))
        .collect();
    let [inner] = loops[..] else { return None };
    let Stmt::ForRange {
        var: k,
        start: ks,
        end: ke,
        inclusive: false,
        body: kb,
    } = &body.stmts[inner]
    else {
        return None;
    };
    // `let c = <constant>` before it.
    let mut accs = Vec::new();
    for s in &body.stmts[..inner] {
        match s {
            Stmt::Let(c, e) if matches!(e.kind, ExprKind::Int(_) | ExprKind::Float(_)) => {
                accs.push(*c)
            }
            _ => return None,
        }
    }
    let simple = |e: &Expr| match e.kind {
        ExprKind::Int(_) => true,
        ExprKind::Local(x) => x != j && locals[x as usize] == Type::I64 && !sets_local(body, x),
        _ => false,
    };
    if accs.is_empty() || locals[*k as usize] != Type::I64 || !simple(ks) || !simple(ke) {
        return None;
    }
    // The inner loop only updates accumulators, with safe expressions.
    let mut p = Plan {
        accs,
        inner,
        by_k: Vec::new(),
        rows: Vec::new(),
        by_j: Vec::new(),
    };
    if kb.value.is_some()
        || any_in_block(kb, &mut |e| {
            matches!(e.kind, ExprKind::If(..) | ExprKind::Block(_))
        })
    {
        return None;
    }
    for s in &kb.stmts {
        let Stmt::Expr(Expr {
            kind: ExprKind::Assign(c, v),
            ..
        }) = s
        else {
            return None;
        };
        if !p.accs.contains(c) || !safe(v, j, *k, body, locals, &mut p) {
            return None;
        }
    }
    if p.rows.is_empty() && p.by_j.is_empty() {
        return None; // nothing depends on `j`: no gain
    }
    // After it: no early exits, and nothing the jammed part reads changes.
    let after = Block {
        stmts: body.stmts[inner + 1..].to_vec(),
        value: None,
    };
    let exits = after
        .stmts
        .iter()
        .any(|s| matches!(s, Stmt::Break | Stmt::Continue | Stmt::Return(_)))
        || any_in_block(&after, &mut |e| match &e.kind {
            ExprKind::If(_, t, o) => has_exit(t) || has_exit(o),
            ExprKind::Block(b) => has_exit(b),
            _ => matches!(e.kind, ExprKind::Fail(_)),
        });
    let read: Vec<Local> = p
        .accs
        .iter()
        .chain(&p.by_k)
        .chain(&p.by_j)
        .copied()
        .chain([j, *k])
        .collect();
    if exits || read.iter().any(|l| sets_local(&after, *l)) {
        return None;
    }
    Some(p)
}

fn has_exit(b: &Block) -> bool {
    b.stmts
        .iter()
        .any(|s| matches!(s, Stmt::Break | Stmt::Continue | Stmt::Return(_)))
}

/// Whether `e` can be computed for eight values of `j` together: float
/// arithmetic (which can't fail), accumulators, loop variables, unchanged
/// number locals, and list items by `k` or `j`.
fn safe(e: &Expr, j: Local, k: Local, body: &Block, locals: &[Type], p: &mut Plan) -> bool {
    let local = |e: &Expr| match e.kind {
        ExprKind::Local(l) => Some(l),
        _ => None,
    };
    let unchanged = |l: Local| !sets_local(body, l);
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Float(_) => true,
        ExprKind::Local(l) => {
            p.accs.contains(l)
                || *l == j
                || *l == k
                || (!locals[*l as usize].is_ref() && unchanged(*l))
        }
        ExprKind::Binary(_, a, b) if a.ty.is_float() => {
            safe(a, j, k, body, locals, p) && safe(b, j, k, body, locals, p)
        }
        ExprKind::Unary(UnOp::Neg, x) if x.ty.is_float() => safe(x, j, k, body, locals, p),
        // Integer to float never fails.
        ExprKind::Convert(x, _) if e.ty.is_float() && x.ty.is_int() => {
            safe(x, j, k, body, locals, p)
        }
        ExprKind::Index(base, i) if !e.ty.is_ref() && e.ty != Type::Unit => {
            let (Some(i), list) = (local(i), local(base)) else {
                return false;
            };
            match (list, &base.kind) {
                (Some(l), _) if i == k && unchanged(l) => push(&mut p.by_k, l),
                (Some(l), _) if i == j && unchanged(l) => push(&mut p.by_j, l),
                (None, ExprKind::Index(outer, ki)) if i == j && local(ki) == Some(k) => {
                    match local(outer) {
                        Some(l) if unchanged(l) => {
                            if !p.rows.iter().any(|(r, _)| *r == l) {
                                p.rows.push((l, base.ty));
                            }
                            push(&mut p.by_k, l)
                        }
                        _ => false,
                    }
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn push(list: &mut Vec<Local>, l: Local) -> bool {
    if !list.contains(&l) {
        list.push(l);
    }
    true
}

mod build;
use build::jammed;

#[cfg(test)]
mod tests;
