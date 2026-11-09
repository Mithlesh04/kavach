//! Counting live objects, for the leak check (`KAVACH_CHECK_LEAKS`):
//! every object made and not yet freed.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Objects allocated and not yet freed, to find reference-counting bugs.
/// While tasks take turns, one thread runs at a time, so a plain load and
/// store is enough; generated code counts the objects it makes itself.
/// When tasks run at once (`rt::parallel`), each thread counts in `LIVE`
/// and adds its count here when its task ends (`flush_live`).
#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static kv_live: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static LIVE: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
}

pub fn live_objects() -> usize {
    flush_live();
    kv_live.load(Ordering::Relaxed)
}

/// Adds this thread's count to `kv_live`.
pub fn flush_live() {
    let n = LIVE.replace(0);
    if n != 0 {
        kv_live.fetch_add(n as usize, Ordering::Relaxed);
    }
}

pub(crate) fn count_live(step: isize) {
    if crate::rt::parallel() {
        LIVE.set(LIVE.get() + step);
        return;
    }
    let n = kv_live.load(Ordering::Relaxed) as isize + step;
    kv_live.store(n as usize, Ordering::Relaxed);
}
