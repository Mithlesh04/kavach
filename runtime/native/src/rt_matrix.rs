// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.matrix` in native code (spec 2.17, decision 0111): the lists'
//! items read as the numbers they are, the work done by
//! `kavach_runtime_matrix`, and the result made a list of type `code`. The
//! interpreter does the same (`compiler/interp/src/std_matrix.rs`). An
//! entry of its own, so only a program with matrices links it.

use crate::heap::{alloc_slots, release, shape};
use crate::rt::{panic_with, type_of};
use crate::shapes::items_of;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_matrix::{self as mx, By, Num, Op, Overflow};

/// The arguments of one operation, as slots: the first list, the second
/// list or a number, and the sizes.
struct Args {
    a: Vec<u64>,
    b: Vec<u64>,
    sizes: Vec<usize>,
}

/// Does `op` on numbers of type `T`, read from slots with `from` and
/// written back with `to`.
fn work<T: Num>(
    op: RtOp,
    args: &Args,
    from: fn(u64) -> T,
    to: fn(T) -> u64,
) -> Result<Vec<u64>, Overflow> {
    let a: Vec<T> = args.a.iter().map(|s| from(*s)).collect();
    let b: Vec<T> = args.b.iter().map(|s| from(*s)).collect();
    let size = |i: usize| args.sizes.get(i).copied().unwrap_or(0);
    let out = match op {
        RtOp::MatrixProduct => mx::product(&a, &b, (size(0), size(1), size(2)), size(3) != 0)?,
        RtOp::MatrixZip => {
            let op = [Op::Add, Op::Sub, Op::Mul][size(0).min(2)];
            mx::zip(op, &a, &b)?
        }
        RtOp::MatrixScale => mx::scale(&a, b.first().copied().unwrap_or_default())?,
        RtOp::MatrixNegate => mx::negate(&a)?,
        RtOp::MatrixTranspose => mx::transpose(&a, size(0), size(1)),
        _ => {
            let by = [By::All, By::Rows, By::Cols][size(2).min(2)];
            mx::sums(&a, size(0), size(1), by)?
        }
    };
    Ok(out.into_iter().map(to).collect())
}

/// The sizes in list `p`, a `[Int]`.
fn sizes(p: i64) -> Vec<usize> {
    items_of(p)
        .iter()
        .map(|s| (*s as i64).max(0) as usize)
        .collect()
}

crate::rt_data::entry!(kv_rt_matrix, |op, a, b, c, code, line| {
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
    let args = match op {
        RtOp::MatrixProduct => Args {
            a: list(a),
            b: list(b),
            sizes: sizes(c),
        },
        RtOp::MatrixZip => Args {
            a: list(a),
            b: list(b),
            sizes: vec![c.max(0) as usize],
        },
        RtOp::MatrixScale => Args {
            a: list(a),
            b: vec![b as u64],
            sizes: Vec::new(),
        },
        RtOp::MatrixNegate => Args {
            a: list(a),
            b: Vec::new(),
            sizes: Vec::new(),
        },
        _ => Args {
            a: list(a),
            b: Vec::new(),
            sizes: sizes(b),
        },
    };
    if matches!(
        op,
        RtOp::MatrixProduct | RtOp::MatrixTranspose | RtOp::MatrixSums
    ) {
        release(if op == RtOp::MatrixProduct { c } else { b });
    }
    let done = match item {
        Type::F64 => work(op, &args, f64::from_bits, f64::to_bits),
        Type::F32 => work(
            op,
            &args,
            |s| f32::from_bits(s as u32),
            |v| v.to_bits() as u64,
        ),
        Type::I64 => work(op, &args, |s| s as i64, |v| v as u64),
        Type::I32 => work(op, &args, |s| s as i32, |v| v as i64 as u64),
        Type::I16 => work(op, &args, |s| s as i16, |v| v as i64 as u64),
        Type::I8 => work(op, &args, |s| s as i8, |v| v as i64 as u64),
        Type::U64 => work(op, &args, |s| s, |v| v),
        Type::U32 => work(op, &args, |s| s as u32, u64::from),
        Type::U16 => work(op, &args, |s| s as u16, u64::from),
        Type::U8 => work(op, &args, |s| s as u8, u64::from),
        _ => panic_with(
            "a matrix holds numbers: `Float`, `Float32` or a whole-number type (spec 2.17)",
            line,
        ),
    };
    match done {
        Ok(slots) => alloc_slots(ty, 0, slots),
        Err(Overflow) => panic_with(
            "integer overflow in a matrix: a result doesn't fit (spec 2.17)",
            line,
        ),
    }
});
