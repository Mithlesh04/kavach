// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

use crate::ir::{ExprKind, Program};
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
