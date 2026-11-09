//! Values computed before an early exit in the same expression, as in
//! `a + (x ?? return none)`, `f(s, try g())` or `"{a}{x ?? continue}"`.
//! Leaving skips the rest of the expression, so a value already computed
//! for it would never be released. So the operands up to the one that
//! may leave go into locals first, in order: leaving releases locals
//! (spec 6.1: freed when its last use ends), and the expression takes
//! its operands out of them only once none of them can leave.

use crate::ir::{Block, Expr, ExprKind, Function, Local, Stmt, Type};
use crate::walk;

/// Keeps the operands computed before an early exit in locals, in every
/// expression of `f`.
pub fn keep_before_exits(f: &mut Function) {
    let mut locals = std::mem::take(&mut f.locals);
    walk::exprs_mut(&mut f.body, &mut |e| keep(e, &mut locals));
    f.locals = locals;
}

fn keep(e: &mut Expr, locals: &mut Vec<Type>) {
    let mut lets = Vec::new();
    {
        let mut ops = operands(e);
        let Some(exit) = ops.iter().rposition(|x| leaves(x)) else {
            return;
        };
        // Every operand up to the exit, in order, so nothing is evaluated
        // in a different order than before.
        for x in ops.iter_mut().take(exit + 1) {
            let constant = matches!(
                x.kind,
                ExprKind::Int(_)
                    | ExprKind::Float(_)
                    | ExprKind::Bool(_)
                    | ExprKind::Char(_)
                    | ExprKind::None
            );
            if constant || x.ty == Type::Unit {
                continue;
            }
            let l = locals.len() as Local;
            locals.push(x.ty);
            let read = Expr {
                kind: ExprKind::Local(l),
                ty: x.ty,
                line: x.line,
            };
            lets.push(Stmt::Let(l, std::mem::replace(*x, read)));
        }
    }
    if lets.is_empty() {
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
    let block = if ty == Type::Unit {
        lets.push(Stmt::Expr(inner));
        Block {
            stmts: lets,
            value: None,
        }
    } else {
        Block {
            stmts: lets,
            value: Some(Box::new(inner)),
        }
    };
    e.kind = ExprKind::Block(block);
}

/// The operands of `e` that are evaluated one after another, in order.
fn operands(e: &mut Expr) -> Vec<&mut Expr> {
    match &mut e.kind {
        ExprKind::Binary(_, a, b)
        | ExprKind::Fast(_, a, b)
        | ExprKind::Index(a, b)
        | ExprKind::InRange(a, b)
        | ExprKind::Context(a, b)
        | ExprKind::SetField(a, _, b) => vec![&mut **a, &mut **b],
        ExprKind::Format(items)
        | ExprKind::List(items)
        | ExprKind::Tuple(items)
        | ExprKind::Call(_, items)
        | ExprKind::Rt(_, items)
        | ExprKind::Record { fields: items, .. } => items.iter_mut().collect(),
        ExprKind::CallValue(callee, args) => {
            let mut all = vec![&mut **callee];
            all.extend(args.iter_mut());
            all
        }
        ExprKind::Map(pairs) => pairs.iter_mut().flat_map(|(k, v)| [k, v]).collect(),
        _ => Vec::new(),
    }
}

/// Whether evaluating `e` may leave early: `fail` (as `try` does), or
/// `return`, `break` or `continue` in a block inside it.
fn leaves(e: &Expr) -> bool {
    walk::any_expr(e, &mut |x| match &x.kind {
        ExprKind::Fail(_) => true,
        ExprKind::Block(b) => exits(b),
        ExprKind::If(_, a, b) | ExprKind::Fallback(a, b) => exits(a) || exits(b),
        _ => false,
    })
}

/// Whether `b` has a statement that leaves, at any depth of loops in it.
fn exits(b: &Block) -> bool {
    b.stmts.iter().any(|s| match s {
        Stmt::Return(_) | Stmt::Break | Stmt::Continue => true,
        Stmt::While { body, .. } | Stmt::ForRange { body, .. } | Stmt::ForEach { body, .. } => {
            exits(body)
        }
        _ => false,
    })
}
