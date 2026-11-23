// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Whole numbers with their own limits (spec 2.1): `Int<bits: 24>`,
//! `UInt<bits: 12>`, `Int<range: 1..=65535>` (`RtOp::IntWhole`). Their
//! values are `Int`s; each result is worked out exactly and checked
//! against the type's limits. The messages are the interpreter's
//! (`compiler/interp/src/ops.rs`, `builtins.rs`).

use crate::rt::{panic_with, range, type_of};
use crate::rt_data::optional;
use crate::shapes::shape;
use kavach_kir::{Shape, Type, WHOLE_OPS};

/// The limits and the name of the type whose code is `code`.
fn limits(code: i64) -> (i128, i128, String) {
    let t = type_of(code);
    match shape(t) {
        Shape::Whole { lo, hi, bits: 0 } if t.is_ref() => (
            i128::from(*lo),
            i128::from(*hi),
            format!("Int<range: {lo}..={hi}>"),
        ),
        Shape::Whole { lo, hi, bits } if t.is_ref() => {
            let sign = if *lo < 0 { "" } else { "U" };
            (
                i128::from(*lo),
                i128::from(*hi),
                format!("{sign}Int<bits: {bits}>"),
            )
        }
        _ => {
            let (lo, hi) = range(t);
            (lo, hi, t.kavach_name().to_string())
        }
    }
}

/// A slot holding a number of type `t`, as an exact whole number; a
/// float or `Decimal` is cut to its whole part, and `None` when it has
/// none or is past `i128`.
fn exact(slot: i64, t: Type) -> Option<i128> {
    match t {
        // A `BigInt` or `Decimal`, held by reference.
        t if t.is_ref() => {
            let n = crate::rt_bigint::big_of(slot)
                .unwrap_or_else(|| crate::rt_big_decimal::any_decimal_of(slot).trunc());
            i128::try_from(&n).ok()
        }
        Type::F64 => {
            let f = f64::from_bits(slot as u64);
            f.is_finite().then(|| f.trunc() as i128)
        }
        t if t.is_signed() || t == Type::I64 => Some(i128::from(slot)),
        _ => Some(i128::from(slot as u64)),
    }
}

// Numbers with bits or a range: an entry of their own, so a program
// without them doesn't link their checks (decision 0024).
crate::rt_data::entry!(kv_rt_whole, |_op, a, b, c, code, line| {
    whole_op(a, b, c, code, line)
});

/// `IntWhole`: `c` holds the op's place in `WHOLE_OPS` (above bit 32)
/// and the type's code; `code` is the result's type.
pub fn whole_op(a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    let op = WHOLE_OPS.get((c >> 32) as usize).copied().unwrap_or("+");
    let (lo, hi, name) = limits(c & 0xffff_ffff);
    let fits = |n: i128| (lo..=hi).contains(&n);
    let (x, y) = (i128::from(a), i128::from(b));
    let overflow = |text: String| -> ! {
        panic_with(
            &format!("integer overflow: {text} doesn't fit in {name}"),
            line,
        )
    };
    let result = match op {
        "fits" => {
            let from = type_of(b);
            let n = exact(a, from);
            // A `BigInt` or `Decimal` comes owned, as every reference does.
            if from.is_ref() {
                crate::heap::release(a);
            }
            return i64::from(n.is_some_and(fits));
        }
        "neg" if fits(-x) => -x,
        "neg" => overflow(format!("-({x})")),
        "~" if fits(!x) => !x,
        "~" => !x & hi,
        _ if op.starts_with("wrapping_") || op.starts_with("checked_") => {
            let exact = match op {
                "wrapping_add" | "checked_add" => x.checked_add(y),
                "wrapping_sub" | "checked_sub" => x.checked_sub(y),
                "wrapping_mul" | "checked_mul" => x.checked_mul(y),
                _ if y == 0 => None,
                _ => x.checked_div(y),
            };
            if op.starts_with("wrapping_") {
                let wrapped = exact.map_or(0, |n| (n - lo).rem_euclid(hi - lo + 1) + lo);
                return wrapped as i64;
            }
            return optional(exact.filter(|n| fits(*n)).map(|n| n as i64), code);
        }
        _ => {
            let done = match op {
                "+" => x.checked_add(y),
                "-" => x.checked_sub(y),
                "*" => x.checked_mul(y),
                "/" | "%" if y == 0 => panic_with("division by zero", line),
                "/" => x.checked_div(y),
                "%" => x.checked_rem(y),
                "**" if y < 0 => panic_with(
                    &format!(
                        "{x} ** {y}: a whole number to a negative power isn't whole; use floats"
                    ),
                    line,
                ),
                "**" => kavach_runtime_core::int_pow(x, y),
                "&" => Some(x & y),
                "|" => Some(x | y),
                "^" => Some(x ^ y),
                // By 0 up to the width less one, as on the other types.
                _ => {
                    let width = (hi + 1).trailing_zeros() + u32::from(lo < 0);
                    match u32::try_from(y) {
                        Ok(s) if s < width && op == "<<" => Some(x << s),
                        Ok(s) if s < width => Some(x >> s),
                        _ => None,
                    }
                }
            };
            match done.filter(|n| fits(*n)) {
                Some(n) => n,
                None => overflow(format!("{x} {op} {y}")),
            }
        }
    };
    result as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_and_unsigned_slots_read_exactly() {
        let cases: [(i64, Type, Option<i128>); 5] = [
            (2.9f64.to_bits() as i64, Type::F64, Some(2)),
            ((-2.9f64).to_bits() as i64, Type::F64, Some(-2)),
            (f64::NAN.to_bits() as i64, Type::F64, None),
            (-1, Type::U64, Some(u64::MAX.into())),
            (-1, Type::I8, Some(-1)),
        ];
        for (slot, t, want) in cases {
            assert_eq!(exact(slot, t), want, "{slot} as {t:?}");
        }
    }
}
