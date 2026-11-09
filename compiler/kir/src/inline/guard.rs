//! Guard inlining: a function that starts with a quick exit, such as
//! `if n < 2 { n } else { … }`, has that test copied into each call left
//! after inlining: `f(a)` becomes `if a < 2 { a } else { f(a) }`. The
//! quick case (the leaves of a recursion, about half of all its calls)
//! then needs no call at all.
//!
//! Only tests that cannot do anything else are copied: comparisons of
//! integer parameters with constants, and an early value that is a
//! parameter or a constant. They cannot panic, print or change anything,
//! so whether they run in the caller or the callee makes no difference.

use crate::ir::*;
use crate::walk::{exprs_mut, locals_mut};

/// A function's quick exit: its parameters, the test, the early value,
/// and whether the early value is taken when the test is true.
struct Guard {
    params: Vec<Local>,
    test: Expr,
    early: Expr,
    when: bool,
}

pub(super) fn add_guards(functions: &mut [Function]) {
    let guards: Vec<Option<Guard>> = functions.iter().map(guard_of).collect();
    for f in functions.iter_mut() {
        let locals = &mut f.locals;
        exprs_mut(&mut f.body, &mut |e| {
            let ExprKind::Call(callee, _) = &e.kind else {
                return;
            };
            let Some(g) = &guards[*callee as usize] else {
                return;
            };
            let ExprKind::Call(callee, args) = std::mem::replace(&mut e.kind, ExprKind::None)
            else {
                unreachable!("a call");
            };
            e.kind = guarded(locals, g, callee, args, e.ty, e.line);
        });
    }
}

/// `f(args)` with `g` in front: each argument into a new local, then
/// `if test { early } else { f(locals) }`.
fn guarded(
    locals: &mut Vec<Type>,
    g: &Guard,
    callee: FuncId,
    args: Vec<Expr>,
    ty: Type,
    line: u32,
) -> ExprKind {
    let at = |kind, ty| Expr { kind, ty, line };
    let temps: Vec<Local> = args
        .iter()
        .map(|a| {
            locals.push(a.ty);
            (locals.len() - 1) as Local
        })
        .collect();
    let rename = |e: &Expr| {
        let mut copy = Block {
            stmts: Vec::new(),
            value: Some(Box::new(e.clone())),
        };
        locals_mut(&mut copy, &mut |l| {
            if let Some(i) = g.params.iter().position(|p| p == l) {
                *l = temps[i];
            }
        });
        *copy.value.expect("a value")
    };
    let call_args = temps
        .iter()
        .zip(&args)
        .map(|(t, a)| at(ExprKind::Local(*t), a.ty))
        .collect();
    let value = |e: Expr| Block {
        stmts: Vec::new(),
        value: Some(Box::new(e)),
    };
    let (early, call) = (
        value(rename(&g.early)),
        value(at(ExprKind::Call(callee, call_args), ty)),
    );
    let (then, otherwise) = if g.when { (early, call) } else { (call, early) };
    let stmts = temps
        .iter()
        .zip(args)
        .map(|(t, a)| Stmt::Let(*t, a))
        .collect();
    ExprKind::Block(Block {
        stmts,
        value: Some(Box::new(at(
            ExprKind::If(Box::new(rename(&g.test)), then, otherwise),
            ty,
        ))),
    })
}

/// The quick exit at the start of `f`, if it has one that may be copied.
fn guard_of(f: &Function) -> Option<Guard> {
    if f.fails || !f.body.stmts.is_empty() {
        return None;
    }
    let ExprKind::If(test, then, otherwise) = &f.body.value.as_ref()?.kind else {
        return None;
    };
    let (then, otherwise) = (only_value(then), only_value(otherwise));
    let quiet = |e: &Expr| simple(e, &f.params);
    if !matches!(test.kind, ExprKind::Binary(..)) || !quiet(test) {
        return None;
    }
    let (early, when) = match (then, otherwise) {
        (Some(e), _) if leaf(e, &f.params) => (e, true),
        (_, Some(e)) if leaf(e, &f.params) => (e, false),
        _ => return None,
    };
    Some(Guard {
        params: f.params.clone(),
        test: (**test).clone(),
        early: early.clone(),
        when,
    })
}

/// The value of a block that has nothing else in it.
fn only_value(b: &Block) -> Option<&Expr> {
    if b.stmts.is_empty() {
        b.value.as_deref()
    } else {
        None
    }
}

/// A parameter or a constant.
fn leaf(e: &Expr, params: &[Local]) -> bool {
    match e.kind {
        ExprKind::Int(_) | ExprKind::Bool(_) => true,
        ExprKind::Local(l) => params.contains(&l),
        _ => false,
    }
}

/// Comparisons of parameters and constants: they cannot panic.
fn simple(e: &Expr, params: &[Local]) -> bool {
    match &e.kind {
        ExprKind::Binary(op, a, b) => {
            matches!(
                op,
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
            ) && leaf(a, params)
                && leaf(b, params)
                && !a.ty.is_float()
        }
        _ => leaf(e, params),
    }
}
