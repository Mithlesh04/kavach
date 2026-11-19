// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

use crate::ir::{ExprKind, Program};
use crate::rt_op::RtOp;

fn lowered(src: &str) -> Program {
    let c = kavach_driver::compile(src, "t.kav", Default::default());
    crate::lower(&c).expect("lowers")
}

/// Whether any function of `program` has an expression `test` holds for.
fn has(program: &Program, test: impl Fn(&ExprKind) -> bool) -> bool {
    program
        .functions
        .iter()
        .any(|f| crate::walk::any_in_block(&f.body, &mut |e| test(&e.kind)))
}

fn fits(program: &Program) -> bool {
    has(program, |k| matches!(k, ExprKind::Rt(RtOp::CheckedFits, _)))
}

/// `??` and `if let` on `checked_*` of `Int`s make no optional: the
/// check is whether the result fits (S14).
#[test]
fn a_checked_result_asked_and_unwrapped_makes_no_optional() {
    for use_it in [
        "s += i.checked_mul(f) ?? 0",
        "if let x = i.checked_add(f) {\n            s += x\n        }",
    ] {
        let src = format!(
            "fn main() {{\n    const f = cli.args().len()\n    let s = 0\n    for i in 0..10 {{\n        {use_it}\n    }}\n    print(s)\n}}\n"
        );
        let src = format!("use std.cli\n\n{src}");
        let program = lowered(&src);
        assert!(fits(&program), "{use_it}");
        assert!(
            !has(&program, |k| matches!(k, ExprKind::Some(_))),
            "{use_it}"
        );
    }
}

/// A checked result kept as a value stays an optional.
#[test]
fn a_checked_result_kept_whole_stays_optional() {
    let src = "use std.cli\n\nfn main() {\n    const f = cli.args().len()\n    const x = f.checked_mul(3)\n    print(x)\n}\n";
    assert!(!fits(&lowered(src)));
}
