//! `std.compress` in native code (spec 14.21), from
//! `kavach_runtime_core::compress`, as the interpreter does. An entry of
//! its own, so a program that doesn't use it links none of the four
//! crates (decision 0024).

use crate::heap::release;
use crate::rt::type_of;
use crate::rt_bytes::{new_bytes, with_bytes};
use crate::rt_errors::set_error;
use kavach_kir::RtOp;
use kavach_runtime_core::compress::{self, Fail};

/// Leaves `e` as a `CompressError` for `kv_take_error`.
fn fail(e: Fail, most: i64) -> i64 {
    let Some(ty) = crate::shapes::find_union("CompressError", "TooLarge") else {
        return 0;
    };
    let error = match e {
        Fail::TooLarge => crate::rt_json::new_variant(ty, "TooLarge", &[most as u64]),
        Fail::Corrupt(why) => {
            let text =
                crate::shapes::new_text(&why, crate::heap::first_of(&kavach_kir::Shape::Str));
            crate::rt_json::new_variant(ty, "Corrupt", &[text as u64])
        }
    };
    set_error(error);
    0
}

// `a` is the data (owned), `b` the format's number, `c` the level, or
// the most bytes to give back.
crate::rt_data::entry!(kv_rt_compress, |op, a, b, c, code, _line| {
    let ty = type_of(code);
    let data = with_bytes(a, <[u8]>::to_vec);
    release(a);
    match op {
        RtOp::CompressEncode => new_bytes(compress::encode(&data, b, c), ty),
        _ => match compress::decode(&data, b, c.max(0) as usize) {
            Ok(out) => new_bytes(out, ty),
            Err(e) => fail(e, c),
        },
    }
});
