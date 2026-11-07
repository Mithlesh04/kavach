//! A recursion that counts one parameter down to zero, as a counted loop:
//! `fn down(n, s) = if n == 0 { base } else { each + down(n - 1, s) }`
//! becomes
//!
//! ```text
//! check 0 <= n <= MAX_ROUNDS      // else: the code as written
//! acc = base (with n = 0)
//! for k in 1..=n { acc = acc + each (with n = k) }
//! ```
//!
//! The adds are the recursion's own, in its order: the deepest call adds
//! first, so a check that fails here fails where the code as written
//! would. And a counted loop is what the range pass reasons best about:
//! `k` is within `1..=MAX_ROUNDS`, and the rounds bound `acc`.

use super::MAX_ROUNDS;
use crate::ir::*;

/// The counted loop for a recursion with condition `c` (choosing the
/// base case, or the recursive one when `flipped`), base case `base`,
/// `each` added per round, and the self-call's `args`. Gives the loop and
/// `f`'s locals with the loop's added; `None` when the recursion doesn't
/// count a parameter down by one to zero, passing the others on as they
/// are.
pub(super) fn counted(
    f: &Function,
    (c, flipped): (&Expr, bool),
    base: &Expr,
    each: Option<&Expr>,
    args: &[Expr],
) -> Option<(Block, Vec<Type>)> {
    let ExprKind::Binary(op, x, y) = &c.kind else {
        return None;
    };
    let want = if flipped { BinOp::Ne } else { BinOp::Eq };
    let p = match (&x.kind, &y.kind) {
        (ExprKind::Local(l), ExprKind::Int(0)) | (ExprKind::Int(0), ExprKind::Local(l))
            if *op == want =>
        {
            *l
        }
        _ => return None,
    };
    let at = f.params.iter().position(|q| *q == p)?;
    if f.locals[p as usize] != Type::I64 {
        return None;
    }
    let is = |e: &Expr, l: Local| matches!(e.kind, ExprKind::Local(x) if x == l);
    for (i, (a, q)) in args.iter().zip(&f.params).enumerate() {
        let passed = match &a.kind {
            ExprKind::Binary(BinOp::Sub, m, one) if i == at => {
                is(m, p) && matches!(one.kind, ExprKind::Int(1))
            }
            _ => i != at && is(a, *q),
        };
        if !passed {
            return None;
        }
    }
    let line = c.line;
    let mut locals = f.locals.clone();
    let mut fresh = |ty: Type| {
        locals.push(ty);
        (locals.len() - 1) as Local
    };
    let (acc, zero, k) = (fresh(Type::I64), fresh(Type::I64), fresh(Type::I64));
    let x = each.map(|e| fresh(e.ty));
    let e = |kind: ExprKind, ty: Type| Expr { kind, ty, line };
    let local = |l: Local| e(ExprKind::Local(l), Type::I64);
    let int = |n: i128| e(ExprKind::Int(n), Type::I64);
    // `v` with `n` read from `to`.
    let at_n = |v: &Expr, to: Local| {
        let mut b = Block {
            stmts: Vec::new(),
            value: Some(Box::new(v.clone())),
        };
        crate::walk::locals_mut(&mut b, &mut |l| {
            if *l == p {
                *l = to;
            }
        });
        *b.value.expect("the value put in")
    };
    let check = |op: BinOp, n: i128| Stmt::Check {
        cond: e(
            ExprKind::Binary(op, Box::new(local(p)), Box::new(int(n))),
            Type::Bool,
        ),
        message: "deep recursion".to_string(),
        line,
    };
    let mut stmts = vec![
        check(BinOp::Ge, 0),
        check(BinOp::Le, MAX_ROUNDS),
        Stmt::Let(zero, int(0)),
        Stmt::Let(acc, at_n(base, zero)),
    ];
    if let (Some(each), Some(x)) = (each, x) {
        let sum = e(
            ExprKind::Binary(BinOp::Add, Box::new(local(acc)), Box::new(local(x))),
            Type::I64,
        );
        let body = Block {
            stmts: vec![
                Stmt::Let(x, at_n(each, k)),
                Stmt::Expr(e(ExprKind::Assign(acc, Box::new(sum)), Type::Unit)),
            ],
            value: None,
        };
        stmts.push(Stmt::ForRange {
            var: k,
            start: int(1),
            end: local(p),
            inclusive: true,
            body,
        });
    }
    let fast = Block {
        stmts,
        value: Some(Box::new(local(acc))),
    };
    Some((fast, locals))
}
