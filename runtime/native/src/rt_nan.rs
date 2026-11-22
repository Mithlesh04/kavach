// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The NaN watch (spec 5.9): a float operation of a program built with it
//! is worked out here, and panics when it gives `NaN`. An entry of its
//! own, so a program without the watch doesn't link it (decision 0024).

use crate::rt::panic_with;
use kavach_kir::RtOp;
use kavach_runtime_core::math;

crate::rt_data::entry!(kv_rt_nan, |_op, a, b, c, code, line| {
    watched(a, b, c, code, line)
});

/// Operation `c` of `a` and `b` (bits; `compiler/kir/src/nan_watch.rs`
/// says how `c` names it), the same as native code works it out.
fn watched(a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    if c >= 0 {
        let op = RtOp::ALL[c as usize];
        let got = crate::rt_std::std_op(op, a, b, code, line);
        if f64::from_bits(got as u64).is_nan() {
            let name = format!("{op:?}")["Math".len()..].to_lowercase();
            let message = math::nan_watch_call(&name, float(a), float(b));
            panic_with(&message, line);
        }
        return got;
    }
    let (narrow, place) = (c <= -11, if c <= -11 { -11 - c } else { -1 - c });
    let symbol = ["+", "-", "*", "/", "%"][place as usize];
    let (got, x, y) = if narrow {
        let (x, y) = (f32::from_bits(a as u32), f32::from_bits(b as u32));
        let got = match place {
            0 => x + y,
            1 => x - y,
            2 => x * y,
            3 => x / y,
            _ => x % y,
        };
        (f64::from(got), f64::from(x), f64::from(y))
    } else {
        let (x, y) = (float(a), float(b));
        let got = match place {
            0 => x + y,
            1 => x - y,
            2 => x * y,
            3 => x / y,
            _ => x % y,
        };
        (got, x, y)
    };
    if got.is_nan() {
        panic_with(&math::nan_watch_op(x, symbol, y, narrow), line);
    }
    if narrow {
        i64::from((got as f32).to_bits())
    } else {
        got.to_bits() as i64
    }
}

fn float(slot: i64) -> f64 {
    f64::from_bits(slot as u64)
}
