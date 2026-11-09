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

thread_local! {
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
    crate::rc::set_drop_hook(defer);
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
    if !table().contains_key(&id) {
        return false;
    }
    if DROPPING.with(|d| d.borrow_mut().remove(&p)) {
        return false;
    }
    obj_mut(p).rc = 1;
    PENDING.with(|q| q.borrow_mut().push(p));
    true
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
