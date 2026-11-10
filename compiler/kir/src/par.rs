//! Loops on all cores (decision 0031): an eligible counted loop is
//! outlined into a chunk function the runtime fans out over threads
//! (`kv_par_for`). Eligible means the body only does arithmetic, reads
//! lists and writes `out[i]` — stores into lists this function made and
//! never shared — so chunks touch disjoint memory and never allocate,
//! retain or release. The chunk is `quiet`: a failing check returns 1
//! instead of panicking. `kv_par_for` also returns 1 when a chunk
//! failed, when the loop is small or when one core is all there is; the
//! caller then runs the plain loop, so a panic keeps its exact round,
//! message and line. Only the LLVM backend fans out; Cranelift always
//! takes the plain loop.

use crate::ir::{Block, Expr, ExprKind, FuncId, Function, Local, Shape, Stmt, Type};
use crate::par::analyze::{eligible, reduction, unshare, unshare_stmt};
use crate::par::chunk::{call_site, make_chunk};
use crate::par::reduce::{make_red_chunk, red_call_site, stmt_line};
use crate::rt_op::RtOp;

mod analyze;
mod chunk;
mod reduce;

/// Loops with fewer rounds than this are left as they are; the runtime
/// checks the real bounds again.
const MIN_ROUNDS: i128 = 1 << 16;

/// Outlines eligible loops in every function; the functions appended
/// here (the chunks) are `quiet` from the returned index on.
pub(crate) fn outline(functions: &mut Vec<Function>, shapes: &mut Vec<Shape>) -> FuncId {
    if std::env::var_os("KAVACH_SERIAL").is_some() {
        return functions.len() as FuncId;
    }
    let quiet_from = functions.len();
    for f in 0..quiet_from {
        let mut chunks = Vec::new();
        let next = functions.len();
        let func = &mut functions[f];
        let mut fresh = Vec::new();
        let mut scratch = None;
        rewrite(
            &mut func.body,
            &mut func.locals,
            &func.name,
            &mut fresh,
            next,
            &mut chunks,
            shapes,
            &mut scratch,
        );
        // A reduction's slot lists, made once at the top (`SLOTS` each,
        // zero; the runtime re-zeroes them before every loop).
        if let Some(slots) = scratch {
            let ty = func.locals[slots[0] as usize];
            let line = func.line;
            for (i, l) in slots.into_iter().enumerate().rev() {
                let zeros = vec![
                    Expr {
                        kind: ExprKind::Int(0),
                        ty: Type::I64,
                        line,
                    };
                    SLOTS
                ];
                let list = Expr {
                    kind: ExprKind::List(zeros),
                    ty,
                    line,
                };
                let _ = i;
                func.body.stmts.insert(0, Stmt::Let(l, list));
            }
        }
        functions.append(&mut chunks);
    }
    quiet_from as FuncId
}

/// How many partial-sum slots a reduction keeps: the runtime never
/// makes more grabs than this (`runtime/native/src/rt_par.rs`).
pub(super) const SLOTS: usize = 256;

/// The three slot lists (partials, prefix maxes, prefix mins) of this
/// function's reductions, made on first use.
fn scratch_lists(
    locals: &mut Vec<Type>,
    shapes: &mut Vec<Shape>,
    scratch: &mut Option<[Local; 3]>,
) -> [Local; 3] {
    *scratch.get_or_insert_with(|| {
        let ty = Type::Ref(intern_int_list(shapes));
        let mut three = [0; 3];
        for slot in &mut three {
            *slot = locals.len() as Local;
            locals.push(ty);
        }
        three
    })
}

/// The `[Int]` heap type, found or added.
fn intern_int_list(shapes: &mut Vec<Shape>) -> u32 {
    let want = Shape::List(Type::I64);
    if let Some(at) = shapes.iter().position(|s| *s == want) {
        return at as u32;
    }
    shapes.push(want);
    (shapes.len() - 1) as u32
}

