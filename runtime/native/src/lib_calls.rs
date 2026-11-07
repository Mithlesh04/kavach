//! Calls into a library made by `kavach build --lib` (decision 0026),
//! from any thread, several at once (decision 0035).
//!
//! A library built through LLVM has a pool and a stack limit per thread,
//! so its calls compute at the same time: a call takes the library lock
//! only at its first effect another call could see (`turn`, from
//! `rt_tasks::turn`), and keeps it until it returns. A library built with
//! Cranelift takes objects from the one global pool, so each call takes
//! the lock as it enters.
//!
//! The lock is held per thread: a C callback that calls back into the
//! library from the same call goes on instead of waiting.

use crate::rt;
use std::cell::Cell;
use std::sync::{Condvar, Mutex};

/// The stack a library function may use below where it was called: the
/// calling thread's stack size is unknown (often 1 MB), so a deep
/// recursion panics well before it could run out (decision 0026).
const LIB_STACK: usize = 256 * 1024;

/// Whether some thread holds the library lock.
static BUSY: Mutex<bool> = Mutex::new(false);
static FREE: Condvar = Condvar::new();

thread_local! {
    /// Library calls running on this thread: more than one when a C
    /// callback called back in.
    static CALLS: Cell<u32> = const { Cell::new(0) };
    /// Whether this thread holds the library lock.
    static HELD: Cell<bool> = const { Cell::new(false) };
}

fn lock() {
    let mut busy = BUSY.lock().unwrap_or_else(|e| e.into_inner());
    while *busy {
        busy = FREE.wait(busy).unwrap_or_else(|e| e.into_inner());
    }
    *busy = true;
    HELD.set(true);
}

fn unlock() {
    *BUSY.lock().unwrap_or_else(|e| e.into_inner()) = false;
    HELD.set(false);
    FREE.notify_one();
}

/// Before an effect other calls could see: a library call that runs
/// without the lock waits for it.
pub(crate) fn turn() {
    if CALLS.get() > 0 && !HELD.get() {
        lock();
    }
}

/// Sets up the runtime on the first call: the library's path and shape
/// table, as `kv_main` does for a program.
fn begin(path: *const u8, path_len: usize, shapes: *const u8, shapes_len: usize) {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let path = rt::text(path, path_len).to_string();
        let shapes = kavach_kir::decode_shapes(rt::text(shapes, shapes_len))
            .expect("the compiler wrote a valid shape table");
        crate::start(&path, &shapes);
    });
    CALLS.set(CALLS.get() + 1);
    rt::set_stack_use(LIB_STACK);
}

/// Before an exported function of a library built with Cranelift runs:
/// sets up the runtime and the thread's stack limit, and takes the lock.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_lib_enter(
    path: *const u8,
    path_len: usize,
    shapes: *const u8,
    shapes_len: usize,
) {
    if !HELD.get() {
        lock();
    }
    begin(path, path_len, shapes, shapes_len);
}

/// Before an exported function of a library built through LLVM runs:
/// as `kv_lib_enter`, with the module's per-thread stack limit
/// (`rt::set_limit_slot`), and the lock left for the call's first effect.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_lib_enter_ahead(
    path: *const u8,
    path_len: usize,
    shapes: *const u8,
    shapes_len: usize,
    limit_slot: extern "C-unwind" fn() -> *mut usize,
) {
    static SLOT: std::sync::Once = std::sync::Once::new();
    SLOT.call_once(|| rt::set_limit_slot(limit_slot));
    begin(path, path_len, shapes, shapes_len);
}

/// After an exported function of a library returns: when it holds the
/// lock, what it printed is written out, and the thread's outermost call
/// gives the lock back.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_lib_leave() {
    let calls = CALLS.get().saturating_sub(1);
    CALLS.set(calls);
    crate::live::flush_live();
    if HELD.get() {
        rt::flush();
        if calls == 0 {
            unlock();
        }
    }
}
