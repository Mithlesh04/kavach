//! The reduction half of decision 0031: building a sum's chunk
//! function and the call site that combines the partial sums exactly.
//! Split from `par.rs`.

use crate::ir::{BinOp, Block, Expr, ExprKind, FastOp, Function, Local, Stmt, Type};
use crate::par::SLOTS;
use crate::par::analyze::RedPlan;
use crate::par::chunk::remap_expr;
use crate::rt_op::RtOp;

/// The line a statement sits on, for synthesized code.
pub(super) fn stmt_line(s: &Stmt) -> u32 {
    match s {
        Stmt::Let(_, e) | Stmt::Expr(e) => e.line,
        Stmt::ForRange { start, .. } => start.line,
        _ => 0,
    }
}

fn at(kind: ExprKind, ty: Type, line: u32) -> Expr {
    Expr { kind, ty, line }
}

fn int(n: i128, line: u32) -> Expr {
    at(ExprKind::Int(n), Type::I64, line)
}

fn read(l: Local, ty: Type, line: u32) -> Expr {
    at(ExprKind::Local(l), ty, line)
}

fn bin(op: BinOp, a: Expr, b: Expr, ty: Type, line: u32) -> Expr {
    at(ExprKind::Binary(op, Box::new(a), Box::new(b)), ty, line)
}

/// `if cond { stmts }`, as a unit statement.
fn when(cond: Expr, stmts: Vec<Stmt>, line: u32) -> Stmt {
    let then = Block { stmts, value: None };
    Stmt::Expr(at(
        ExprKind::If(Box::new(cond), then, Block::default()),
        Type::Unit,
        line,
    ))
}

fn set(l: Local, value: Expr, line: u32) -> Stmt {
    Stmt::Expr(at(ExprKind::Assign(l, Box::new(value)), Type::Unit, line))
}

/// The call site of a reduction: run the chunks, then combine their
/// partial sums exactly — a serial prefix that would have left `Int`
/// anywhere inside a chunk (its stored prefix bounds say) falls back to
/// the plain loop, which panics at the right round.
pub(super) fn red_call_site(
    loop_stmt: &Stmt,
    plan: &RedPlan,
    chunk: usize,
    locals: &mut Vec<Type>,
    slots: [Local; 3],
    start: Expr,
    end: Expr,
) -> Vec<Stmt> {
    let line = start.line;
    let i64t = Type::I64;
    let mut new = |ty: Type| {
        locals.push(ty);
        (locals.len() - 1) as Local
    };
    let (flag, offset, kk) = (new(i64t), new(i64t), new(i64t));
    let (mx, mn, pv) = (new(i64t), new(i64t), new(i64t));
    let list_ty = locals[slots[0] as usize];
    let mut args = vec![start, end, int(chunk as i128, line)];
    for c in &plan.caps {
        args.push(read(*c, locals[*c as usize], line));
    }
    for sl in slots {
        args.push(read(sl, list_ty, line));
    }
    let run = Stmt::Let(flag, at(ExprKind::Rt(RtOp::ParForRed, args), i64t, line));
    let item = |list: Local, i: Local| {
        at(
            ExprKind::Index(
                Box::new(read(list, list_ty, line)),
                Box::new(read(i, i64t, line)),
            ),
            i64t,
            line,
        )
    };
    let flag_up = |cond: Expr| when(cond, vec![set(flag, int(1, line), line)], line);
    // `bound > 0 && offset > MAX - bound` (and the mirror): the serial
    // prefix would leave `Int` inside this chunk.
    let and = |a: Expr, b: Expr| at(ExprKind::And(Box::new(a), Box::new(b)), Type::Bool, line);
    let spills_up = |l: Local| {
        and(
            bin(
                BinOp::Gt,
                read(l, i64t, line),
                int(0, line),
                Type::Bool,
                line,
            ),
            bin(
                BinOp::Gt,
                read(offset, i64t, line),
                bin(
                    BinOp::Sub,
                    int(i64::MAX as i128, line),
                    read(l, i64t, line),
                    i64t,
                    line,
                ),
                Type::Bool,
                line,
            ),
        )
    };
    let spills_down = |l: Local| {
        and(
            bin(
                BinOp::Lt,
                read(l, i64t, line),
                int(0, line),
                Type::Bool,
                line,
            ),
            bin(
                BinOp::Lt,
                read(offset, i64t, line),
                bin(
                    BinOp::Sub,
                    int(i64::MIN as i128, line),
                    read(l, i64t, line),
                    i64t,
                    line,
                ),
                Type::Bool,
                line,
            ),
        )
    };
    let flag_is = |n: i128| {
        bin(
            if n == 0 { BinOp::Eq } else { BinOp::Ne },
            read(flag, i64t, line),
            int(0, line),
            Type::Bool,
            line,
        )
    };
    let walk = Stmt::ForRange {
        var: kk,
        start: int(0, line),
        end: int(SLOTS as i128, line),
        inclusive: false,
        body: Block {
            stmts: vec![
                Stmt::Let(mx, item(slots[1], kk)),
                Stmt::Let(mn, item(slots[2], kk)),
                Stmt::Let(pv, item(slots[0], kk)),
                flag_up(spills_up(mx)),
                flag_up(spills_down(mn)),
                flag_up(spills_up(pv)),
                flag_up(spills_down(pv)),
                when(
                    flag_is(0),
                    vec![set(
                        offset,
                        bin(
                            BinOp::Add,
                            read(offset, i64t, line),
                            read(pv, i64t, line),
                            i64t,
                            line,
                        ),
                        line,
                    )],
                    line,
                ),
            ],
            value: None,
        },
    };
    let combine = when(
        flag_is(0),
        vec![
            Stmt::Let(offset, read(plan.acc, i64t, line)),
            walk,
            when(
                flag_is(0),
                vec![set(
                    plan.acc,
                    match plan.modulus {
                        // The flat sum `% m`: the same number the
                        // round-by-round form gives (see `analyze.rs`).
                        Some(m) => bin(
                            BinOp::Rem,
                            read(offset, i64t, line),
                            int(m, line),
                            i64t,
                            line,
                        ),
                        None => read(offset, i64t, line),
                    },
                    line,
                )],
                line,
            ),
        ],
        line,
    );
    let replay = when(flag_is(1), vec![loop_stmt.clone()], line);
    vec![run, combine, replay]
}

