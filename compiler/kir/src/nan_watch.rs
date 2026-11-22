// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The NaN watch (spec 5.9): with it on, each float operator and
//! `std.math` function the program's own code writes is worked out by
//! the runtime (`NanWatch`), which panics when it gives `NaN`. With it
//! off this pass doesn't run, so a program pays nothing.

use crate::ir::{BinOp, Expr, ExprKind, Function, Type};
use crate::rt_op::RtOp;

/// The operators watched. `NanWatch`'s third operand is `-1 - place`
/// here, `-11 - place` on `Float32`s, or a `std.math` op's code.
pub const WATCHED: [BinOp; 5] = [BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div, BinOp::Rem];

/// Watches `f`'s float operations on lines before `std_from`, the first
/// line of the standard library, whose code makes `NaN` on purpose.
pub(crate) fn watch_nan(f: &mut Function, std_from: u32) {
    crate::walk::exprs_mut(&mut f.body, &mut |e| {
        if e.line < std_from {
            watch(e);
        }
    });
}

/// `e` through `NanWatch`, when it is a watched float operation.
fn watch(e: &mut Expr) {
    let what = match &e.kind {
        ExprKind::Binary(op, _, _) if matches!(e.ty, Type::F64 | Type::F32) => {
            let Some(place) = WATCHED.iter().position(|o| o == op) else {
                return;
            };
            let narrow = if e.ty == Type::F32 { 10 } else { 0 };
            -1 - narrow - place as i128
        }
        ExprKind::Rt(op, args) if e.ty == Type::F64 && is_math(*op) && args.len() <= 2 => {
            i128::from(op.code())
        }
        _ => return,
    };
    let line = e.line;
    let int = |n: i128| Expr {
        kind: ExprKind::Int(n),
        ty: Type::I64,
        line,
    };
    let (a, b) = match std::mem::replace(&mut e.kind, ExprKind::None) {
        ExprKind::Binary(_, a, b) => (*a, *b),
        ExprKind::Rt(_, args) => {
            let mut args = args.into_iter();
            let a = args.next().unwrap_or_else(|| int(0));
            (a, args.next().unwrap_or_else(|| int(0)))
        }
        _ => unreachable!("matched above"),
    };
    e.kind = ExprKind::Rt(RtOp::NanWatch, vec![a, b, int(what)]);
}

/// `std.math`'s functions of floats (14.5).
fn is_math(op: RtOp) -> bool {
    format!("{op:?}").starts_with("Math")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn float(kind: ExprKind, ty: Type) -> Expr {
        Expr { kind, ty, line: 3 }
    }

    /// Float operators and `std.math` calls go through the runtime; other
    /// operations, and the standard library's lines, stay as they are.
    #[test]
    fn only_float_operations_of_the_program_are_watched() {
        let x = || Box::new(float(ExprKind::Float(1.0), Type::F64));
        let cases = [
            (
                float(ExprKind::Binary(BinOp::Div, x(), x()), Type::F64),
                10,
                Some(-4),
            ),
            (
                float(ExprKind::Binary(BinOp::Add, x(), x()), Type::F32),
                10,
                Some(-11),
            ),
            (
                float(ExprKind::Rt(RtOp::MathSqrt, vec![*x()]), Type::F64),
                10,
                Some(RtOp::MathSqrt.code() as i128),
            ),
            (
                float(ExprKind::Binary(BinOp::Lt, x(), x()), Type::Bool),
                10,
                None,
            ),
            (
                float(ExprKind::Binary(BinOp::Add, x(), x()), Type::I64),
                10,
                None,
            ),
            (
                float(ExprKind::Binary(BinOp::Div, x(), x()), Type::F64),
                3,
                None,
            ),
        ];
        for (i, (e, std_from, want)) in cases.into_iter().enumerate() {
            let mut f = Function {
                name: "f".into(),
                params: Vec::new(),
                ret: Type::Unit,
                locals: Vec::new(),
                body: crate::ir::Block {
                    stmts: vec![crate::ir::Stmt::Expr(e)],
                    value: None,
                },
                line: 1,
                fails: false,
                external: None,
                variadic: false,
            };
            watch_nan(&mut f, std_from);
            let crate::ir::Stmt::Expr(e) = &f.body.stmts[0] else {
                unreachable!()
            };
            let got = match &e.kind {
                ExprKind::Rt(RtOp::NanWatch, args) => match args[2].kind {
                    ExprKind::Int(n) => Some(n),
                    _ => None,
                },
                _ => None,
            };
            assert_eq!(got, want, "case {i}");
        }
    }
}
