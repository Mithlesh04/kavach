// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A list's buffer: changing it as a `Vec`, and adding to it. A list
//! keeps its buffer as a `Vec<u64>` taken apart (`heap::ListObj`), with
//! its length and capacity counted in items; a flat list's items are
//! several words each (`flat.rs`).

use crate::heap::{list_mut, obj};

/// Changes a list's items as a `Vec`, then stores it back. The items of
/// a flat list are boxed into objects for `f`, and unboxed after it.
pub fn with_slots<R>(p: i64, f: impl FnOnce(&mut Vec<u64>) -> R) -> R {
    let ty = obj(p).ty();
    // A packed list (decisions 0030 and 0045): one item per word for
    // `f`, packed back after, with the exact item count kept aside.
    if let size @ 1.. = crate::flat::item_size(ty) {
        let item = crate::flat::item_of(ty);
        let mut items = with_packed(p, |len, words| crate::flat::unpack(words, len, item));
        let result = f(&mut items);
        let n = items.len();
        let packed = crate::flat::pack(&items, size);
        with_packed(p, move |_, words| *words = packed);
        crate::heap::list_mut(p).len = n;
        return result;
    }
    let width = crate::flat::width(ty);
    if width == 0 {
        return with_words(p, f);
    }
    let item = crate::flat::item_type(ty);
    let mut items = with_words(p, |w| {
        let items = crate::flat::box_all(item, w, width);
        // The boxed items now hold the long text the words held.
        if crate::flat::is_text(ty) {
            crate::text_items::release_all(w);
        }
        items
    });
    let result = f(&mut items);
    with_words(p, |w| *w = crate::flat::unbox_all(items, width));
    result
}

/// A packed list's words and item count, the words as a `Vec`.
///
/// The buffer always holds exactly `words_for(l.cap, size)` words, the
/// number `rc::free` gives back too: `size` divides 8, so without a limit
/// that is the `Vec`'s capacity, and a buffer that grows past a limit is
/// made again at exactly the limit's words.
#[allow(unsafe_code)]
fn with_packed<R>(p: i64, f: impl FnOnce(usize, &mut Vec<u64>) -> R) -> R {
    let size = crate::flat::item_size(obj(p).ty());
    let l = list_mut(p);
    let cap = crate::flat::words_for(l.cap, size);
    // SAFETY: a packed list's buffer is a `Vec<u64>` taken apart
    // (`new_list_raw`) with a capacity of `cap` words (above), always
    // zero-filled to it, rebuilt here and taken apart the same way.
    let mut v = unsafe { Vec::from_raw_parts(l.ptr, cap, cap) };
    let result = f(l.len, &mut v);
    let mut items = v.capacity() * 8 / size;
    // The room past a limit's last word is never used (spec 2.2).
    if let Some(n) = crate::heap::limit_of(p).filter(|n| items > *n) {
        items = n;
        let want = crate::flat::words_for(n, size);
        if v.capacity() != want {
            v = exactly(&v[..want.min(v.len())], want);
        }
    }
    v.resize(v.capacity(), 0);
    debug_assert_eq!(v.capacity(), crate::flat::words_for(items, size));
    let mut v = std::mem::ManuallyDrop::new(v);
    (l.ptr, l.cap) = (v.as_mut_ptr(), items);
    result
}

/// `words` in a new `Vec` whose capacity is exactly `room` words:
/// `with_capacity` promises that, where `shrink_to` and `reserve_exact`
/// don't.
fn exactly(words: &[u64], room: usize) -> Vec<u64> {
    let mut v = Vec::with_capacity(room);
    v.extend_from_slice(words);
    v
}

/// Changes a list's buffer as a `Vec`: its items, or a flat list's words.
#[allow(unsafe_code)]
pub fn with_words<R>(p: i64, f: impl FnOnce(&mut Vec<u64>) -> R) -> R {
    let width = crate::flat::width(obj(p).ty()).max(1);
    let l = list_mut(p);
    // SAFETY: a list's `ptr`, `len` and `cap` (in items of `width` words)
    // come from one `Vec<u64>`, rebuilt here and taken apart again before
    // returning.
    let mut v = unsafe { Vec::from_raw_parts(l.ptr, l.len * width, l.cap * width) };
    let result = f(&mut v);
    let mut v = std::mem::ManuallyDrop::new(crate::flat::fit(v, width));
    (l.ptr, l.len, l.cap) = (v.as_mut_ptr(), v.len() / width, v.capacity() / width);
    result
}

/// Makes room for `extra` more items, so a counted loop of pushes can
/// store them without checks (`stmt.rs` push hoisting).
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_list_reserve(p: i64, extra: i64) {
    // Never called for a list with a limit: its pushes are checked.
    if crate::heap::limit_of(p).is_some() {
        return;
    }
    let extra = usize::try_from(extra).unwrap_or(0);
    if let size @ 1.. = crate::flat::item_size(obj(p).ty()) {
        return with_packed(p, |len, w| {
            let want = crate::flat::words_for(len + extra, size);
            w.reserve(want.saturating_sub(w.len()));
        });
    }
    let width = crate::flat::width(obj(p).ty()).max(1);
    with_words(p, |w| w.reserve(extra * width));
}

/// List `p` (held only by the caller) with room made for exactly its
/// limit `n` (spec 2.2), which it then keeps.
pub fn room_for_limit(p: i64, n: usize) {
    if let size @ 1.. = crate::flat::item_size(obj(p).ty()) {
        with_packed(p, |len, w| {
            let used = crate::flat::words_for(len, size);
            *w = exactly(&w[..used], crate::flat::words_for(n, size));
        });
    } else {
        let want = n * crate::flat::width(obj(p).ty()).max(1);
        with_words(p, |w| *w = exactly(w, want));
    }
    crate::heap::set_limit(p, n);
}

