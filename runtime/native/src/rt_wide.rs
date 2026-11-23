// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Whole numbers of a `bits` type wider than 64 (spec 2.1):
//! `Int<bits: 128>`, `UInt<bits: 256>`. Their values are `BigInt`s;
//! `RtOp::IntWhole` sends their operations here, which work each out
//! through `kavach_runtime_core::wide`, as the interpreter does.

use crate::heap::release;
use crate::rt::{panic_with, type_code, type_of};
use crate::shapes::shape;
use kavach_kir::{Shape, Type};
use kavach_runtime_core::BigInt;
use kavach_runtime_core::wide::Wide;

/// The number in `slot` of type `t`, as a `BigInt`, cut to a whole one
/// as `Int(x)` does; `None` for a float that isn't a number. A `BigInt`
/// or `Decimal` comes owned and is released.
fn big_of_slot(slot: i64, t: Type) -> Option<BigInt> {
    match t {
        t if t.is_ref() => Some(owned_big(slot)),
        Type::F64 => {
            let f = f64::from_bits(slot as u64);
            f.is_finite()
                .then(|| format!("{:.0}", f.trunc()).parse().ok())
                .flatten()
        }
        t if t.is_signed() => Some(BigInt::from(slot)),
        _ => Some(BigInt::from(slot as u64)),
    }
}

/// The `BigInt` or `Decimal` `p` (owned, released), as a whole number.
fn owned_big(p: i64) -> BigInt {
    let n = crate::rt_bigint::big_of(p)
        .unwrap_or_else(|| crate::rt_big_decimal::any_decimal_of(p).trunc());
    release(p);
    n
}

/// `IntWhole` on a wide type `w`: `a op b`, each a `BigInt` (owned),
/// giving a `BigInt` of type `code`, or for `checked_*` an optional one.
/// `fits` takes any number `a` of the type whose code is `b`; `neg` and
/// `~` read only `a`.
pub(crate) fn wide_op(op: &str, w: Wide, a: i64, b: i64, code: i64, line: i32) -> i64 {
    if op == "fits" {
        let n = big_of_slot(a, type_of(b));
        return i64::from(n.is_some_and(|n| w.fits(&n)));
    }
    let x = owned_big(a);
    let y = match op {
        "neg" | "~" => BigInt::default(),
        _ => owned_big(b),
    };
    // A `checked_*` gives an optional `BigInt`: the value, or 0 for none.
    let big = match shape(type_of(code)) {
        Shape::Optional(inner) => type_code(*inner),
        _ => code,
    };
    match w.op(op, &x, &y) {
        Ok(Some(n)) => crate::rt_bigint::new_big(n, big),
        Ok(None) => 0,
        Err(why) => panic_with(&why, line),
    }
}
