//! Lists kept in GPU memory (spec 15.2, decision 0055): which lists
//! have their items there, bringing them back when the CPU reads them,
//! and letting go of them.

use super::{as_bytes, as_bytes_mut, finish_late, slot};
use crate::heap::{obj, release};
use crate::rt::panic_with;

thread_local! {
    /// Lists whose items are in GPU memory too, the newest last.
    static RESIDENT: std::cell::RefCell<Resident> = const { std::cell::RefCell::new(Resident(Vec::new())) };
}

/// The most lists kept in GPU memory, and the most bytes they take.
const KEEP: usize = 4;
const KEEP_BYTES: usize = 512 << 20;

/// Lists kept in GPU memory, so a later `map` of one reads it there
/// instead of copying it over again (spec 15.2). Each list is held (one
/// reference), so it can't change in place: a write copies it first, and
/// the GPU's copy stays true. A list a `map` made may be only there
/// (`stale`: its items here aren't written yet) until `fetch` brings it
/// back; the compiler fetches it before any other read (`gpu_stay.rs`).
struct Resident(Vec<Kept>);

struct Kept {
    list: i64,
    items: kavach_runtime_gpu::Resident,
    stale: bool,
}

impl Drop for Resident {
    fn drop(&mut self) {
        for k in self.0.drain(..) {
            release(k.list);
        }
    }
}

/// `list`'s items in GPU memory, when they are kept there.
pub(super) fn resident(list: i64) -> Option<kavach_runtime_gpu::Resident> {
    RESIDENT.with(|r| {
        let r = r.borrow();
        r.0.iter().find(|k| k.list == list).map(|k| k.items.clone())
    })
}

/// Keeps `list` (now held here too) with its items in GPU memory,
/// letting go of the oldest beyond the limits. One only there comes
/// back first, unless nothing else holds it.
pub(super) fn keep(list: i64, items: kavach_runtime_gpu::Resident, stale: bool) {
    static FREE_AT_EXIT: std::sync::Once = std::sync::Once::new();
    FREE_AT_EXIT.call_once(|| {
        crate::rt::at_exit(finish_late);
        crate::rt::at_exit(forget_all);
    });
    crate::rc::retain(list);
    let dropped: Vec<Kept> = RESIDENT.with(|r| {
        let kept = &mut r.borrow_mut().0;
        // A list only there and held by nothing else can't be read
        // again: it goes first.
        let (mut dropped, held): (Vec<Kept>, Vec<Kept>) = std::mem::take(kept)
            .into_iter()
            .partition(|k| k.stale && crate::heap::obj(k.list).rc == 1);
        *kept = held;
        kept.push(Kept { list, items, stale });
        while kept.len() > KEEP || kept.iter().map(|k| k.items.bytes()).sum::<usize>() > KEEP_BYTES
        {
            dropped.push(kept.remove(0));
        }
        dropped
    });
    for k in dropped {
        if k.stale && crate::heap::obj(k.list).rc > 1 {
            bring_back(k.list, &k.items);
        }
        release(k.list);
    }
}

/// `list`, with its items written here if they were only on the GPU.
pub(super) fn fetch(list: i64) -> i64 {
    let stale = RESIDENT.with(|r| {
        let mut r = r.borrow_mut();
        let k = r.0.iter_mut().find(|k| k.list == list && k.stale)?;
        k.stale = false;
        Some(k.items.clone())
    });
    if let Some(items) = stale {
        bring_back(list, &items);
    }
    list
}

/// Copies `items` from GPU memory into `list`'s own: as they are for a
/// packed list of 32-bit items, field by field for a flat list of
/// records or tuples.
pub(super) fn bring_back(list: i64, items: &kavach_runtime_gpu::Resident) {
    finish_late();
    let n = crate::heap::list_len_items_pub(list);
    let ty = obj(list).ty();
    let width = crate::flat::width(ty);
    if width == 0 {
        let bytes = &mut as_bytes_mut(crate::heap::slots_mut(list))[..n * 4];
        if kavach_runtime_gpu::read(items, bytes).is_err() {
            panic_with("the GPU stopped working while it held a list's items", 0);
        }
        return;
    }
    let mut words = vec![0u64; (n * width).div_ceil(2)];
    if kavach_runtime_gpu::read(items, &mut as_bytes_mut(&mut words)[..n * width * 4]).is_err() {
        panic_with("the GPU stopped working while it held a list's items", 0);
    }
    // Each field's type once, then item by item: 4 million fields come
    // back in a few milliseconds, not twenty (benchmarks/gpu).
    let item = crate::flat::item_type(ty);
    let types: Vec<kavach_kir::Type> = (0..width)
        .map(|k| crate::flat::field_type(item, k))
        .collect();
    let bytes = &as_bytes(&words)[..n * width * 4];
    let slots = &mut crate::heap::slots_mut(list)[..n * width];
    for (fields, came) in slots
        .chunks_exact_mut(width)
        .zip(bytes.chunks_exact(4 * width))
    {
        for ((s, b), t) in fields.iter_mut().zip(came.as_chunks::<4>().0).zip(&types) {
            *s = slot(u32::from_le_bytes(*b), *t);
        }
    }
}

/// `list`'s items in GPU memory, out of the table, when only the table and
/// its caller hold it: whoever holds it now brings it back if it must
/// (and nothing lets go of it to make room). With whether they are only
/// there.
pub(super) fn take_alone(list: i64) -> Option<(kavach_runtime_gpu::Resident, bool)> {
    if obj(list).rc != 2 {
        return None;
    }
    let kept = RESIDENT.with(|r| {
        let kept = &mut r.borrow_mut().0;
        let at = kept.iter().position(|k| k.list == list)?;
        Some(kept.remove(at))
    })?;
    release(kept.list);
    Some((kept.items.clone(), kept.stale))
}

/// Lets go of `list`'s items in GPU memory, which a `map` wrote over.
pub(super) fn forget(list: i64) {
    let kept = RESIDENT.with(|r| {
        let kept = &mut r.borrow_mut().0;
        let at = kept.iter().position(|k| k.list == list)?;
        Some(kept.remove(at))
    });
    if let Some(k) = kept {
        release(k.list);
    }
}

/// Lets go of every kept list (at the end, for the leak check).
fn forget_all() {
    let kept = RESIDENT.with(|r| std::mem::take(&mut r.borrow_mut().0));
    for k in kept {
        release(k.list);
    }
}
