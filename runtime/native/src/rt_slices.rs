// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Slices of lists, text and bytes, and a text's characters (spec 2.19).
//! The rules are `kavach_runtime_core::slices`, which the interpreter
//! uses too, so both give the same items and the same panics.

use crate::heap::{items_of, new_str, obj, release, shape};
use crate::rt::{panic_with, type_of};
use crate::shapes::with_str;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_core::slices::{self, Part};

/// The slice in the list `words` of four numbers (`Part::words`), which
/// it lets go.
pub(crate) fn part_of(words: i64) -> Part {
    let w = items_of(words);
    let at = |i: usize| w.get(i).copied().unwrap_or(0) as i64;
    let part = Part::from_words([at(0), at(1), at(2), at(3)]);
    drop(w);
    release(words);
    part
}

fn item_type(list: i64) -> Type {
    match shape(obj(list).ty()) {
        Shape::List(t) => *t,
        _ => Type::I64,
    }
}

/// `xs[a:b:step]` and `xs[a:b:step] = ys`. `a` is the list (owned), `b`
/// the slice's words, `c` the items given.
pub fn list_part(op: RtOp, a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    let part = part_of(b);
    if op == RtOp::ListSetPart {
        let given = crate::heap::len_of(c);
        let at = part
            .set_positions(crate::heap::len_of(a), given)
            .unwrap_or_else(|m| panic_with(&m, line));
        let mut list = a;
        for (j, i) in at.into_iter().enumerate() {
            let item = crate::shapes::item_owned(c, j);
            list = crate::rt_maps::list_set(list, i as i64, item, line);
        }
        release(c);
        return list;
    }
    slice_list(a, &part, code, line)
}

/// `a[part]`, a new list of type `code`; lets go of `a`.
pub(crate) fn slice_list(a: i64, part: &Part, code: i64, line: i32) -> i64 {
    if let Some(out) = flat_part(a, part, code, line) {
        return out;
    }
    let item = item_type(a);
    let slots = items_of(a);
    let picked = slices::pick(&slots, part).unwrap_or_else(|m| panic_with(m, line));
    let out = crate::rt_text::list_of(&picked, item, code);
    drop(slots);
    release(a);
    out
}

/// `s[a:b:step]` and `s[i]`. `a` is the text (owned), `b` the slice's
/// words or the position.
/// A slice of a flat list, whose items are words in place (text, or
/// records laid out flat): only the picked items' words are copied, and
/// what they point to retained. Going through `items_of` made an object
/// of every item of the whole list first: 8 slices of a million short
/// strings took 350 ms, against 33 ms for records (Sakshi's S17).
fn flat_part(a: i64, part: &Part, code: i64, line: i32) -> Option<i64> {
    let ty = obj(a).ty();
    let width = crate::flat::width(ty);
    if width == 0 || crate::flat::width(type_of(code)) != width {
        return None;
    }
    let n = crate::heap::list_len_items_pub(a);
    let words = crate::heap::slots(a);
    let mut out = Vec::new();
    if part.step.unwrap_or(1) == 1 {
        let (start, end, _) = part.bounds(n).unwrap_or_else(|m| panic_with(m, line));
        let (start, end) = (start as usize, end.max(start) as usize);
        out.extend_from_slice(&words[start * width..end * width]);
    } else {
        let at = part.positions(n).unwrap_or_else(|m| panic_with(m, line));
        out.reserve(at.len() * width);
        for i in at {
            out.extend_from_slice(&words[i * width..(i + 1) * width]);
        }
    }
    if crate::flat::is_text(ty) {
        crate::text_items::retain_all(&out);
    } else {
        let item = crate::flat::item_type(ty);
        for (j, w) in out.iter().enumerate() {
            if crate::flat::field_type(item, j % width).is_ref() {
                crate::heap::retain(*w as i64);
            }
        }
    }
    release(a);
    Some(crate::heap::alloc_words(
        type_of(code),
        crate::flat::fit(out, width),
    ))
}

pub fn text_part(op: RtOp, a: i64, b: i64, code: i64, line: i32) -> i64 {
    // The text is read in place, not copied first.
    if op == RtOp::StrAt {
        let c = with_str(a, |s| slices::char_at(s, i128::from(b)));
        release(a);
        return match c {
            Ok(c) => c as i64,
            Err(m) => panic_with(&m, line),
        };
    }
    let part = part_of(b);
    let t = with_str(a, |s| slices::text(s, &part));
    release(a);
    let t = t.unwrap_or_else(|m| panic_with(m, line));
    new_str(t, type_of(code))
}

/// `data[a:b:step]` on `Bytes`. `a` is the bytes (owned).
pub fn bytes_part(a: i64, b: i64, code: i64, line: i32) -> i64 {
    let part = part_of(b);
    let cut = crate::rt_bytes::with_bytes(a, |d| slices::pick(d, &part));
    release(a);
    let cut = cut.unwrap_or_else(|m| panic_with(m, line));
    crate::rt_bytes::new_bytes(cut, type_of(code))
}
