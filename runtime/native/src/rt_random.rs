// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Random bits for `std.random` in native code (spec 14.15), from
//! `kavach_runtime_net::random`. A module and entry of its own, so only a
//! program that uses random numbers links the generator (decision 0024).

use kavach_kir::RtOp;
use kavach_runtime_net::random;

crate::rt_data::entry!(kv_rt_random, |op, a, b, c, code, _line| {
    match op {
        // A `random.seeded` block's numbers first (spec 14.15).
        RtOp::RandomBits => crate::rt_scopes::seeded_bits().unwrap_or_else(random::bits) as i64,
        // `n` numbers at once for `tensor.random_int` (spec 14.33), drawn
        // as `random.int` draws them, from the same bits.
        RtOp::RandomInts => {
            let n = a.max(0) as usize;
            // Asking for a seeded block draws its first bits: none then.
            let seeded = (n > 0 && c > b).then(crate::rt_scopes::seeded_bits);
            let drawn = match seeded.flatten() {
                None => random::ints(n, b, c, random::many_bits()),
                Some(first) => {
                    let mut first = Some(first);
                    let next = move || {
                        first
                            .take()
                            .or_else(crate::rt_scopes::seeded_bits)
                            .unwrap_or(0)
                    };
                    random::ints(n, b, c, next)
                }
            };
            let slots = drawn.into_iter().map(|v| v as u64).collect();
            crate::heap::alloc_slots(crate::rt::type_of(code), 0, slots)
        }
        _ => unreachable!("not a random op: {op:?}"),
    }
});
