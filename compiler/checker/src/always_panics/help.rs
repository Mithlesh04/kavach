// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The help line of an overflow of a type smaller than `Int` (spec 1.8):
//! both ways out, with the code's own operands. `a + b` gets `for 254,
//! widen first: Int(a) + Int(b); to wrap: a.wrapping_add(b)`, and the
//! editor and `kavach fix` write the first (`tools/lsp/src/suggest.rs`).
//! `count += 1` keeps `count`'s type, so widening its operands can't
//! help: it gets a wider type for `count` instead. A range type can't
//! wrap (spec 2.1), so it gets only the first way.

use crate::ty::Prim;
use kavach_ast::*;
use num_bigint::BigInt;

/// How the arithmetic that overflows is written.
#[derive(Clone, Copy)]
pub(super) enum Written<'e> {
    /// `l op r`, an expression.
    Expr(&'e Expr, &'e Expr),
    /// `x op= v`, which keeps `x`'s type.
    Assign(&'e Expr, &'e Expr),
}

/// The help line for `op` giving `value`, which doesn't fit type `p`;
/// none for `Int` and the wider types.
pub(super) fn help(p: Prim, op: BinaryOp, value: &BigInt, written: Written) -> Option<String> {
    let wraps = match p {
        Prim::Int8 | Prim::Int16 | Prim::Int32 | Prim::UInt8 | Prim::UInt16 | Prim::UInt32 => true,
        Prim::Bits(bits, _) if bits < 64 => true,
        Prim::Ranged(..) => false,
        _ => return None,
    };
    let method = match op {
        BinaryOp::Add => "wrapping_add",
        BinaryOp::Sub => "wrapping_sub",
        BinaryOp::Mul => "wrapping_mul",
        _ => return None,
    };
    let (wide, wrapped) = match written {
        Written::Expr(l, r) => {
            let wide = format!("widen first: {}", widened(op, l, r)?);
            // `100 + a`: the literal takes its type from `a` only as an
            // operand, so `a` is the receiver; `100 - a` names the type.
            let (receiver, argument) = match op {
                _ if !literal(l) => (receiver(l)?, code(r)?),
                BinaryOp::Add | BinaryOp::Mul => (receiver(r)?, code(l)?),
                _ => (format!("{}({})", p.name(), code(l)?), code(r)?),
            };
            (wide, format!("{receiver}.{method}({argument})"))
        }
        Written::Assign(x, v) => {
            let x = code(x)?;
            let wide = format!("declare `{x}` with a wider type");
            (wide, format!("{x} = {x}.{method}({})", code(v)?))
        }
    };
    Some(match wraps {
        true => format!("for {value}, {wide}; to wrap: {wrapped}"),
        false => format!("for {value}, {wide}"),
    })
}

/// `l op r` with each operand widened to `Int` first: `Int(a) + Int(b)`.
/// A literal stays as it is: it takes `Int` from the other side. Shared
/// with the quick fix, which finds the expression by this text.
pub fn widened(op: BinaryOp, l: &Expr, r: &Expr) -> Option<String> {
    let wide = |e: &Expr| match literal(e) {
        true => code(e),
        false => Some(format!("Int({})", code(e)?)),
    };
    Some(format!("{} {} {}", wide(l)?, sign(op)?, wide(r)?))
}

/// Whether `e` is a whole-number literal: `100`, `-100`.
fn literal(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) => true,
        ExprKind::Unary(UnaryOp::Neg, x) => matches!(x.kind, ExprKind::Int(_)),
        _ => false,
    }
}

fn sign(op: BinaryOp) -> Option<&'static str> {
    Some(match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        _ => return None,
    })
}

/// `e` as a method's receiver: in brackets unless it is one piece.
fn receiver(e: &Expr) -> Option<String> {
    match e.kind {
        ExprKind::Unary(..) | ExprKind::Binary(..) => Some(format!("({})", code(e)?)),
        _ => code(e),
    }
}

