// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Copying a value for a task that runs at the same time as others
//! (decision 0020). Reference counts are not atomic, so two threads
//! running at once must never hold the same object: a task that runs
//! ahead gets a copy of every object its closure holds.
//!
//! Values shared on purpose (`Shared`, channels, tasks) are objects
//! around a *cell*, counted atomically: a copy is a new object around the
//! same cell. What a cell holds is only used by the task holding the turn
//! (`rt_tasks::turn`), so freeing an object around a cell, which may free
//! what the cell holds, waits for the turn too. Compiled regular
//! expressions are shared as they are: nothing in them changes.

use crate::heap::{
    Data, FIXED, alloc, alloc_fixed, data, has_data, new_obj, obj, release_slot, slot_type, slots,
    slots_mut,
};
use crate::shapes::{holds_refs, shape};
use crate::text_items::{INLINE, WIDTH};
use kavach_kir::{Shape, Type};
use std::cell::Cell;
use std::sync::Arc;

/// What a `Shared` value holds (spec 6.2), and its lock: with tasks on
/// all cores, `get`, `set` and `update` take it (decision 0052). It is
/// held for a few instructions, never across a wait, so a task that
/// finds it taken spins.
pub struct SharedCell {
    value: Cell<u64>,
    /// The task holding the lock, plus one; 0 when it is free.
    owner: std::sync::atomic::AtomicUsize,
    /// How many times its owner took it: `get` inside `update` takes it
    /// again.
    depth: Cell<u32>,
}

impl SharedCell {
    fn lock(&self) {
        use std::sync::atomic::Ordering;
        let me = crate::rt_tasks::me() + 1;
        if self.owner.load(Ordering::Acquire) == me {
            self.depth.set(self.depth.get() + 1);
            return;
        }
        // Taken by another core: wait twice as long after each try, so
        // many cores wanting one value don't all hammer it at once.
        let mut backoff = 1u32;
        while self
            .owner
            .compare_exchange_weak(0, me, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self.owner.load(Ordering::Relaxed) != 0 {
                if backoff < 1 << 10 {
                    for _ in 0..backoff {
                        std::hint::spin_loop();
                    }
                    backoff *= 2;
                } else {
                    std::thread::yield_now();
                }
            }
        }
        self.depth.set(1);
        kavach_runtime_core::tasks::hold(true);
    }

    fn unlock(&self) {
        let depth = self.depth.get() - 1;
        self.depth.set(depth);
        if depth == 0 {
            kavach_runtime_core::tasks::hold(false);
            self.owner.store(0, std::sync::atomic::Ordering::Release);
        }
    }
}

/// A value going in or out of a `Shared`: with several workers, a copy
/// that shares no object with the task that gave it (as for channels).
fn crossing(v: u64, ty: Type) -> u64 {
    if !kavach_runtime_core::tasks::many_workers() || !ty.is_ref() || v == 0 {
        return v;
    }
    copy(v, ty)
}

// SAFETY: a cell is used only by the thread holding its program's turn
// (see the top of this file), and the turn passes through a mutex, which
// orders memory between threads. Only the atomic count of the `Arc`
// around it is touched by threads running at once.
#[allow(unsafe_code)]
unsafe impl Send for SharedCell {}
#[allow(unsafe_code)]
unsafe impl Sync for SharedCell {}
#[allow(unsafe_code)]
unsafe impl Send for crate::rt_tasks::TaskObj {}
#[allow(unsafe_code)]
unsafe impl Sync for crate::rt_tasks::TaskObj {}
#[allow(unsafe_code)]
unsafe impl Send for crate::rt_channels::ChanObj {}
#[allow(unsafe_code)]
unsafe impl Sync for crate::rt_channels::ChanObj {}

/// `Shared(v)` (`v` owned) of type `ty`.
pub fn new_shared(v: u64, ty: Type, line: i32) -> i64 {
    let cell = Arc::new(SharedCell {
        value: Cell::new(v),
        owner: std::sync::atomic::AtomicUsize::new(0),
        depth: Cell::new(0),
    });
    if crate::rt::kv_free_at_exit.load(std::sync::atomic::Ordering::Relaxed) != 0 {
        let mut made = MADE.lock().unwrap_or_else(|e| e.into_inner());
        made.push((Arc::downgrade(&cell), line));
    }
    alloc(ty, Data::Shared(cell))
}

/// With the leak check on (`KAVACH_CHECK_LEAKS`), every `Shared` cell
/// made and the line that made it, so one never freed is reported with
/// its place (spec 6.2).
static MADE: std::sync::Mutex<Vec<(std::sync::Weak<SharedCell>, i32)>> =
    std::sync::Mutex::new(Vec::new());

/// The lines of the `Shared` values still alive, with how many each: at
/// the end of a program whose objects were all freed (`kv_free_at_exit`),
/// only a cycle keeps one alive.
pub fn leaked_shared() -> Vec<(i32, usize)> {
    let made = MADE.lock().unwrap_or_else(|e| e.into_inner());
    // In the order they were made; a leak is rare, so a scan is enough.
    let mut counted: Vec<(i32, usize)> = Vec::new();
    for (_, line) in made.iter().filter(|(cell, _)| cell.strong_count() > 0) {
        match counted.iter_mut().find(|(l, _)| l == line) {
            Some((_, n)) => *n += 1,
            None => counted.push((*line, 1)),
        }
    }
    counted
}

