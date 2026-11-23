// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.font` in native code (spec 14.31, decision 0121), from
//! `kavach_runtime_font`, as the interpreter does. An entry of its own,
//! so only a program with fonts links the shaper (decision 0024).

use crate::heap::{alloc_slots, release, slots};
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

/// The texts of a list of the op's result type.
fn texts(all: Vec<String>, code: i64) -> i64 {
    let item = match crate::heap::shape(type_of(code)) {
        Shape::List(t) => *t,
        _ => kavach_kir::Type::I64,
    };
    let words = all
        .into_iter()
        .map(|t| crate::shapes::new_str(t, item) as u64)
        .collect();
    alloc_slots(type_of(code), 0, words)
}

// `a` is the font's bytes (owned), an atlas's number, or a text (owned);
// `b` the text (owned); `c` the size (a `Float`'s bits) to shape at, or
// for `FontAtlas` the list (owned) of the size, 1 when exact, and the
// atlas to add to (-1 for none).
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
        RtOp::FontShape => {
            let (text, size) = (str_of(b), f64::from_bits(c as u64));
            release(b);
            let made = crate::rt_bytes::with_bytes(a, |data| {
                let parts = font::shape_flat(data, &text, size);
                numbers(parts.iter().map(|v| v.to_bits()).collect(), code)
            });
            release(a);
            made
        }
        RtOp::FontAtlas => {
            let text = str_of(b);
            release(b);
            let s: Vec<f64> = slots(c).iter().map(|v| f64::from_bits(*v)).collect();
            release(c);
            let part = |k: usize| s.get(k).copied().unwrap_or(-1.0);
            let made = crate::rt_bytes::with_bytes(a, |data| {
                font::atlas_kept(data, &text, part(0), part(1) > 0.5, part(2) as i64)
            });
            release(a);
            made.unwrap_or_else(|why| crate::rt::panic_with(&why, line))
        }
        // `a` the atlas's pixels (owned), `b` its layout (owned), `c` its
        // size's bits.
        RtOp::FontAtlasBase => {
            let layout: Vec<i64> = slots(b).iter().map(|v| *v as i64).collect();
            release(b);
            let size = f64::from_bits(c as u64);
            let n = crate::rt_bytes::with_bytes(a, |px| font::keep_base(px, &layout, size));
            release(a);
            n
        }
        RtOp::FontInstalled => texts(font::installed_flat(), code),
        RtOp::FontFallback => {
            let text = str_of(a);
            release(a);
            font::fallback(&text).map_or(-1, |k| k as i64)
        }
        RtOp::FontFile => {
            let path = str_of(a);
            release(a);
            crate::rt_bytes::new_bytes(font::read_installed(&path), type_of(code))
        }
        RtOp::FontAtlasBoxes => numbers(
            font::atlas_boxes(a).iter().map(|v| *v as u64).collect(),
            code,
        ),
        _ => crate::rt_bytes::new_bytes(font::take_atlas_pixels(a), type_of(code)),
    }
});