/// `e` as code, for what the compiler knows the value of
/// (`Known::value`): literals, names, a minus, conversions and `+ - *`,
/// in brackets where the order needs them.
fn code(e: &Expr) -> Option<String> {
    Some(match &e.kind {
        ExprKind::Int(n) => n.to_string(),
        ExprKind::Name(id) => id.name.clone(),
        ExprKind::Unary(UnaryOp::Neg, x) => format!("-{}", receiver(x)?),
        ExprKind::Call { callee, args, .. } => match (&callee.kind, args.as_slice()) {
            (ExprKind::Name(f), [a]) if a.name.is_none() => {
                format!("{}({})", f.name, code(&a.value)?)
            }
            _ => return None,
        },
        ExprKind::Binary(op, l, r) => {
            let sum =
                |x: &Expr| matches!(x.kind, ExprKind::Binary(BinaryOp::Add | BinaryOp::Sub, ..));
            let left = match *op == BinaryOp::Mul && sum(l) {
                true => format!("({})", code(l)?),
                false => code(l)?,
            };
            // `a - (b + c)`, `a * (b * c)`: what the right side groups.
            let right =
                match matches!(r.kind, ExprKind::Binary(..)) && (*op == BinaryOp::Mul || sum(r)) {
                    true => format!("({})", code(r)?),
                    false => code(r)?,
                };
            format!("{left} {} {right}", sign(*op)?)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    /// The help line of the first error of `body`, inside `main`.
    fn help(body: &str) -> Option<String> {
        let src = format!("fn main() {{\n{body}\n}}\n");
        let c = kavach_driver::compile(&src, "t.kav", Default::default());
        let first = c
            .messages
            .iter()
            .find(|m| m.severity == kavach_driver::Severity::Error)
            .expect("an error");
        first
            .text
            .split_once("\nhelp: ")
            .map(|(_, h)| h.to_string())
    }

    #[test]
    fn a_small_type_names_both_ways_out() {
        let cases = [
            (
                "    const a: Int8 = 127\n    const b: Int8 = 127\n    print(a + b)",
                "for 254, widen first: Int(a) + Int(b); to wrap: a.wrapping_add(b)",
            ),
            (
                "    const a: Int8 = -100\n    print(100 - a)",
                "for 200, widen first: 100 - Int(a); to wrap: Int8(100).wrapping_sub(a)",
            ),
            (
                "    const a: UInt8 = 200\n    print(2 * a)",
                "for 400, widen first: 2 * Int(a); to wrap: a.wrapping_mul(2)",
            ),
            (
                "    const a: Int16 = 400\n    const b: Int16 = 200\n    print(a * (b - 100))",
                "for 40000, widen first: Int(a) * Int(b - 100); to wrap: a.wrapping_mul(b - 100)",
            ),
            (
                "    let count: Int8 = 120\n    count += 8\n    print(count)",
                "for 128, declare `count` with a wider type; to wrap: count = count.wrapping_add(8)",
            ),
            (
                "    let low: Int8 = -100\n    for _ in 0..3 {\n        low -= 20\n    }\n    print(low)",
                "for -140, declare `low` with a wider type; to wrap: low = low.wrapping_sub(20)",
            ),
        ];
        for (body, want) in cases {
            assert_eq!(help(body).as_deref(), Some(want), "{body}");
        }
    }

    /// The help line shows under the code it is about.
    #[test]
    fn the_help_line_shows_under_the_code() {
        let src = "fn main() {\n    const a: Int8 = 127\n    print(a + a)\n}\n";
        let shown = kavach_driver::compile(src, "t.kav", Default::default()).render();
        assert!(shown.starts_with("error: this always panics: integer overflow: 127 + 127 doesn't fit in Int8 (spec 1.8)\n"), "{shown}");
        let help = "help: for 254, widen first: Int(a) + Int(a); to wrap: a.wrapping_add(a)";
        assert!(shown.contains(&format!("^^^^^\n{help}\n\n")), "{shown}");
    }

    /// `Int` itself has no wider type to name, so no help line.
    #[test]
    fn int_gets_no_help_line() {
        assert_eq!(
            help("    const a = 9223372036854775807\n    print(a + 1)"),
            None
        );
    }
}