/// Takes item 0 out of list `p` (held only by the caller), releasing
/// what it held: room for a full ring's push (spec 2.2).
pub fn drop_first(p: i64) {
    let ty = obj(p).ty();
    if crate::flat::packed(ty) {
        return with_slots(p, |items| {
            items.remove(0);
        });
    }
    let width = crate::flat::width(ty).max(1);
    let words = crate::heap::slots(p);
    if crate::flat::is_text(ty) {
        crate::text_items::release_all(&words[..width]);
    } else if crate::shapes::holds_refs(ty) {
        for (i, slot) in words.iter().enumerate().take(width) {
            crate::rc::release_slot(*slot as i64, crate::shapes::slot_type(p, i));
        }
    }
    with_words(p, |w| {
        w.drain(..width);
    });
}

/// Adds `item` (owned) to the end of list `p`.
pub fn push_item(p: i64, item: u64) {
    if let size @ 1.. = crate::flat::item_size(obj(p).ty()) {
        let at = list_mut(p).len;
        with_packed(p, |_, w| {
            if w.len() < crate::flat::words_for(at + 1, size) {
                w.push(0);
            }
            crate::flat::set_item(w, at, size, item);
        });
        list_mut(p).len = at + 1;
        return;
    }
    if crate::flat::width(obj(p).ty()) == 0 {
        with_words(p, |s| s.push(item));
    } else {
        with_words(p, |w| crate::flat::unbox_into(item as i64, w));
    }
}

/// `xs.swap(i, j)` (spec 2.2): list `p` (owned) with items `i` and `j`
/// swapped, in place when nothing else holds it. Only the items' words
/// move, so nothing is counted or copied: text and records included. An
/// index out of range panics as `xs[i]` does.
pub fn swap_checked(p: i64, i: i64, j: i64, line: i32) -> i64 {
    let n = crate::heap::len_of(p);
    for k in [i, j] {
        if k < 0 || k >= n as i64 {
            crate::rt::panic_with(
                &kavach_runtime_core::list_index_panic(i128::from(k), n),
                line,
            )
        }
    }
    if i == j {
        return p;
    }
    let q = crate::heap::unique(p);
    let (i, j) = (i as usize, j as usize);
    let ty = obj(q).ty();
    if let size @ 1.. = crate::flat::item_size(ty) {
        with_packed(q, |_, w| {
            let (a, b) = (
                crate::flat::get_item(w, i, size),
                crate::flat::get_item(w, j, size),
            );
            crate::flat::set_item(w, i, size, b);
            crate::flat::set_item(w, j, size, a);
        });
        return q;
    }
    let width = crate::flat::width(ty).max(1);
    let words = crate::heap::slots_mut(q);
    for k in 0..width {
        words.swap(i * width + k, j * width + k);
    }
    q
}

/// Cuts list `p` (held only by the caller) to its first `n` items,
/// releasing what the others held: the second half of `xs.pop()`.
pub fn truncate_list(p: i64, n: usize) {
    let ty = obj(p).ty();
    let len = list_mut(p).len;
    if n >= len {
        return;
    }
    if let size @ 1.. = crate::flat::item_size(ty) {
        // The buffer stays zero-filled past the last item (`with_packed`).
        with_packed(p, |_, w| {
            (n..len).for_each(|i| crate::flat::set_item(w, i, size, 0))
        });
        list_mut(p).len = n;
        return;
    }
    let width = crate::flat::width(ty).max(1);
    let words = crate::heap::slots(p);
    if crate::flat::is_text(ty) {
        crate::text_items::release_all(&words[n * width..]);
    } else if crate::shapes::holds_refs(ty) {
        for (i, slot) in words.iter().enumerate().skip(n * width) {
            crate::rc::release_slot(*slot as i64, crate::shapes::slot_type(p, i));
        }
    }
    with_words(p, |w| w.truncate(n * width));
}

#[cfg(test)]
mod tests {
    use crate::capacity::{capacity_op, push};
    use crate::heap::{len_of, release};
    use crate::send::tests::{BYTES, set_shapes};
    use kavach_kir::{RtOp, Type};

    /// `List<UInt8>.capacity(5)`, and `Ring<UInt8>.capacity(9)` with 20
    /// pushes, each made and let go.
    fn lists_with_a_limit() {
        let Type::Ref(id) = BYTES else { unreachable!() };
        let code = 100 + i64::from(id);
        let list = capacity_op(RtOp::ListCapacity, 5, 0, code, 0);
        release(list);
        let mut ring = capacity_op(RtOp::RingCapacity, 9, 0, code, 0);
        for i in 0..20 {
            ring = push(ring, i, 0);
        }
        assert_eq!(len_of(ring), 9);
        release(ring);
    }

    /// A packed list with a limit gives its buffer back with the size it
    /// was taken with, and keeps none (unsafe audit (a) 1).
    #[test]
    fn packed_lists_with_a_limit_free_what_they_took() {
        set_shapes();
        // Once first, for what the runtime makes once per thread.
        lists_with_a_limit();
        let (wrong, held) = crate::rt_memory::watch::watch(lists_with_a_limit);
        assert_eq!(wrong, 0, "blocks given back with the wrong size");
        assert_eq!(held, 0, "blocks never given back");
    }
}
