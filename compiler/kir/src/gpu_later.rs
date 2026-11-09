//! Sums a loop doesn't wait for (spec 15.2, decision 0067). In
//!
//! ```text
//! for chunk in fs.chunks(path, size) {
//!     total += Int(chunk.as_list<Int32>().map(f).sum(g))
//! }
//! ```
//!
//! a sum that waits for the GPU each round stops the disk and the copies
//! too. When the loop reads `total` nowhere else, nothing needs the sum
//! yet: the kernel goes into the GPU's queue, and the statement runs later,
//! with the value it would have had, in the same order. So each round
//! becomes
//!
//! ```text
//! let held = <the list>
//! GpuSumLater(held, closure, kernel)
//! while GpuLaterNext(q, false) { <the statement> }
//! ```
//!
//! and the same `while`, told to wait for all, runs after the loop and
//! before anything that leaves the function from inside it.

use crate::gpu_stay;
use crate::ir::{Block, Expr, ExprKind as K, Function, Local, Stmt, Type};
use crate::rt_op::RtOp;
use crate::walk::{self, Part};
use std::collections::{BTreeMap, HashMap, HashSet};

mod fits;
use fits::{accumulators, fits, is_fail, plain, reduce_of};

/// Lines of loops that wait for the GPU each round, and why: what
/// `kavach check` notes (decision 0067).
pub type Notes = BTreeMap<u32, &'static str>;

pub(crate) fn overlap_sums(f: &mut Function, notes: &mut Notes) {
    let on_gpu = gpu_stay::set_by_gpu(f);
    let any_gpu = |e: &Expr| {
        reduce_of(e).is_some() || matches!(e.kind, K::Rt(RtOp::GpuFetch | RtOp::GpuMap, _))
    };
    if !walk::any_in_block(&f.body, &mut { any_gpu }) {
        return;
    }
    let mut locals = std::mem::take(&mut f.locals);
    in_block(&mut f.body, &on_gpu, &mut locals, notes);
    f.locals = locals;
}

/// Each loop in `b`, at any depth, the inner ones first.
fn in_block(b: &mut Block, on_gpu: &HashSet<Local>, locals: &mut Vec<Type>, notes: &mut Notes) {
    let mut i = 0;
    while i < b.stmts.len() {
        walk::stmt_parts(&mut b.stmts[i], &mut |p| in_part(p, on_gpu, locals, notes));
        let (before, after) = match &mut b.stmts[i] {
            Stmt::While { body, .. } | Stmt::ForRange { body, .. } | Stmt::ForEach { body, .. } => {
                in_loop(body, on_gpu, locals, notes)
            }
            _ => (Vec::new(), Vec::new()),
        };
        let (ahead, behind) = (before.len(), after.len());
        b.stmts.splice(i + 1..i + 1, after);
        b.stmts.splice(i..i, before);
        i += ahead + behind + 1;
    }
    if let Some(v) = &mut b.value {
        in_part(Part::Expr(v), on_gpu, locals, notes);
    }
}

fn in_part(p: Part<'_>, on_gpu: &HashSet<Local>, locals: &mut Vec<Type>, notes: &mut Notes) {
    match p {
        Part::Block(b) => in_block(b, on_gpu, locals, notes),
        Part::Expr(e) => walk::parts(e, &mut |p| in_part(p, on_gpu, locals, notes)),
    }
}

/// The statements that go before and after the loop of `body`.
fn in_loop(
    body: &mut Block,
    on_gpu: &HashSet<Local>,
    locals: &mut Vec<Type>,
    notes: &mut Notes,
) -> (Vec<Stmt>, Vec<Stmt>) {
    join_kept_sums(body);
    let (mut before, mut after) = (Vec::new(), Vec::new());
    let mut totals = Vec::new();
    accumulators(body, &mut totals);
    for total in totals {
        if let Err(why) = fits(body, total, on_gpu) {
            if let (Some(why), Some(line)) = (why, fits::line_of(body, total)) {
                notes.entry(line).or_insert(why);
            }
            continue;
        }
        let q = locals.len() as Local;
        locals.push(Type::I64);
        let Some(template) = rewrite(body, total, q, locals) else {
            continue;
        };
        let at = |kind, ty| Expr { kind, ty, line: 0 };
        before.push(Stmt::Let(
            q,
            at(K::Rt(RtOp::GpuLaterNew, Vec::new()), Type::I64),
        ));
        leave_after_draining(body, &template, q, locals);
        after.push(draining(&template, q, true, locals));
    }
    // What is left of the GPU's work in the loop makes each round wait.
    walk::any_in_block(body, &mut |e| {
        let why = match &e.kind {
            K::Block(b) if reduce_of(e).is_some() => {
                let list_on_gpu = matches!(b.stmts.first(), Some(Stmt::Let(_, list))
                    if gpu_stay::is_gpu_list(list)
                        || matches!(list.kind, K::Local(l) if on_gpu.contains(&l)));
                list_on_gpu.then_some("the round uses the sum at once")
            }
            K::Rt(RtOp::GpuFetch | RtOp::GpuMap, _) => Some("the CPU reads the result each round"),
            _ => None,
        };
        if let Some(why) = why {
            notes.entry(e.line).or_insert(why);
        }
        false
    });
    (before, after)
}