/// Runs `f` on the cell of `Shared` object `p` (borrowed), under its lock.
fn with_cell<R>(p: i64, f: impl FnOnce(&SharedCell) -> R) -> Option<R> {
    match &*data(p).borrow() {
        Data::Shared(c) => {
            c.lock();
            let r = f(c);
            c.unlock();
            Some(r)
        }
        _ => None,
    }
}

/// The value in `Shared` object `p` (borrowed), of type `ty`, as a value
/// of its own (owned): a copy when tasks run on several cores.
pub fn shared_value(p: i64, ty: Type) -> u64 {
    with_cell(p, |c| {
        let v = c.value.get();
        if kavach_runtime_core::tasks::many_workers() {
            return crossing(v, ty);
        }
        if ty.is_ref() && v != 0 {
            crate::heap::retain(v as i64);
        }
        v
    })
    .unwrap_or(0)
}

/// Puts `v` (owned), of type `ty`, in `Shared` object `p`; gives the old
/// value (owned).
pub fn set_shared(p: i64, v: u64, ty: Type) -> u64 {
    let given = crossing(v, ty);
    if given != v {
        release_slot(v as i64, ty);
    }
    with_cell(p, |c| c.value.replace(given)).unwrap_or(given)
}

/// Runs `f` on the value in `Shared` object `p`, borrowed, under its lock:
/// for printing it, with no copy.
pub fn with_shared_value<R>(p: i64, f: impl FnOnce(u64) -> R) -> Option<R> {
    with_cell(p, |c| f(c.value.get()))
}

