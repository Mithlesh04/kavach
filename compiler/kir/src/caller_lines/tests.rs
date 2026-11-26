// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

use crate::ir::{Block, Expr, ExprKind, Program, Stmt, Type};
use crate::rt_op::RtOp;

fn lowered(src: &str) -> Program {
    let c = kavach_driver::compile(src, "t.kav", Default::default());
    assert!(!c.has_errors(), "{}", c.render());
    crate::lower(&c).expect("lowers")
}

/// How many calls set the call line, in the whole program.
fn marked(program: &Program) -> usize {
    let mut n = 0;
    for f in &program.functions {
        crate::walk::any_in_block(&f.body, &mut |e| {
            n += usize::from(matches!(e.kind, ExprKind::Rt(RtOp::CallLineSwap, _)));
            false
        });
    }
    n
}

/// A call of a library function that can panic (a `require`) sets the
/// line a panic in it shows (spec 5.6).
#[test]
fn a_call_into_the_library_that_can_panic_is_marked() {
    let program = lowered("use std.math\n\nfn main() {\n    print(math.gcd(12, 18))\n}\n");
    assert_eq!(marked(&program), 1);
}

/// Float arithmetic can't panic, so `std.vec`'s `+` on floats costs
/// nothing more; a program that calls no library function has no marks.
#[test]
fn calls_that_cant_panic_are_not_marked() {
    let floats = lowered(
        "use std.vec\n\nfn main() {\n    const a = Vec2(x: 1.0, y: 2.0) + Vec2(x: 3.0, y: 4.0)\n    print(a.x)\n}\n",
    );
    assert_eq!(marked(&floats), 0);
    let own =
        lowered("fn twice(n: Int) -> Int {\n    n * 2\n}\n\nfn main() {\n    print(twice(4))\n}\n");
    assert_eq!(marked(&own), 0);
}

/// An `if` chain as a `match` of `depth` arms becomes, each arm's
/// condition `true`, with `last` in the innermost `else`.
fn chain(depth: u32, last: Block) -> Block {
    let at = |kind| Expr {
        kind,
        ty: Type::Unit,
        line: 1,
    };
    let mut rest = last;
    for _ in 0..depth {
        let arm = Block::default();
        rest = Block {
            stmts: Vec::new(),
            value: Some(Box::new(at(ExprKind::If(
                Box::new(at(ExprKind::Bool(true))),
                arm,
                rest,
            )))),
        };
    }
    rest
}

/// Each block of a long `match` is looked at once: a check in a loop in
/// its last arm is found at once, as is the lack of one (it took time
/// doubling with each arm).
#[test]
fn a_check_deep_in_a_long_if_chain_is_found_at_once() {
    let check = Stmt::Check {
        cond: Expr {
            kind: ExprKind::Bool(true),
            ty: Type::Bool,
            line: 1,
        },
        message: "x".into(),
        line: 1,
    };
    let in_loop = Block {
        stmts: vec![Stmt::While {
            cond: Expr {
                kind: ExprKind::Bool(false),
                ty: Type::Bool,
                line: 1,
            },
            body: Block {
                stmts: vec![check],
                value: None,
            },
        }],
        value: None,
    };
    assert!(super::has_check(&chain(200, in_loop)));
    assert!(!super::has_check(&chain(200, Block::default())));
}
