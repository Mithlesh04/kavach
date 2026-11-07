//! Serial ports in native code (spec 14.18, decision 0068), from
//! `kavach_runtime_core::serial`, by the port's number. A module and entry
//! of its own, so only a program that uses them links `serial2` (decision
//! 0024).

use crate::heap::{alloc_slots, new_str, release, shape, str_of};
use crate::rt::type_of;
use crate::rt_errors::set_error;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_core::serial::{self, DevError};

/// Leaves `e` as a `DeviceError` for `kv_take_error`.
fn fail(e: DevError) -> i64 {
    let Some(ty) = crate::shapes::find_union("DeviceError", "NotFound") else {
        return 0;
    };
    let text = |s: &str| crate::shapes::new_text(s, crate::heap::first_of(&Shape::Str)) as u64;
    let error = match e {
        DevError::NotFound(path) => crate::rt_json::new_variant(ty, "NotFound", &[text(&path)]),
        DevError::Denied(path) => crate::rt_json::new_variant(ty, "Denied", &[text(&path)]),
        DevError::Failed(path, why) => {
            crate::rt_json::new_variant(ty, "Failed", &[text(&path), text(&why)])
        }
    };
    set_error(error);
    0
}

crate::rt_data::entry!(kv_rt_serial, |op, a, b, _c, code, _line| {
    match op {
        RtOp::SerialPorts => {
            let item = match shape(type_of(code)) {
                Shape::List(t) => *t,
                _ => Type::I64,
            };
            let names = serial::ports()
                .into_iter()
                .map(|n| new_str(n, item) as u64)
                .collect();
            alloc_slots(type_of(code), 0, names)
        }
        // `a` is the path (owned), `b` the baud rate.
        RtOp::SerialOpen => {
            let path = str_of(a);
            release(a);
            serial::open(&path, b).unwrap_or_else(fail)
        }
        RtOp::SerialRead => match serial::read(a, b) {
            Ok(bytes) => crate::rt_bytes::new_bytes(bytes, type_of(code)),
            Err(e) => fail(e),
        },
        // `b` is the data (owned).
        RtOp::SerialWrite => {
            let data = crate::rt_bytes::with_bytes(b, <[u8]>::to_vec);
            release(b);
            serial::write(a, &data).map_or_else(fail, |()| 0)
        }
        RtOp::SerialClose => {
            serial::close(a);
            0
        }
        _ => unreachable!("not a serial op: {op:?}"),
    }
});