thread_local! {
    /// The values whose `update` this thread's task is running: a panic
    /// in the function unwinds past the end of the `update` (decision
    /// 0065), so the task's start lets go of them (`let_go_after_panic`).
    static UPDATING: std::cell::RefCell<Vec<Arc<SharedCell>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// `update` takes (`true`) or lets go of `Shared` object `p`'s lock: it
/// holds it while its function runs (decision 0052).
pub fn shared_lock(p: i64, taken: bool) {
    if let Data::Shared(c) = &*data(p).borrow() {
        if taken {
            c.lock();
            if c.depth.get() == 1 {
                UPDATING.with(|u| u.borrow_mut().push(Arc::clone(c)));
            }
        } else {
            c.unlock();
            if c.depth.get() == 0 {
                UPDATING.with(|u| u.borrow_mut().retain(|x| !Arc::ptr_eq(x, c)));
            }
        }
    }
}

/// A panic unwound out of `update`s, whose functions never stored their
/// results: their values let go, as they were (spec 9.1).
pub(crate) fn let_go_after_panic() {
    for c in UPDATING.with(|u| std::mem::take(&mut *u.borrow_mut())) {
        c.depth.set(1);
        c.unlock();
    }
}

// `s.weak()` and `w.shared()`, apart from `kv_rt`, so a program without a
// `Weak` doesn't link them (decision 0024).
crate::rt_data::entry!(kv_rt_weak, |op, a, _b, _c, code, _line| {
    weak_op(op == kavach_kir::RtOp::WeakMake, a, code)
});

/// `s.weak()` (`make`): a `Weak` around `Shared` object `a`'s cell; or
/// `w.shared()`: the `Shared` that `Weak` object `a` points to, or `none`
/// once it was freed (spec 6.2). `code` is the result's type; `a` is
/// released.
fn weak_op(make: bool, a: i64, code: i64) -> i64 {
    let ty = crate::rt::type_of(code);
    let made = match &*data(a).borrow() {
        Data::Shared(c) if make => Some((ty, Data::Weak(Arc::downgrade(c)))),
        Data::Weak(w) if !make => w.upgrade().map(|c| {
            // `Shared<T>?`: the optional of an object is the object or 0.
            let shared = match shape(ty) {
                Shape::Optional(t) => *t,
                _ => ty,
            };
            (shared, Data::Shared(c))
        }),
        _ => None,
    };
    crate::heap::release(a);
    made.map_or(0, |(t, d)| alloc(t, d))
}

/// Whether object `p`, of a `Shared` type, is a `Weak`.
pub fn is_weak(p: i64) -> bool {
    matches!(&*data(p).borrow(), Data::Weak(_))
}

/// Runs `f` on the value `Weak` object `p` points to, while it lives.
pub fn with_weak_value<R>(p: i64, f: impl FnOnce(u64) -> R) -> Option<R> {
    let cell = match &*data(p).borrow() {
        Data::Weak(w) => w.upgrade(),
        _ => None,
    }?;
    cell.lock();
    let r = f(cell.value.get());
    cell.unlock();
    Some(r)
}

/// What tells copies of one cell from other values: the cell's address.
pub fn identity(p: i64) -> usize {
    match &*data(p).borrow() {
        Data::Shared(c) => Arc::as_ptr(c) as usize,
        Data::Weak(c) => c.as_ptr() as usize,
        Data::Task(t) => Arc::as_ptr(t) as usize,
        Data::Channel(c) => Arc::as_ptr(c) as usize,
        // Apart from the sender's: the two ends are different values.
        Data::Receiver(r) => Arc::as_ptr(r.chan()) as usize + 1,
        Data::Reader(r) => Arc::as_ptr(r) as usize,
        Data::Socket(s) => s.address(),
        _ => p as usize,
    }
}

/// Before object `p`, around a cell, is freed: when it is the cell's last
/// copy, what the cell holds is released (`kv_free`).
pub fn free_cell(p: i64) {
    crate::rt_tasks::turn();
    let item = match shape(obj(p).ty()) {
        Shape::Shared(t) | Shape::Task(t) | Shape::Channel(t) => *t,
        _ => Type::Unit,
    };
    match &*data(p).borrow() {
        Data::Shared(c) if Arc::strong_count(c) == 1 => release_slot(c.value.get() as i64, item),
        Data::Task(t) if Arc::strong_count(t) == 1 => crate::rt_tasks::free_task(t, item),
        Data::Channel(c) if Arc::strong_count(c) == 1 => crate::rt_channels::free_chan(c, item),
        Data::Receiver(r) if Arc::strong_count(r.chan()) == 1 => {
            crate::rt_channels::free_chan(r.chan(), item)
        }
        _ => {}
    }
}
/// A copy of `slot` (of type `ty`) that shares no object with it, owned
/// by the caller. Every value can be copied: a cell is shared through its
/// atomic count, a regular expression as it is (see the top of this file).
pub fn copy(slot: u64, ty: Type) -> u64 {
    if !ty.is_ref() || slot == 0 {
        return slot;
    }
    let p = slot as i64;
    let (own, tag) = (obj(p).ty(), obj(p).tag);
    if has_data(p) {
        // SAFETY: copying only reads `p`. An object two threads can reach
        // has more than one holder, so nothing changes it in place (it is
        // copied on write); reading it through `borrow()` would write the
        // `RefCell`'s borrow count, which isn't atomic, so two threads
        // copying one object, or one copying while another reads it, could
        // corrupt the count ("already mutably borrowed", seen under load
        // in `06-memory/drop-runs-once-for-copies-on-other-cores.kav`).
        let mut d = unsafe { (*data(p).as_ptr()).clone() };
        match &mut d {
            Data::Map(t) => {
                for (i, s) in t.slots.iter_mut().enumerate() {
                    *s = copy(*s, slot_type(p, i));
                }
            }
            Data::Context(c) => c.1 = copy(c.1, own),
            // The document is shared as it is: it never changes. Its list
            // or map is made again on the other side.
            Data::Json(_, _, made) => *made = 0,
            _ => {}
        }
        return new_obj(own, tag, Vec::new(), d) as u64;
    }
    if obj(p).kind == FIXED {
        // A record, tuple or union value: made with its slots in place,
        // with no list of words in between.
        let made = alloc_fixed(own, tag, slots(p).len());
        let words = slots_mut(made);
        words.copy_from_slice(slots(p));
        if holds_refs(own) {
            copy_held(p, own, tag, words);
        }
        crate::rt_drops::copied(p, made);
        return made as u64;
    }
    let mut words = slots(p).to_vec();
    if crate::flat::packed(own) {
        // Packed numbers: the words carry no references.
        return crate::heap::new_list_raw(own, tag, words, crate::heap::list_len_items(p)) as u64;
    }
    if crate::flat::is_text(own) {
        // Long text items are objects; short ones are in the words.
        for item in words.chunks_mut(WIDTH) {
            if (item[1] >> 56) as u8 & INLINE == 0 {
                item[0] = copy(item[0], own);
            }
        }
    } else if holds_refs(own) {
        // Numbers, and flat lists of them, are copied with the words.
        copy_held(p, own, tag, &mut words);
    }
    let made = new_obj(own, tag, words, Data::Slots);
    // A `Drop` value's copies are one value: `drop` runs once (spec 6.1).
    crate::rt_drops::copied(p, made);
    made as u64
}

/// Replaces each object in `words`, the slots of object `p` (of type `own`
/// and variant `tag`), with a copy. The slots' types are looked up once
/// for the object, not once a slot: a list of a million records asks for
/// its item type once.
fn copy_held(p: i64, own: Type, tag: u32, words: &mut [u64]) {
    match shape(own) {
        // Not flat (`holds_refs`): one item a word.
        Shape::List(t) => words.iter_mut().for_each(|w| *w = copy(*w, *t)),
        Shape::Record { fields, .. } => {
            for (w, f) in words.iter_mut().zip(fields) {
                *w = copy(*w, f.ty);
            }
        }
        Shape::Union { variants, .. } => {
            for (w, f) in words.iter_mut().zip(&variants[tag as usize].fields) {
                *w = copy(*w, f.ty);
            }
        }
        _ => {
            for (i, w) in words.iter_mut().enumerate() {
                *w = copy(*w, slot_type(p, i));
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
