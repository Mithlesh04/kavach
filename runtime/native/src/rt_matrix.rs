// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.tensor`'s `a @ b` in native code (spec 14.33, decisions 0111 and
//! 0136): the lists' items read as the numbers they are, the product
//! worked out by `kavach_runtime_matrix`, and the result made a list of
//! type `code`. The interpreter does the same
//! (`compiler/interp/src/std_matrix.rs`). An entry of its own, so only a
//! program with products links it.

use crate::heap::{alloc_slots, release, shape};
use crate::rt::{panic_with, type_of};
use crate::shapes::items_of;
use kavach_kir::{Shape, Type};
use kavach_runtime_matrix::{self as mx, Num, Overflow};

/// The product's arguments, as slots: the two lists and the sizes.
struct Args {
    a: Vec<u64>,
    b: Vec<u64>,
    sizes: Vec<usize>,
}

/// The product of numbers of type `T`, read from slots with `from` and
/// written back with `to`.
fn work<T: Num>(args: &Args, from: fn(u64) -> T, to: fn(T) -> u64) -> Result<Vec<u64>, Overflow> {
    let a: Vec<T> = args.a.iter().map(|s| from(*s)).collect();
    let b: Vec<T> = args.b.iter().map(|s| from(*s)).collect();
    let size = |i: usize| args.sizes.get(i).copied().unwrap_or(0);
    let out = mx::product(&a, &b, (size(0), size(1), size(2)), size(3) != 0)?;
    Ok(out.into_iter().map(to).collect())
}

/// The sizes in list `p`, a `[Int]`.
fn sizes(p: i64) -> Vec<usize> {
    items_of(p)
        .iter()
        .map(|s| (*s as i64).max(0) as usize)
        .collect()
}

crate::rt_data::entry!(kv_rt_matrix, |_op, a, b, c, code, line| {
    let ty = type_of(code);
    let item = match shape(ty) {
        Shape::List(t) => *t,
        _ => Type::I64,
    };
    let list = |p: i64| {
        let slots = items_of(p).to_vec();
        release(p);
        slots
    };
    let args = Args {
        a: list(a),
        b: list(b),
        sizes: sizes(c),
    };
    release(c);
    let done = match item {
        Type::F64 => work(&args, f64::from_bits, f64::to_bits),
        Type::F32 => work(&args, |s| f32::from_bits(s as u32), |v| v.to_bits() as u64),
        Type::I64 => work(&args, |s| s as i64, |v| v as u64),
        Type::I32 => work(&args, |s| s as i32, |v| v as i64 as u64),
        Type::I16 => work(&args, |s| s as i16, |v| v as i64 as u64),
        Type::I8 => work(&args, |s| s as i8, |v| v as i64 as u64),
        Type::U64 => work(&args, |s| s, |v| v),
        Type::U32 => work(&args, |s| s as u32, u64::from),
        Type::U16 => work(&args, |s| s as u16, u64::from),
        Type::U8 => work(&args, |s| s as u8, u64::from),
        _ => panic_with(
            "`@` works on tensors of `Float`, `Float32` or a whole-number type (spec 14.33)",
            line,
        ),
    };
    match done {
        Ok(slots) => alloc_slots(ty, 0, slots),
        Err(Overflow) => panic_with(
            "integer overflow in `@`: a sum of products doesn't fit (spec 14.33)",
            line,
        ),
    }
});