/// `const s = xs.map(f).sum(g)`, then `total += s`, used nowhere else:
/// as if written in one statement.
fn join_kept_sums(body: &mut Block) {
    let mentions = mention_counts(body);
    walk::blocks_mut(body, &mut |b| {
        let mut i = 0;
        while i + 1 < b.stmts.len() {
            let (first, rest) = b.stmts.split_at_mut(i + 1);
            let joined = match (&first[i], &mut rest[0]) {
                (
                    Stmt::Let(s, sum),
                    Stmt::Expr(Expr {
                        kind: K::Assign(_, value),
                        ..
                    }),
                ) if reduce_of(sum).is_some() && mentions.get(s) == Some(&2) && plain(value) => {
                    put_in(value, *s, sum)
                }
                _ => false,
            };
            if joined {
                b.stmts.remove(i);
            } else {
                i += 1;
            }
        }
    });
}

/// Replaces the one read of local `s` in `e`, which `plain` allows, with
/// `value`. Numbers worked out either side of it change nothing by that.
fn put_in(e: &mut Expr, s: Local, value: &Expr) -> bool {
    match &mut e.kind {
        K::Local(l) if *l == s => {
            *e = value.clone();
            true
        }
        K::Convert(x, _) | K::Unary(_, x) | K::Some(x) | K::Unwrap(x) => put_in(x, s, value),
        K::Binary(_, a, b) | K::Fast(_, a, b) => put_in(b, s, value) || put_in(a, s, value),
        _ => false,
    }
}

/// How many times each local is mentioned in `body`.
fn mention_counts(body: &Block) -> HashMap<Local, usize> {
    let mut counts = HashMap::new();
    walk::locals_mut(&mut body.clone(), &mut |l| {
        *counts.entry(*l).or_insert(0) += 1;
    });
    counts
}

fn one_statement(e: &Expr) -> Block {
    Block {
        stmts: vec![Stmt::Expr(e.clone())],
        value: None,
    }
}

/// The later form of a reduction's op.
fn later_op(op: RtOp) -> RtOp {
    match op {
        RtOp::GpuSum => RtOp::GpuSumLater,
        RtOp::GpuMin => RtOp::GpuMinLater,
        _ => RtOp::GpuMaxLater,
    }
}

/// What stays in the loop of a reduction.
struct Round {
    held: Local,
    list: Expr,
    value: Local,
    op: RtOp,
    args: Vec<Expr>,
    ty: Type,
}

/// Takes the list and the GPU op out of the reduction in `work`, leaving
/// only its choice between the GPU's value and the CPU's.
fn take_out_list(work: &mut Expr) -> Option<Round> {
    let placeholder = Expr {
        kind: K::Int(0),
        ty: work.ty,
        line: work.line,
    };
    let mut block = one_statement(&std::mem::replace(work, placeholder));
    let mut round = None;
    walk::exprs_mut(&mut block, &mut |e| {
        if reduce_of(e).is_none() {
            return;
        }
        let K::Block(b) = &mut e.kind else { return };
        let mut parts = b.stmts.drain(..);
        if let (Some(Stmt::Let(held, list)), Some(Stmt::Let(value, gpu))) =
            (parts.next(), parts.next())
            && let K::Rt(op, args) = gpu.kind
        {
            round = Some(Round {
                held,
                list,
                value,
                op,
                args,
                ty: gpu.ty,
            });
        }
    });
    if let Some(Stmt::Expr(statement)) = block.stmts.pop() {
        *work = statement;
    }
    round
}

