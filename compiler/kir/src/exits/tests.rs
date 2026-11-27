// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! An early exit in the middle of an expression: the operands before it,
//! and it, go into locals in order, so leaving releases what was already
//! computed; the operands after it stay where they are.

use super::*;
use crate::ir::BinOp;
use crate::rt_op::RtOp;

const LIST: Type = Type::Ref(0);

fn e(kind: ExprKind, ty: Type) -> Expr {
    Expr { kind, ty, line: 1 }
}

/// `{ return }` as an operand: leaves the function.
fn leave() -> Expr {
    let body = Block {
        stmts: vec![Stmt::Return(None)],
        value: Some(Box::new(e(ExprKind::Int(0), Type::I64))),
    };
    e(ExprKind::Block(body), Type::I64)
}

fn function(locals: Vec<Type>, stmts: Vec<Stmt>) -> Function {
    Function {
        name: "f".into(),
        params: Vec::new(),
        ret: Type::Unit,
        locals,
        body: Block { stmts, value: None },
        line: 1,
        fails: false,
        external: None,
        variadic: false,
    }
}

/// The locals each `let` of `b` sets, in order, and `b`'s value.
fn lets(b: &Block) -> (Vec<Local>, &Expr) {
    let set = (b.stmts.iter())
        .map(|s| match s {
            Stmt::Let(l, _) => *l,
            other => panic!("not a let: {other:?}"),
        })
        .collect();
    (set, b.value.as_deref().expect("a value"))
}

/// `g(xs, 7, { return }, ys)`: `xs` and the exit go into new locals in
/// order; the constant stays, as nothing is released for it, and so
/// does `ys`, which is never computed when the call leaves.
#[test]
fn operands_up_to_the_exit_go_into_locals_in_order() {
    let call = ExprKind::Call(
        0,
        vec![
            e(ExprKind::Local(0), LIST),
            e(ExprKind::Int(7), Type::I64),
            leave(),
            e(ExprKind::Local(1), LIST),
        ],
    );
    let mut f = function(vec![LIST, LIST], vec![Stmt::Expr(e(call, LIST))]);
    keep_before_exits(&mut f);
    assert_eq!(f.locals, [LIST, LIST, LIST, Type::I64]);
    let Stmt::Expr(Expr {
        kind: ExprKind::Block(b),
        ..
    }) = &f.body.stmts[0]
    else {
        panic!("{:?}", f.body.stmts[0])
    };
    let (set, value) = lets(b);
    assert_eq!(set, [2, 3]);
    let ExprKind::Call(0, args) = &value.kind else {
        panic!("{value:?}")
    };
    let kinds: Vec<_> = args.iter().map(|a| format!("{:?}", a.kind)).collect();
    assert_eq!(kinds, ["Local(2)", "Int(7)", "Local(3)", "Local(1)"]);
}

/// `a + (x ?? return)`, an exit as the right operand of an operator:
/// the left operand is computed first, into a local.
#[test]
fn an_operator_keeps_its_left_operand() {
    let call = e(ExprKind::Call(1, Vec::new()), Type::I64);
    let sum = ExprKind::Binary(BinOp::Add, Box::new(call), Box::new(leave()));
    let mut f = function(Vec::new(), vec![Stmt::Let(0, e(sum, Type::I64))]);
    f.locals.push(Type::I64);
    keep_before_exits(&mut f);
    let Stmt::Let(
        0,
        Expr {
            kind: ExprKind::Block(b),
            ..
        },
    ) = &f.body.stmts[0]
    else {
        panic!("{:?}", f.body.stmts[0])
    };
    let (set, value) = lets(b);
    assert_eq!(set, [1, 2]);
    assert!(matches!(&b.stmts[0], Stmt::Let(1, x) if matches!(x.kind, ExprKind::Call(1, _))));
    assert!(matches!(value.kind, ExprKind::Binary(BinOp::Add, _, _)));
}

/// An expression with no exit is left as it is.
#[test]
fn no_exit_changes_nothing() {
    let call = ExprKind::Rt(
        RtOp::ListGet,
        vec![e(ExprKind::Local(0), LIST), e(ExprKind::Int(0), Type::I64)],
    );
    let mut f = function(vec![LIST], vec![Stmt::Expr(e(call, Type::I64))]);
    keep_before_exits(&mut f);
    assert_eq!(f.locals.len(), 1);
    assert!(matches!(
        &f.body.stmts[0],
        Stmt::Expr(Expr {
            kind: ExprKind::Rt(..),
            ..
        })
    ));
}

/// `f(s, try g())` where `f` can fail: the call stays the call of its
/// `LetResult`, as code generation reads the error only of a call, and
/// the kept operands go before it in the enclosing block.
#[test]
fn a_call_that_can_fail_stays_the_call_of_its_let_result() {
    let fail = e(
        ExprKind::Fail(Box::new(e(ExprKind::Local(0), LIST))),
        Type::I64,
    );
    let call = ExprKind::Call(0, vec![e(ExprKind::Local(0), LIST), fail]);
    let mut f = function(
        vec![LIST, Type::I64, LIST],
        vec![Stmt::LetResult {
            value: 1,
            error: 2,
            call: e(call, Type::I64),
        }],
    );
    keep_before_exits(&mut f);
    let [
        Stmt::Let(3, _),
        Stmt::Let(4, _),
        Stmt::LetResult { call, .. },
    ] = &f.body.stmts[..]
    else {
        panic!("{:?}", f.body.stmts)
    };
    let ExprKind::Call(0, args) = &call.kind else {
        panic!("{call:?}")
    };
    assert!(matches!(
        args[..],
        [
            Expr {
                kind: ExprKind::Local(3),
                ..
            },
            Expr {
                kind: ExprKind::Local(4),
                ..
            }
        ]
    ));
}
