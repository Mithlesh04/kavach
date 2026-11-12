//! `std.base64` in native code (spec 14.20), from
//! `kavach_runtime_core::base64`, as the interpreter does. An entry of its
//! own, so a program that doesn't use it links none of it (decision 0024).

use crate::heap::release;
use crate::rt::type_of;
use crate::rt_bytes::{new_bytes, with_bytes};
use crate::rt_errors::set_error;
use crate::shapes::{new_str, str_of};
use kavach_kir::RtOp;
use kavach_runtime_core::base64::{self, Base64Fail};

/// Leaves `e` as a `Base64Error` for `kv_take_error`.
fn fail(e: Base64Fail) -> i64 {
    let Some(ty) = crate::shapes::find_union("Base64Error", "BadCharacter") else {
        return 0;
    };
    let error = match e {
        Base64Fail::BadCharacter(at) => {
            crate::rt_json::new_variant(ty, "BadCharacter", &[at as u64])
        }
        Base64Fail::BadLength => crate::rt_json::new_variant(ty, "BadLength", &[]),
        Base64Fail::BadPadding => crate::rt_json::new_variant(ty, "BadPadding", &[]),
        Base64Fail::ExtraBits => crate::rt_json::new_variant(ty, "ExtraBits", &[]),
    };
    set_error(error);
    0
}

crate::rt_data::entry!(kv_rt_base64, |op, a, b, c, code, _line| {
    let ty = type_of(code);
    match op {
        // `a` is the data (owned), `b` whether URL-safe, `c` whether padded.
        RtOp::Base64Encode => {
            let data = with_bytes(a, <[u8]>::to_vec);
            release(a);
            new_str(base64::encode(&data, b != 0, c != 0), ty)
        }
        // `a` is the text (owned), `b` whether URL-safe.
        _ => {
            let text = str_of(a);
            release(a);
            match base64::decode(&text, b != 0) {
                Ok(data) => new_bytes(data, ty),
                Err(e) => fail(e),
            }
        }
    }
});
