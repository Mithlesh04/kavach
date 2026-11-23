// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `sum(f)` of floats in native code (spec 14.2, decision 0135): an
//! exact sum, rounded once, from `kavach_runtime_core::math::exact`, as
//! the interpreter adds. The loop holds the sum's address in a local;
//! `ExactEnd` frees it. An entry of its own, so only a program with a
//! float sum links it.

use crate::heap::{release, slots};
use crate::rt::type_of;
use kavach_kir::{RtOp, Type};
use kavach_runtime_core::math::exact::ExactSum;

// `a` is the sum's address (from `ExactNew`), `b` the `Float` to add, as
// its bits, or a list of them (owned); `code` is `ExactEnd`'s result type.
crate::rt_data::entry!(kv_rt_exact, |op, a, b, _c, code, _line| {
    match op {
        RtOp::ExactNew => Box::into_raw(Box::new(ExactSum::new())) as i64,
        RtOp::ExactAdd => {
            // SAFETY: `a` is the address `ExactNew` gave this loop, which
            // only its own `ExactEnd` frees, after its last `ExactAdd`.
            let sum = unsafe { &mut *(a as *mut ExactSum) };
            sum.add(f64::from_bits(b as u64));
            0
        }
        RtOp::ExactAddList => {
            // SAFETY: as for `ExactAdd`.
            let sum = unsafe { &mut *(a as *mut ExactSum) };
            // A list of `Float`s keeps one in each slot, unless packed.
            if crate::flat::packed(crate::heap::obj(b).ty()) {
                crate::shapes::items_of(b)
                    .iter()
                    .for_each(|&x| sum.add(f64::from_bits(x)));
            } else {
                slots(b).iter().for_each(|&x| sum.add(f64::from_bits(x)));
            }
            release(b);
            0
        }
        _ => {
            // SAFETY: as above; this is the loop's one `ExactEnd`.
            let sum = unsafe { Box::from_raw(a as *mut ExactSum) };
            match type_of(code) {
                Type::F32 => crate::display::f32_slot(sum.value32()),
                _ => sum.value().to_bits() as i64,
            }
        }
    }
});