/// The reduction's chunk: sums its rounds from zero with checked adds
/// (an overflow bails out quietly), tracks its running prefix's bounds,
/// and stores all three in its slot.
pub(super) fn make_red_chunk(
    loop_stmt: &Stmt,
    plan: &RedPlan,
    locals: &[Type],
    scratch_ty: Type,
    name: &str,
) -> Function {
    let line = stmt_line(loop_stmt);
    let i64t = Type::I64;
    let caps = &plan.caps;
    let mut chunk_locals = vec![i64t; 3];
    let mut map: Vec<(Local, Local)> = Vec::new();
    for (k, c) in caps.iter().enumerate() {
        map.push((*c, (k + 3) as Local));
        chunk_locals.push(locals[*c as usize]);
    }
    // The slot lists ride along as the last three parameters.
    let p_part = chunk_locals.len() as Local;
    let (p_max, p_min) = (p_part + 1, p_part + 2);
    chunk_locals.extend([scratch_ty; 3]);
    let params: Vec<Local> = (0..chunk_locals.len() as Local).collect();
    let mut new = |ty: Type| {
        chunk_locals.push(ty);
        (chunk_locals.len() - 1) as Local
    };
    let (sum, pmax, pmin, iv) = (new(i64t), new(i64t), new(i64t), new(i64t));
    let mut round = Vec::new();
    let (body, var) = match loop_stmt {
        Stmt::ForRange { var, body, .. } => {
            map.push((*var, iv));
            (body, *var)
        }
        Stmt::ForEach {
            var,
            index,
            list,
            body,
            ..
        } => {
            // The item: `list[i]`, where the list is the first capture.
            let list_param = map
                .iter()
                .find(|(old, _)| old == list)
                .expect("the list is captured")
                .1;
            let item = new(i64t);
            round.push(Stmt::Let(
                item,
                at(
                    ExprKind::Index(
                        Box::new(read(list_param, locals[*list as usize], line)),
                        Box::new(read(iv, i64t, line)),
                    ),
                    i64t,
                    line,
                ),
            ));
            map.push((*var, item));
            map.push((*index, iv));
            (body, *var)
        }
        _ => unreachable!("a counted loop or a list loop"),
    };
    let _ = var;
    for s in &body.stmts {
        match s {
            Stmt::Let(l, e) => {
                let mut e = e.clone();
                remap_expr(&mut e, &mut map);
                let new_l = chunk_locals.len() as Local;
                chunk_locals.push(locals[*l as usize]);
                map.push((*l, new_l));
                round.push(Stmt::Let(new_l, e));
            }
            Stmt::Expr(e) => {
                // The accumulation: `sum += value`, checked, so a chunk
                // whose own sum leaves `Int` bails out to the replay.
                let ExprKind::Assign(_, v) = &e.kind else {
                    unreachable!("an eligible reduction");
                };
                let added = match &v.kind {
                    ExprKind::Fast(FastOp::RemOnce, inner, _) => inner,
                    _ => v,
                };
                let (ExprKind::Binary(BinOp::Add, _, value)
                | ExprKind::Fast(FastOp::Add, _, value)) = &added.kind
                else {
                    unreachable!("an eligible reduction");
                };
                let mut value = value.clone();
                remap_expr(&mut value, &mut map);
                round.push(set(
                    sum,
                    bin(BinOp::Add, read(sum, i64t, line), *value, i64t, line),
                    line,
                ));
            }
            _ => unreachable!("an eligible reduction"),
        }
    }
    for (bound, op) in [(pmax, BinOp::Gt), (pmin, BinOp::Lt)] {
        round.push(when(
            bin(
                op,
                read(sum, i64t, line),
                read(bound, i64t, line),
                Type::Bool,
                line,
            ),
            vec![set(bound, read(sum, i64t, line), line)],
            line,
        ));
    }
    let mut stmts = vec![
        Stmt::Let(sum, int(0, line)),
        Stmt::Let(pmax, int(0, line)),
        Stmt::Let(pmin, int(0, line)),
        Stmt::ForRange {
            var: iv,
            start: read(0, i64t, line),
            end: read(1, i64t, line),
            inclusive: false,
            body: Block {
                stmts: round,
                value: None,
            },
        },
    ];
    for (param, value) in [(p_part, sum), (p_max, pmax), (p_min, pmin)] {
        stmts.push(set(
            param,
            at(
                ExprKind::Rt(
                    RtOp::ListSet,
                    vec![
                        at(ExprKind::Move(param), scratch_ty, line),
                        read(2, i64t, line),
                        read(value, i64t, line),
                    ],
                ),
                scratch_ty,
                line,
            ),
            line,
        ));
    }
    Function {
        name: format!("{name} sum chunk"),
        params,
        ret: i64t,
        locals: chunk_locals,
        body: Block {
            stmts,
            value: Some(Box::new(int(0, line))),
        },
        line,
        fails: false,
        external: None,
    }
}
