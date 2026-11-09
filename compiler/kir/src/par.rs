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

use crate::ir::{BinOp, Block, Expr, ExprKind, FuncId, Function, Local, Shape, Stmt, Type};
use crate::par::analyze::{Plan, eligible, reduction, unshare, unshare_stmt};
use crate::par::reduce::{make_red_chunk, red_call_site, stmt_line};
use crate::rt_op::RtOp;

mod analyze;
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
/// alone), and replaces eligible loops. `next` numbers the next chunk.
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
                    rewrite(
                        body,
                        locals,
                        name,
                        fresh,
                        next + chunks.len(),
                        chunks,
                        shapes,
                        scratch,
                    );
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
                    rewrite(
                        body,
                        locals,
                        name,
                        fresh,
                        next + chunks.len(),
                        chunks,
                        shapes,
                        scratch,
                    );
                }
            }
            Stmt::Expr(e) => unshare(e, fresh),
            other => unshare_stmt(other, fresh),
        }
        b.stmts.push(s);
    }
    if let Some(v) = &b.value {
        unshare(v, fresh);
    }
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

/// The replacement at the call site: `flag = kv_par_for(…)`, then the
/// plain loop under `if flag != 0`.
fn call_site(
    loop_stmt: &Stmt,
    plan: &Plan,
    chunk: usize,
    locals: &mut Vec<Type>,
    start: &Expr,
    end: &Expr,
) -> (Stmt, Stmt) {
    let line = start.line;
    let flag = locals.len() as Local;
    locals.push(Type::I64);
    let mut args = vec![
        start.clone(),
        end.clone(),
        Expr {
            kind: ExprKind::Int(chunk as i128),
            ty: Type::I64,
            line,
        },
    ];
    for c in &plan.caps {
        args.push(Expr {
            kind: ExprKind::Local(*c),
            ty: locals[*c as usize],
            line,
        });
    }
    let call = Expr {
        kind: ExprKind::Rt(RtOp::ParFor, args),
        ty: Type::I64,
        line,
    };
    let cond = Expr {
        kind: ExprKind::Binary(
            BinOp::Ne,
            Box::new(Expr {
                kind: ExprKind::Local(flag),
                ty: Type::I64,
                line,
            }),
            Box::new(Expr {
                kind: ExprKind::Int(0),
                ty: Type::I64,
                line,
            }),
        ),
        ty: Type::Bool,
        line,
    };
    let then = Block {
        stmts: vec![loop_stmt.clone()],
        value: None,
    };
    let guard = Expr {
        kind: ExprKind::If(Box::new(cond), then, Block::default()),
        ty: Type::Unit,
        line,
    };
    (Stmt::Let(flag, call), Stmt::Expr(guard))
}

/// The chunk: `fn(lo, hi, caps…) -> 0`, the loop body over `lo..hi` with
/// every local renumbered.
fn make_chunk(loop_stmt: &Stmt, plan: &Plan, locals: &[Type], name: &str) -> Function {
    let Stmt::ForRange {
        var, body, start, ..
    } = loop_stmt
    else {
        unreachable!("a counted loop");
    };
    let line = start.line;
    let mut map: Vec<(Local, Local)> = Vec::new();
    let mut chunk_locals = vec![Type::I64, Type::I64];
    for (k, c) in plan.caps.iter().enumerate() {
        map.push((*c, (k + 2) as Local));
        chunk_locals.push(locals[*c as usize]);
    }
    let new_var = chunk_locals.len() as Local;
    chunk_locals.push(Type::I64);
    map.push((*var, new_var));
    let mut body = body.clone();
    remap_block(&mut body, &mut map, &mut chunk_locals, locals);
    let at = |kind, ty| Expr { kind, ty, line };
    let run = Stmt::ForRange {
        var: new_var,
        start: at(ExprKind::Local(0), Type::I64),
        end: at(ExprKind::Local(1), Type::I64),
        inclusive: false,
        body,
    };
    Function {
        name: format!("{name} chunk"),
        params: (0..2 + plan.caps.len() as Local).collect(),
        ret: Type::I64,
        locals: chunk_locals,
        body: Block {
            stmts: vec![run],
            value: Some(Box::new(at(ExprKind::Int(0), Type::I64))),
        },
        line,
        fails: false,
        external: None,
    }
}

fn remap_block(
    b: &mut Block,
    map: &mut Vec<(Local, Local)>,
    chunk_locals: &mut Vec<Type>,
    locals: &[Type],
) {
    for s in &mut b.stmts {
        match s {
            Stmt::Let(l, e) => {
                remap_expr(e, map);
                let new = chunk_locals.len() as Local;
                chunk_locals.push(locals[*l as usize]);
                map.push((*l, new));
                *l = new;
            }
            Stmt::Expr(e) => remap_expr(e, map),
            _ => unreachable!("eligible bodies hold only lets and stores"),
        }
    }
}

pub(super) fn remap_expr(e: &mut Expr, map: &mut [(Local, Local)]) {
    match &mut e.kind {
        ExprKind::Local(l) | ExprKind::Move(l) => *l = renumber(*l, map),
        ExprKind::Assign(l, v) => {
            remap_expr(v, map);
            *l = renumber(*l, map);
        }
        _ => {
            for_parts(e, |p| remap_expr(p, map));
        }
    }
}

fn renumber(l: Local, map: &[(Local, Local)]) -> Local {
    map.iter()
        .rev()
        .find(|(old, _)| *old == l)
        .map(|(_, new)| *new)
        .expect("every local in an eligible body is mapped")
}

fn for_parts(e: &mut Expr, mut f: impl FnMut(&mut Expr)) {
    match &mut e.kind {
        ExprKind::Binary(_, a, b)
        | ExprKind::Fast(_, a, b)
        | ExprKind::Index(a, b)
        | ExprKind::InRange(a, b) => {
            f(a);
            f(b);
        }
        ExprKind::Unary(_, x) | ExprKind::Convert(x, _) => f(x),
        ExprKind::Rt(_, args) => {
            for a in args {
                f(a);
            }
        }
        _ => {}
    }
}
