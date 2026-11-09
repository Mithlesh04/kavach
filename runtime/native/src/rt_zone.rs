//! Time zones in native code (spec 14.4, decision 0064): a zone's offset
//! from `kavach_runtime_core::zones`. A module and entry of its own, so
//! only a program that uses zones links `jiff` and the zone data
//! (decision 0024).

use crate::heap::release;
use crate::shapes::with_str;
use kavach_kir::RtOp;

crate::rt_data::entry!(kv_rt_zone, |op, a, b, _c, code, _line| {
    match op {
        // `a` is the zone's name (owned), `b` the moment.
        RtOp::ZoneOffset => {
            let name = with_str(a, str::to_string);
            release(a);
            let seconds = kavach_runtime_core::zones::offset_seconds(&name, b);
            crate::rt_data::optional(seconds, code)
        }
        _ => unreachable!("not a zone op: {op:?}"),
    }
});
