// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `Drop` in native code (spec 6.1), as in the interpreter
//! (`compiler/interp/src/drops.rs`): when the last reference to a value of
//! a type that implements `Drop` goes, the object isn't freed yet. It
//! waits, held once more, and `DropRun`, which generated code calls at
//! each statement boundary, runs its `drop`. `drop` takes the value and
//! lets go of it at its end: then the object is freed for real.
//!
//! The program registers each type's `drop` at its start (`DropRegister`):
//! a program without `Drop` never reaches this file (decision 0024).

use crate::heap::{obj, obj_mut, slots};
use kavach_kir::{RtOp, Type};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

crate::rt_data::entry!(kv_rt_drops, |op, a, b, c, code, line| match op {
    RtOp::DropRegister => register(a, b),
    RtOp::DropRun => run(),
    _ => {
        let _ = (c, code, line);
        0
    }
});

/// Each type's closure around its `drop`, kept for the whole program.
static TABLE: OnceLock<Mutex<HashMap<u32, i64>>> = OnceLock::new();

fn table() -> std::sync::MutexGuard<'static, HashMap<u32, i64>> {
    TABLE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// How many objects wait for their `drop`, on every thread: generated
/// code calls `DropRun` only when this isn't 0, so a statement boundary
/// costs a load, not a call (Sakshi's S11).
#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static kv_drops_waiting: AtomicU64 = AtomicU64::new(0);

/// Bumped each time a type registers its `drop`: a thread's copy of the
/// types (`KNOWN`) is out of date when it differs.
static GENERATION: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// This thread's copy of the types with a `drop`, and the generation it
    /// was taken at. Every free asks it, so it is read with no lock: one
    /// lock on every free kept 8 cores waiting on each other (S3).
    static KNOWN: RefCell<(u64, HashSet<u32>)> = RefCell::new((u64::MAX, HashSet::new()));
    /// Objects whose last reference went, waiting for their `drop`.
    static PENDING: RefCell<Vec<i64>> = const { RefCell::new(Vec::new()) };
    /// Objects whose `drop` has started: the next release frees them.
    static DROPPING: RefCell<HashSet<i64>> = RefCell::new(HashSet::new());
    /// Inside a `drop`: the waiting objects before this index wait for it.
    static FLOOR: Cell<usize> = const { Cell::new(0) };
}

/// `DropRegister`: values of type number `ty` run closure `drop` (owned).
fn register(ty: i64, drop: i64) -> i64 {
    if let Some(old) = table().insert(ty as u32, drop) {
        crate::heap::release(old);
    }
    GENERATION.fetch_add(1, Ordering::Release);
    crate::rc::set_drop_hook(defer);
    crate::rc::set_drop_runner(|| {
        let _ = run();
    });
    static AT_EXIT: std::sync::Once = std::sync::Once::new();
    AT_EXIT.call_once(|| crate::rt::at_exit(forget_all));
    0
}

/// Lets go of the closures at the end (for the leak check).
fn forget_all() {
    let all: Vec<i64> = table().drain().map(|(_, c)| c).collect();
    for c in all {
        crate::heap::release(c);
    }
}

/// The last reference to `p` is going: `true` when it waits for its
/// `drop` instead of being freed now.
fn defer(p: i64) -> bool {
    let Type::Ref(id) = obj(p).ty() else {
        return false;
    };
    if !has_drop(id) {
        return false;
    }
    if DROPPING.with(|d| d.borrow_mut().remove(&p)) {
        return false;
    }
    // A copy made for another core, while other copies still hold the
    // value: freed now, as `drop` runs once, for the last (spec 6.1).
    if !leave_copies(p) {
        return false;
    }
    obj_mut(p).rc = 1;
    PENDING.with(|q| q.borrow_mut().push(p));
    kv_drops_waiting.fetch_add(1, Ordering::Relaxed);
    true
}

/// Whether type number `id` has a `drop`, from this thread's copy.
fn has_drop(id: u32) -> bool {
    let now = GENERATION.load(Ordering::Acquire);
    KNOWN.with(|k| {
        let mut k = k.borrow_mut();
        if k.0 != now {
            *k = (now, table().keys().copied().collect());
        }
        k.1.contains(&id)
    })
}

/// `DropRun`: runs `drop` for the objects waiting, oldest first; inside
/// a `drop`, only those that waited from within it.
fn run() -> i64 {
    loop {
        let floor = FLOOR.get();
        let next = PENDING.with(|q| {
            let mut q = q.borrow_mut();
            (q.len() > floor).then(|| q.remove(floor))
        });
        let Some(p) = next else { return 0 };
        kv_drops_waiting.fetch_sub(1, Ordering::Relaxed);
        let Type::Ref(id) = obj(p).ty() else { continue };
        let Some(closure) = table().get(&id).copied() else {
            continue;
        };
        DROPPING.with(|d| d.borrow_mut().insert(p));
        FLOOR.set(PENDING.with(|q| q.borrow().len()));
        // The closure's function borrows the closure (`kavach_kir::borrows`)
        // and takes the value, owned.
        let f = slots(closure)[0] as usize;
        // SAFETY: slot 0 of a closure is its function's address; the code
        // generator compiled a `drop` closure as `fn(closure, value)`,
        // returning nothing, and the code lives as long as the program.
        #[allow(unsafe_code)]
        let f: extern "C-unwind" fn(i64, i64) = unsafe { std::mem::transmute(f) };
        f(closure, p);
        FLOOR.set(floor);
    }
}

/// Copies of one `Drop` value made for tasks on other cores (`send.rs`):
/// each copy's object, and how many of them are still held. Copies are
/// the same value (spec 6.1), so `drop` runs once, when the last goes.
static COPIES: OnceLock<Mutex<HashMap<i64, std::sync::Arc<AtomicU64>>>> = OnceLock::new();

/// Whether any `Drop` value was ever copied for another core.
static ANY_COPIES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn copies() -> std::sync::MutexGuard<'static, HashMap<i64, std::sync::Arc<AtomicU64>>> {
    COPIES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// `to` was made from `from` for another core: when its type has a
/// `drop`, the two count as holders of one value.
pub(crate) fn copied(from: i64, to: i64) {
    // No type has a `drop` yet: nothing to ask for each object copied.
    if GENERATION.load(Ordering::Acquire) == 0 {
        return;
    }
    let Type::Ref(id) = obj(from).ty() else {
        return;
    };
    if !has_drop(id) {
        return;
    }
    ANY_COPIES.store(true, Ordering::Release);
    let mut all = copies();
    let group = match all.get(&from) {
        Some(g) => {
            g.fetch_add(1, Ordering::AcqRel);
            g.clone()
        }
        None => {
            let g = std::sync::Arc::new(AtomicU64::new(2));
            all.insert(from, g.clone());
            g
        }
    };
    all.insert(to, group);
}

/// `p`'s last reference is going: whether it is the last of its copies,
/// so its `drop` runs. A value never copied is its own last.
fn leave_copies(p: i64) -> bool {
    if !ANY_COPIES.load(Ordering::Acquire) {
        return true;
    }
    match copies().remove(&p) {
        Some(group) => group.fetch_sub(1, Ordering::AcqRel) == 1,
        None => true,
    }
}
