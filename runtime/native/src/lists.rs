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
#[allow(unsafe_code)]
fn with_packed<R>(p: i64, f: impl FnOnce(usize, &mut Vec<u64>) -> R) -> R {
    let size = crate::flat::item_size(obj(p).ty());
    let l = list_mut(p);
    let cap = l.cap * size / 8;
    // SAFETY: a packed list's buffer is a `Vec<u64>` taken apart
    // (`new_list_raw`), always zero-filled to its capacity, rebuilt
    // here and taken apart the same way.
    let mut v = unsafe { Vec::from_raw_parts(l.ptr, cap, cap) };
    let result = f(l.len, &mut v);
    v.resize(v.capacity(), 0);
    let mut v = std::mem::ManuallyDrop::new(v);
    (l.ptr, l.cap) = (v.as_mut_ptr(), v.capacity() * 8 / size);
    result
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
