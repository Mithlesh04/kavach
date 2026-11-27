// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `moves` must follow the order the code generators evaluate operands
//! in (spec 3.10): the read that becomes a move is the last one in that
//! order. Hand-made functions pin the order for calls, records, function
//! values and methods that change their receiver; whole programs, with
//! named arguments and fields written out of order, are checked for a
//! read after a move.

use super::*;
use crate::rt_op::RtOp;

const LIST: Type = Type::Ref(0);

fn e(kind: ExprKind, ty: Type) -> Expr {
    Expr { kind, ty, line: 1 }
}

fn read(l: Local) -> Expr {
    e(ExprKind::Local(l), LIST)
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

fn marked(mut f: Function) -> Function {
    mark_moves(&mut f, &HashSet::new());
    f
}

/// Whether each of `items` is a move.
fn moves(items: &[Expr]) -> Vec<bool> {
    (items.iter())
        .map(|x| matches!(x.kind, ExprKind::Move(_)))
        .collect()
}

/// The expression of statement `i` of `f`.
fn stmt_expr(f: &Function, i: usize) -> &Expr {
    match &f.body.stmts[i] {
        Stmt::Expr(x) | Stmt::Let(_, x) => x,
        other => panic!("not an expression: {other:?}"),
    }
}

/// `f(xs, xs)`, a record `{xs, xs}` and the items of a list: left to
/// right, so the second read is the last and only it moves.
#[test]
fn operands_move_left_to_right() {
    for kind in [
        ExprKind::Call(0, vec![read(0), read(0)]),
        ExprKind::Record {
            variant: 0,
            fields: vec![read(0), read(0)],
        },
        ExprKind::Rt(RtOp::ListGet, vec![read(0), read(0)]),
    ] {
        let f = marked(function(vec![LIST], vec![Stmt::Expr(e(kind, LIST))]));
        let (ExprKind::Call(_, items)
        | ExprKind::Record { fields: items, .. }
        | ExprKind::Rt(_, items)) = &stmt_expr(&f, 0).kind
        else {
            unreachable!()
        };
        assert_eq!(moves(items), [false, true], "{:?}", stmt_expr(&f, 0));
    }
}

/// `g(xs)` where `g` is a function value read from `xs`'s own local:
/// the function value is worked out before its arguments (spec 3.10).
#[test]
fn a_function_value_comes_before_its_arguments() {
    let call = ExprKind::CallValue(Box::new(read(0)), vec![read(0)]);
    let f = marked(function(vec![LIST], vec![Stmt::Expr(e(call, LIST))]));
    let ExprKind::CallValue(callee, args) = &stmt_expr(&f, 0).kind else {
        unreachable!()
    };
    assert!(matches!(callee.kind, ExprKind::Local(0)));
    assert_eq!(moves(args), [true]);
}

/// `xs.push(ys)` as `xs = push(xs, ys)`: the code generator reads the
/// receiver last, so it moves (the list changes in place); `ys`, read
/// again afterwards, does not.
#[test]
fn a_method_that_changes_its_receiver_reads_it_last() {
    let push = ExprKind::Rt(RtOp::ListPush, vec![read(0), read(1)]);
    let f = marked(function(
        vec![LIST, LIST, LIST],
        vec![
            Stmt::Expr(e(ExprKind::Assign(0, Box::new(e(push, LIST))), Type::Unit)),
            Stmt::Let(2, read(1)),
            Stmt::Expr(read(0)),
        ],
    ));
    let ExprKind::Assign(_, value) = &stmt_expr(&f, 0).kind else {
        unreachable!()
    };
    let ExprKind::Rt(_, args) = &value.kind else {
        unreachable!()
    };
    assert_eq!(moves(args), [true, false]);
    assert!(matches!(stmt_expr(&f, 1).kind, ExprKind::Move(1)));
}

/// A list a loop reads each round is never moved inside the loop, even
/// when nothing reads it after the loop.
#[test]
fn a_read_in_a_loop_is_not_a_move() {
    let body = Block {
        stmts: vec![Stmt::Expr(e(ExprKind::Call(0, vec![read(0)]), LIST))],
        value: None,
    };
    let f = marked(function(
        vec![LIST, Type::I64],
        vec![Stmt::ForRange {
            var: 1,
            start: e(ExprKind::Int(0), Type::I64),
            end: e(ExprKind::Int(3), Type::I64),
            inclusive: false,
            body,
        }],
    ));
    let Stmt::ForRange { body, .. } = &f.body.stmts[0] else {
        unreachable!()
    };
    let Stmt::Expr(call) = &body.stmts[0] else {
        unreachable!()
    };
    let ExprKind::Call(_, args) = &call.kind else {
        unreachable!()
    };
    assert_eq!(moves(args), [false]);
}

/// Reads of locals after a move of them, in the order operands are
/// evaluated: each is a value read from an emptied variable.
fn reads_after_moves(f: &Function) -> Vec<String> {
    let mut found = Vec::new();
    let mut moved = Live::new();
    after_block(&f.body, &mut moved, &mut found, &f.name);
    found
}

fn after_block(b: &Block, moved: &mut Live, found: &mut Vec<String>, name: &str) {
    for s in &b.stmts {
        match s {
            Stmt::Let(l, x)
            | Stmt::LetResult {
                value: l, call: x, ..
            } => {
                after(x, moved, found, name);
                moved.remove(l);
            }
            Stmt::Expr(x) | Stmt::Check { cond: x, .. } | Stmt::Return(Some(x)) => {
                after(x, moved, found, name)
            }
            Stmt::Print(parts) => {
                for p in parts {
                    if let PrintPart::Value(x) = p {
                        after(x, moved, found, name);
                    }
                }
            }
            // Twice, so a move in one round read in the next shows.
            Stmt::While { cond, body } => {
                for _ in 0..2 {
                    after(cond, moved, found, name);
                    after_block(body, moved, found, name);
                }
            }
            Stmt::ForRange {
                start, end, body, ..
            } => {
                after(start, moved, found, name);
                after(end, moved, found, name);
                after_block(body, moved, found, name);
                after_block(body, moved, found, name);
            }
            Stmt::ForEach { body, .. } => {
                after_block(body, moved, found, name);
                after_block(body, moved, found, name);
            }
            Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
        }
    }
    if let Some(v) = &b.value {
        after(v, moved, found, name);
    }
}

/// `e`'s operands in the order they are evaluated, as `moves` assumes.
fn after(e: &Expr, moved: &mut Live, found: &mut Vec<String>, name: &str) {
    match &e.kind {
        ExprKind::Local(l) | ExprKind::Move(l) => {
            if moved.contains(l) {
                found.push(format!(
                    "{name}: local {l} read after a move (line {})",
                    e.line
                ));
            }
            if matches!(e.kind, ExprKind::Move(_)) {
                moved.insert(*l);
            }
        }
        // A change in place reads its receiver last (`assign` above).
        ExprKind::Assign(l, value) => {
            let is_l =
                |x: &Expr| matches!(x.kind, ExprKind::Local(y) | ExprKind::Move(y) if y == *l);
            match &value.kind {
                ExprKind::Rt(op, args) if op.changes_receiver() && is_l(&args[0]) => {
                    args[1..].iter().for_each(|x| after(x, moved, found, name));
                    after(&args[0], moved, found, name);
                }
                ExprKind::SetField(base, _, field) if is_l(base) => {
                    after(field, moved, found, name);
                    after(base, moved, found, name);
                }
                _ => after(value, moved, found, name),
            }
            moved.remove(l);
        }
        ExprKind::Fail(x)
        | ExprKind::Unary(_, x)
        | ExprKind::Convert(x, _)
        | ExprKind::Some(x)
        | ExprKind::IsSome(x)
        | ExprKind::Unwrap(x)
        | ExprKind::GetField(x, _)
        | ExprKind::HasVariant(x, _)
        | ExprKind::Parse(x) => after(x, moved, found, name),
        ExprKind::Binary(_, a, b)
        | ExprKind::Fast(_, a, b)
        | ExprKind::Index(a, b)
        | ExprKind::InRange(a, b)
        | ExprKind::Context(a, b)
        | ExprKind::SetField(a, _, b) => {
            after(a, moved, found, name);
            after(b, moved, found, name);
        }
        ExprKind::And(a, b) | ExprKind::Or(a, b) => {
            after(a, moved, found, name);
            let mut maybe = moved.clone();
            after(b, &mut maybe, found, name);
            moved.extend(maybe);
        }
        ExprKind::If(cond, then, otherwise) => {
            after(cond, moved, found, name);
            let mut other = moved.clone();
            after_block(then, moved, found, name);
            after_block(otherwise, &mut other, found, name);
            moved.extend(other);
        }
        ExprKind::Block(b) => after_block(b, moved, found, name),
        // The slow half may run after any part of the fast one.
        ExprKind::Fallback(fast, slow) => {
            after_block(fast, moved, found, name);
            after_block(slow, moved, found, name);
        }
        ExprKind::Format(items)
        | ExprKind::List(items)
        | ExprKind::Tuple(items)
        | ExprKind::Call(_, items)
        | ExprKind::Rt(_, items)
        | ExprKind::Record { fields: items, .. }
        | ExprKind::Closure {
            captures: items, ..
        } => items.iter().for_each(|x| after(x, moved, found, name)),
        ExprKind::Map(pairs) => pairs.iter().for_each(|(k, v)| {
            after(k, moved, found, name);
            after(v, moved, found, name);
        }),
        ExprKind::CallValue(callee, args) => {
            after(callee, moved, found, name);
            args.iter().for_each(|x| after(x, moved, found, name));
        }
        _ => {}
    }
}

/// The program at `src`, lowered with every pass.
fn lowered(src: &str) -> crate::ir::Program {
    let c = kavach_driver::compile(src, "t.kav", Default::default());
    assert!(!c.has_errors(), "{}", c.render());
    crate::lower(&c).expect("lowers")
}

/// Named arguments in another order than the parameters, fields written
/// out of order and methods on a list read again: no variable is read
/// after it was moved, in the order the operands are worked out.
#[test]
fn no_read_follows_a_move_in_whole_programs() {
    let programs = [
        "fn pair(a: [Int], b: Int) -> Int {\n    a.len() + b\n}\n\nfn main() {\n    let xs = [1, 2, 3]\n    xs.push(4)\n    print(pair(b: xs.len(), a: xs))\n}\n",
        "fn keep(first: [Int], second: [Int]) -> [Int] {\n    let all = first\n    all.push(second.len())\n    all\n}\n\nfn main() {\n    const xs = [1, 2]\n    const ys = keep(second: xs, first: xs)\n    print(`{xs.len()} {ys.len()}`)\n}\n",
        "type Two {\n    left: [Int]\n    right: [Int]\n}\n\nfn main() {\n    const xs = [5]\n    const t = Two(right: xs, left: xs)\n    print(`{t.left.len()} {t.right.len()}`)\n}\n",
        "fn main() {\n    let rows: [[Int]] = [[1], [2]]\n    const row = rows[0]\n    rows[1].push(row.len())\n    rows.push(row)\n    print(rows.len())\n}\n",
    ];
    for src in programs {
        let program = lowered(src);
        for f in &program.functions {
            let found = reads_after_moves(f);
            assert!(found.is_empty(), "{src}\n{}", found.join("\n"));
        }
    }
}