/// Puts the round's part in place of the statement that sets `total`, and
/// gives what a `while` runs for each sum that is ready: the statement,
/// reading its sum and its list from the runtime.
fn rewrite(body: &mut Block, total: Local, q: Local, locals: &mut Vec<Type>) -> Option<Block> {
    let mut template = None;
    walk::blocks_mut(body, &mut |b| {
        let found = b.stmts.iter().position(
            |s| matches!(s, Stmt::Expr(Expr { kind: K::Assign(l, _), .. }) if *l == total),
        );
        let Some(i) = found else { return };
        let Stmt::Expr(statement) = &mut b.stmts[i] else {
            return;
        };
        let line = statement.line;
        let Some(sum) = take_out_list(statement) else {
            return;
        };
        let Stmt::Expr(statement) = b.stmts.remove(i) else {
            return;
        };
        let list_ty = sum.list.ty;
        let at = |kind, ty| Expr { kind, ty, line };
        let later = at(K::Rt(later_op(sum.op), sum.args), sum.ty);
        let round = [
            Stmt::Let(sum.held, sum.list),
            Stmt::Expr(later),
            Stmt::While {
                cond: next_of(q, false, line),
                body: Block::default(),
            },
        ];
        b.stmts.splice(i..i, round);
        template = Some(Block {
            stmts: vec![
                Stmt::Let(
                    sum.value,
                    at(K::Rt(RtOp::GpuLaterValue, Vec::new()), sum.ty),
                ),
                Stmt::Let(sum.held, at(K::Rt(RtOp::GpuLaterList, Vec::new()), list_ty)),
                Stmt::Expr(statement),
            ],
            value: None,
        });
    });
    let template = template?;
    // The `while` of the round, now that the template is known.
    let copy = fresh(&template, locals);
    walk::blocks_mut(body, &mut |b| {
        for s in &mut b.stmts {
            if let Stmt::While { cond, body } = s
                && is_next_of(cond, q)
                && body.stmts.is_empty()
            {
                *body = copy.clone();
            }
        }
    });
    Some(template)
}

fn next_of(q: Local, all: bool, line: u32) -> Expr {
    let at = |kind, ty| Expr { kind, ty, line };
    at(
        K::Rt(
            RtOp::GpuLaterNext,
            vec![
                at(K::Local(q), Type::I64),
                at(K::Int(i128::from(all)), Type::I64),
            ],
        ),
        Type::Bool,
    )
}

fn is_next_of(e: &Expr, q: Local) -> bool {
    matches!(&e.kind, K::Rt(RtOp::GpuLaterNext, args)
        if matches!(args.first().map(|a| &a.kind), Some(K::Local(l)) if *l == q))
}

/// A `while` that runs the statement for each sum ready (or for all).
fn draining(template: &Block, q: Local, all: bool, locals: &mut Vec<Type>) -> Stmt {
    Stmt::While {
        cond: next_of(q, all, 0),
        body: fresh(template, locals),
    }
}

/// Before anything in the loop that leaves the function, every sum still
/// on its way: an overflow it finds comes first, as it would have.
fn leave_after_draining(body: &mut Block, template: &Block, q: Local, locals: &mut Vec<Type>) {
    walk::blocks_mut(body, &mut |b| {
        let mut i = 0;
        while i < b.stmts.len() {
            if matches!(b.stmts[i], Stmt::Return(_)) || is_fail(&b.stmts[i]) {
                b.stmts.insert(i, draining(template, q, true, locals));
                i += 1;
            }
            i += 1;
        }
    });
}

/// A copy of `template` with locals of its own.
fn fresh(template: &Block, locals: &mut Vec<Type>) -> Block {
    let mut copy = template.clone();
    let renamed: HashMap<Local, Local> = defined_in(template)
        .into_iter()
        .map(|l| {
            let new = locals.len() as Local;
            locals.push(locals[l as usize]);
            (l, new)
        })
        .collect();
    walk::locals_mut(&mut copy, &mut |l| {
        if let Some(new) = renamed.get(l) {
            *l = *new;
        }
    });
    copy
}

/// The locals `b` sets for itself: by `let`, or as a loop's.
fn defined_in(b: &Block) -> HashSet<Local> {
    let mut defined = HashSet::new();
    walk::blocks_mut(&mut b.clone(), &mut |inner| {
        for s in &inner.stmts {
            match s {
                Stmt::Let(l, _) => {
                    defined.insert(*l);
                }
                Stmt::LetResult { value, error, .. } => {
                    defined.insert(*value);
                    defined.insert(*error);
                }
                Stmt::ForRange { var, .. } => {
                    defined.insert(*var);
                }
                Stmt::ForEach { var, index, .. } => {
                    defined.insert(*var);
                    defined.insert(*index);
                }
                _ => {}
            }
        }
    });
    defined
}

#[cfg(test)]
mod tests;
