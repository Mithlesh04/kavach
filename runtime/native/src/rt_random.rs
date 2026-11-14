//! Random bits for `std.random` in native code (spec 14.15), from
//! `kavach_runtime_net::random`. A module and entry of its own, so only a
//! program that uses random numbers links the generator (decision 0024).

use kavach_kir::RtOp;

crate::rt_data::entry!(kv_rt_random, |op, _a, _b, _c, _code, _line| {
    match op {
        // A `random.seeded` block's numbers first (spec 14.15).
        RtOp::RandomBits => {
            crate::rt_scopes::seeded_bits().unwrap_or_else(kavach_runtime_net::random::bits) as i64
        }
        _ => unreachable!("not a random op: {op:?}"),
    }
});
