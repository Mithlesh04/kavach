// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.font` in native code (spec 14.31, decision 0121), from
//! `kavach_runtime_font`, as the interpreter does. An entry of its own,
//! so only a program with fonts links the shaper (decision 0024).

use crate::heap::{alloc_slots, release};
use crate::rt::type_of;
use crate::rt_errors::set_error;
use crate::shapes::str_of;
use kavach_kir::{RtOp, Shape};
use kavach_runtime_font as font;

/// Leaves `why` as a `FontError.BadFont` for `kv_take_error`.
fn fail(why: &str) -> i64 {
    if let Some(ty) = crate::shapes::find_union("FontError", "BadFont") {
        let text = crate::shapes::new_text(why, crate::heap::first_of(&Shape::Str));
        set_error(crate::rt_json::new_variant(ty, "BadFont", &[text as u64]));
    }
    0
}

/// A list of numbers of the op's result type, each kept as its bits.
fn numbers(words: Vec<u64>, code: i64) -> i64 {
    alloc_slots(type_of(code), 0, words)
}

// `a` is the font's bytes (owned), or an atlas's number; `b` the text
// (owned) and `c` the size (a `Float`'s bits) to shape or draw it at.
crate::rt_data::entry!(kv_rt_font, |op, a, b, c, code, line| {
    match op {
        RtOp::FontMetrics => {
            let read = crate::rt_bytes::with_bytes(a, font::metrics);
            release(a);
            match read {
                Ok(m) => numbers(m.iter().map(|v| v.to_bits()).collect(), code),
                Err(why) => fail(&why),
            }
        }
        RtOp::FontShape | RtOp::FontAtlas => {
            let (text, size) = (str_of(b), f64::from_bits(c as u64));
            release(b);
            let made = crate::rt_bytes::with_bytes(a, |data| {
                if op == RtOp::FontShape {
                    let parts = font::shape_flat(data, &text, size);
                    Ok(numbers(parts.iter().map(|v| v.to_bits()).collect(), code))
                } else {
                    font::atlas_kept(data, &text, size)
                }
            });
            release(a);
            made.unwrap_or_else(|why| crate::rt::panic_with(&why, line))
        }
        RtOp::FontAtlasBoxes => numbers(
            font::atlas_boxes(a).iter().map(|v| *v as u64).collect(),
            code,
        ),
        _ => crate::rt_bytes::new_bytes(font::take_atlas_pixels(a), type_of(code)),
    }
});
