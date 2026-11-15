//! `std.image` in native code (spec 14.29, decision 0117), from
//! `kavach_runtime_image`, as the interpreter does. An entry of its own,
//! so only a program with images links the decoders (decision 0024).

use crate::heap::{alloc_slots, release};
use crate::rt::type_of;
use crate::rt_errors::set_error;
use kavach_kir::{RtOp, Shape};
use kavach_runtime_image as image;

/// Leaves `why` as an `ImageError.BadImage` for `kv_take_error`.
fn fail(why: &str) -> i64 {
    if let Some(ty) = crate::shapes::find_union("ImageError", "BadImage") {
        let text = crate::shapes::new_text(why, crate::heap::first_of(&Shape::Str));
        set_error(crate::rt_json::new_variant(ty, "BadImage", &[text as u64]));
    }
    0
}

// `a` is the file's bytes (owned) for `ImageDecode`, a picture's number,
// or the width for `ImagePng`, with `b` the height and `c` the pixels
// (owned).
crate::rt_data::entry!(kv_rt_image, |op, a, b, c, code, line| {
    match op {
        RtOp::ImageDecode => {
            let made = crate::rt_bytes::with_bytes(a, image::decode_kept);
            release(a);
            made.unwrap_or_else(|why| fail(&why))
        }
        RtOp::ImageSize => {
            let (w, h) = image::size(a);
            alloc_slots(type_of(code), 0, vec![w as u64, h as u64])
        }
        RtOp::ImagePixels => crate::rt_bytes::new_bytes(image::take_pixels(a), type_of(code)),
        _ => {
            let fit = |v: i64| u32::try_from(v).unwrap_or(0);
            let made = crate::rt_bytes::with_bytes(c, |px| image::png(fit(a), fit(b), px));
            release(c);
            match made {
                Ok(file) => crate::rt_bytes::new_bytes(file, type_of(code)),
                Err(why) => crate::rt::panic_with(&why, line),
            }
        }
    }
});
