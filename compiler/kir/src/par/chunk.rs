//! A plain loop's chunk for `kv_par_for` (`par.rs`): the call site that
//! fans it out, with the loop as written behind it, and the chunk
//! function, the loop's body over `lo..hi` with every local renumbered.
//! A reduction's are in `reduce.rs`.

use crate::ir::{BinOp, Block, Expr, ExprKind, Function, Local, Stmt, Type};
use crate::par::analyze::Plan;
use crate::rt_op::RtOp;

/// The replacement at the call site: `flag = kv_par_for(…)`, then the
/// plain loop under `if flag != 0`.
pub(super) fn call_site(
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
pub(super) fn make_chunk(loop_stmt: &Stmt, plan: &Plan, locals: &[Type], name: &str) -> Function {
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
