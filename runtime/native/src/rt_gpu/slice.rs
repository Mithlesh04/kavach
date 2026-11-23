// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `xs[a:b:step]` of a list in GPU memory (spec 15.4): `GpuSlice` and
//! `GpuSliceStay`. The slice is made there, by the GPU, when the list's
//! items are only there, or when the slice stays there for the GPU to
//! read next; then nothing comes back to the CPU. Otherwise the CPU makes
//! it, as `ListSlicePart` does. The ends and the step are worked out here
//! first, so the panics are the CPU's.

use super::kept::{bring_back, fetch, keep, only_on_gpu, resident};
use crate::heap::release;
use crate::rt::{panic_with, type_of};
use kavach_runtime_core::slices::Part;

/// `list[part]`, `words` the slice's four words; with `stay`, the slice
/// may stay in GPU memory only, as a `map`'s result does.
pub(super) fn slice(list: i64, words: i64, code: i64, line: i32, stay: bool) -> i64 {
    let part = crate::rt_slices::part_of(words);
    if let Some(made) = on_gpu(list, &part, code, line, stay) {
        release(list);
        return made;
    }
    fetch(list);
    crate::rt_slices::slice_list(list, &part, code, line)
}

/// The slice made on the GPU, or `None` for the CPU to make it.
fn on_gpu(list: i64, part: &Part, code: i64, line: i32, stay: bool) -> Option<i64> {
    let items = resident(list)?;
    if !stay && !only_on_gpu(list) {
        return None;
    }
    let ty = type_of(code);
    // A packed list of 32-bit items, or a flat one of records and tuples
    // of them: as a `map` leaves its results.
    let width = match crate::flat::item_size(ty) {
        4 => 1,
        _ if crate::flat::is_text(ty) => return None,
        _ => crate::flat::width(ty),
    };
    let n = crate::heap::list_len_items_pub(list);
    if width == 0 || items.bytes() != n * width * 4 {
        return None;
    }
    let (start, end, step) = part.bounds(n).unwrap_or_else(|m| panic_with(m, line));
    let count = match step > 0 {
        true if end > start => (end - start + step - 1) / step,
        false if start > end => (start - end - step - 1) / -step,
        _ => 0,
    } as usize;
    // A GPU buffer can't be empty: the CPU makes an empty list.
    if count == 0 {
        return None;
    }
    let made_on = kavach_runtime_gpu::slice(&items, start as usize, step, count, width).ok()?;
    let made = match width {
        1 => crate::heap::new_list_raw(ty, 0, vec![0; count.div_ceil(2)], count),
        w => crate::heap::alloc_words(ty, vec![0; count * w]),
    };
    // One the CPU reads next comes back now, and isn't kept: it would
    // take the place of lists the GPU reads again (`kept.rs`).
    match stay {
        true => keep(made, made_on, true),
        false => bring_back(made, &made_on),
    }
    Some(made)
}