/// Walks `b`, keeping `fresh` (lists this function made and still owns
/// alone), and replaces eligible loops. `next` is the number of this
/// function's first chunk, so a new chunk is `next + chunks.len()`.
#[expect(clippy::too_many_arguments)]
fn rewrite(
    b: &mut Block,
    locals: &mut Vec<Type>,
    name: &str,
    fresh: &mut Vec<Local>,
    next: usize,
    chunks: &mut Vec<Function>,
    shapes: &mut Vec<Shape>,
    scratch: &mut Option<[Local; 3]>,
) {
    let log = std::env::var_os("KAVACH_PAR_LOG").is_some();
    let stmts = std::mem::take(&mut b.stmts);
    for mut s in stmts {
        match &mut s {
            Stmt::Let(l, e) => {
                if matches!(e.kind, ExprKind::List(_)) && e.ty.is_ref() {
                    fresh.push(*l);
                } else {
                    nested(e, locals, name, fresh, next, chunks, shapes, scratch);
                    unshare(e, fresh);
                }
            }
            Stmt::ForRange { body, .. } | Stmt::While { body, .. } => {
                // What the loop shares stays shared; then try the loop
                // itself (the eligible one is often nested in a counted
                // outer loop).
                for inner in &body.stmts {
                    unshare_stmt(inner, fresh);
                }
                if let Stmt::ForRange {
                    var,
                    start,
                    end,
                    inclusive: false,
                    body,
                } = &s
                    && big_enough(start, end)
                {
                    if let Some(plan) = eligible(body, *var, fresh, locals) {
                        let chunk = next + chunks.len();
                        let (flag_stmt, guard) = call_site(&s, &plan, chunk, locals, start, end);
                        chunks.push(make_chunk(&s, &plan, locals, name));
                        b.stmts.push(flag_stmt);
                        b.stmts.push(guard);
                        continue;
                    }
                    if let Some(plan) = reduction(body, &[*var], locals) {
                        let chunk = next + chunks.len();
                        let slots = scratch_lists(locals, shapes, scratch);
                        let stmts = red_call_site(
                            &s,
                            &plan,
                            chunk,
                            locals,
                            slots,
                            start.clone(),
                            end.clone(),
                        );
                        chunks.push(make_red_chunk(
                            &s,
                            &plan,
                            locals,
                            locals[slots[0] as usize],
                            name,
                        ));
                        b.stmts.extend(stmts);
                        continue;
                    }
                    if log {
                        eprintln!("[par] {name}: loop over l{var} not eligible");
                    }
                }
                if let Stmt::ForRange { body, .. } | Stmt::While { body, .. } = &mut s {
                    rewrite(body, locals, name, fresh, next, chunks, shapes, scratch);
                }
            }
            Stmt::ForEach {
                var,
                index,
                list,
                body,
            } => {
                let (var, index, list) = (*var, *index, *list);
                fresh.retain(|x| *x != list);
                for inner in &body.stmts {
                    unshare_stmt(inner, fresh);
                }
                let plan = reduction(body, &[var, index], locals).filter(|p| {
                    // The chunk reads `list[i]`: one more capture seat,
                    // and the items must be plain integers.
                    !p.caps.contains(&list) && p.caps.len() < 3 && locals[var as usize] == Type::I64
                });
                if let Some(mut plan) = plan {
                    plan.caps.insert(0, list);
                    let chunk = next + chunks.len();
                    let slots = scratch_lists(locals, shapes, scratch);
                    let line = body.stmts.first().map_or(0, stmt_line);
                    let start = Expr {
                        kind: ExprKind::Int(0),
                        ty: Type::I64,
                        line,
                    };
                    let end = Expr {
                        kind: ExprKind::Rt(
                            RtOp::ListLen,
                            vec![Expr {
                                kind: ExprKind::Local(list),
                                ty: locals[list as usize],
                                line,
                            }],
                        ),
                        ty: Type::I64,
                        line,
                    };
                    let stmts = red_call_site(&s, &plan, chunk, locals, slots, start, end);
                    chunks.push(make_red_chunk(
                        &s,
                        &plan,
                        locals,
                        locals[slots[0] as usize],
                        name,
                    ));
                    b.stmts.extend(stmts);
                    continue;
                }
                if let Stmt::ForEach { body, .. } = &mut s {
                    rewrite(body, locals, name, fresh, next, chunks, shapes, scratch);
                }
            }
            Stmt::Expr(e) => {
                nested(e, locals, name, fresh, next, chunks, shapes, scratch);
                unshare(e, fresh);
            }
            other => {
                // `print`, `return`, a call's result, a check: the
                // blocks of their expressions are tried too.
                crate::walk::stmt_parts(other, &mut |part| {
                    if let crate::walk::Part::Expr(x) = part {
                        nested(x, locals, name, fresh, next, chunks, shapes, scratch);
                    }
                });
                unshare_stmt(other, fresh);
            }
        }
        b.stmts.push(s);
    }
    if let Some(v) = &mut b.value {
        nested(v, locals, name, fresh, next, chunks, shapes, scratch);
        unshare(v, fresh);
    }
}

/// The loops in the blocks inside `e`, a statement's expression, at any
/// depth, tried as `rewrite` tries a block's: an inlined function's body
/// is a block, often an argument (`xs.push(work(s))`), and a loop
/// `version.rs` copied sits in an `if`. Each block is tried from what is
/// fresh before the statement; the caller then unshares `e` as a whole.
#[expect(clippy::too_many_arguments)]
fn nested(
    e: &mut Expr,
    locals: &mut Vec<Type>,
    name: &str,
    fresh: &[Local],
    next: usize,
    chunks: &mut Vec<Function>,
    shapes: &mut Vec<Shape>,
    scratch: &mut Option<[Local; 3]>,
) {
    crate::walk::parts(e, &mut |part| match part {
        crate::walk::Part::Expr(x) => nested(x, locals, name, fresh, next, chunks, shapes, scratch),
        crate::walk::Part::Block(b) => {
            let mut inside = fresh.to_vec();
            rewrite(b, locals, name, &mut inside, next, chunks, shapes, scratch);
        }
    });
}

/// Both bounds constant and far enough apart to be worth chunking?
/// Non-constant bounds are fine too: the runtime checks again.
fn big_enough(start: &Expr, end: &Expr) -> bool {
    match (&start.kind, &end.kind) {
        (ExprKind::Int(a), ExprKind::Int(b)) => b - a >= MIN_ROUNDS,
        (_, ExprKind::Int(b)) => *b >= MIN_ROUNDS,
        _ => matches!(end.kind, ExprKind::Local(_)),
    }
}
