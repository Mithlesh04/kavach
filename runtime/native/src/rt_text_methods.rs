// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Text methods in native code (spec 14.2), from
//! `kavach_runtime_core::text_methods`, as the interpreter calls them.
//! An entry of its own, as each feature group has (decision 0024).

use crate::heap::{new_str, release, str_of, with_str};
use crate::rt::type_of;
use crate::rt_data::optional;
use crate::rt_text::{text_item, text_words};
use kavach_kir::RtOp;
use kavach_runtime_core::text_methods as tm;

/// The text `p` (owned).
fn take(p: i64) -> String {
    let t = str_of(p);
    release(p);
    t
}

// `a` is the text (owned); `b` a `String` (owned) or an `Int`; `c` the
// fill `Char` of padding.
crate::rt_data::entry!(kv_rt_text_methods, |op, a, b, c, code, line| {
    let result = type_of(code);
    let s = take(a);
    match op {
        RtOp::TextTrimStart => new_str(s.trim_start().to_string(), result),
        RtOp::TextTrimEnd => new_str(s.trim_end().to_string(), result),
        RtOp::TextCapitalize => new_str(tm::capitalize(&s), result),
        RtOp::TextTitle => new_str(tm::title(&s), result),
        RtOp::TextIsBlank => i64::from(tm::is_blank(&s)),
        RtOp::TextWords => {
            let words = text_words(tm::words(&s).into_iter(), 0, text_item(code));
            crate::heap::alloc_words(result, words)
        }
        RtOp::TextLastIndexOf => {
            let at = with_str(b, |part| tm::last_index_of(&s, part));
            release(b);
            optional(at.map(|n| n as i64), code)
        }
        RtOp::TextCount => {
            let n = with_str(b, |part| tm::count(&s, part));
            release(b);
            n as i64
        }
        RtOp::TextRemovePrefix => new_str(tm::remove_prefix(&s, &take(b)).to_string(), result),
        RtOp::TextRemoveSuffix => new_str(tm::remove_suffix(&s, &take(b)).to_string(), result),
        // A negative count counts as 0 (spec 14.2).
        RtOp::TextTruncate => new_str(tm::truncate(&s, b.max(0) as usize).to_string(), result),
        _ => {
            let left = op == RtOp::TextPadLeft;
            let b = b.max(0);
            if b > tm::MOST_WIDTH {
                let name = if left { "pad_left" } else { "pad_right" };
                let why = format!("`{name}` takes a width from 0 to {}", tm::MOST_WIDTH);
                crate::rt::panic_with(&why, line)
            } else {
                let fill = char::from_u32(c as u32).unwrap_or(' ');
                new_str(tm::pad(&s, b as usize, fill, left), result)
            }
        }
    }
});
