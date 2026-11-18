//! A pool for small fixed objects (records, tuples, union values,
//! closures). Freed objects wait in a list per size; new ones are cut from
//! large blocks. Taking or giving back an object is a few instructions,
//! with no locking and no per-object header from the system allocator.
//!
//! Memory in the pool is reused by later objects of the same size, and
//! returned to the system when the program ends.
//!
//! While tasks take turns, one thread runs at a time, so the pool is a
//! plain global, which `kavach run --native` code reads directly. When
//! tasks run on several threads at once (`rt::parallel`, in programs
//! built by `kavach build`), each thread has its own pool. A thread that
//! ends leaves its free objects and the rest of its block to the threads
//! after it: an `on threads` loop starts a thread a round (spec 4.9), and
//! a block kept by each would hold 4 MB a round.

use std::alloc::Layout;
use std::cell::UnsafeCell;
use std::ptr::null_mut;
use std::sync::Mutex;

/// Objects with at most this many slots come from the pool.
pub const MAX_SLOTS: usize = 16;

/// The size of each block objects are cut from, and its alignment: whole
/// 2 MB pages, so the operating system can back blocks with huge pages
/// (one page fault per 2 MB instead of one per 4 KB).
const BLOCK: usize = 4 << 20;
const BLOCK_ALIGN: usize = 2 << 20;

#[repr(C)]
pub struct Pool {
    /// For each slot count, the first free object; each free object holds
    /// the address of the next in its first word.
    free: [*mut u8; MAX_SLOTS + 1],
    /// The unused part of the current block.
    next: *mut u8,
    end: *mut u8,
}

pub struct Global(UnsafeCell<Pool>);

/// Byte offsets in `kv_pool`, for generated code: the free list of
/// objects with `n` slots is at `8 * n`.
pub const POOL_NEXT: i32 = 8 * (MAX_SLOTS as i32 + 1);
pub const POOL_END: i32 = POOL_NEXT + 8;
const _: () = assert!(std::mem::offset_of!(Pool, next) == POOL_NEXT as usize);
const _: () = assert!(std::mem::offset_of!(Pool, end) == POOL_END as usize);

// SAFETY: the global pool is used only while one thread runs at a time
// (see the top of this file).
unsafe impl Sync for Global {}

/// The pool. Generated code takes small objects from it directly (the
/// offsets above), and calls the runtime only when a block runs out.
#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static kv_pool: Global = Global(UnsafeCell::new(Pool {
    free: [null_mut(); MAX_SLOTS + 1],
    next: null_mut(),
    end: null_mut(),
}));

thread_local! {
    static LOCAL: UnsafeCell<Pool> = const {
        UnsafeCell::new(Pool {
            free: [null_mut(); MAX_SLOTS + 1],
            next: null_mut(),
            end: null_mut(),
        })
    };
    static LEAVING: Leaving = const { Leaving };
}

/// Reached by a thread's first block: when the thread ends, what its pool
/// holds goes to `LEFT`.
struct Leaving;

impl Drop for Leaving {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        let _ = LOCAL.try_with(|p| {
            // SAFETY: only this thread reaches its pool, and nothing else
            // holds a reference to it while its destructors run.
            let pool = unsafe { &mut *p.get() };
            let mut left = left();
            for n in 1..=MAX_SLOTS {
                if !pool.free[n].is_null() {
                    left.free[n].push(pool.free[n] as usize);
                    pool.free[n] = null_mut();
                }
            }
            if pool.next < pool.end {
                left.blocks.push((pool.next as usize, pool.end as usize));
            }
            pool.next = null_mut();
            pool.end = null_mut();
        });
    }
}

/// What ended threads left: per slot count, lists of free objects (by
/// their first), and the unused parts of blocks.
struct Left {
    free: [Vec<usize>; MAX_SLOTS + 1],
    blocks: Vec<(usize, usize)>,
}

static LEFT: Mutex<Left> = Mutex::new(Left {
    free: [const { Vec::new() }; MAX_SLOTS + 1],
    blocks: Vec::new(),
});

fn left() -> std::sync::MutexGuard<'static, Left> {
    LEFT.lock().unwrap_or_else(|e| e.into_inner())
}

/// The pool of the running thread.
fn pool() -> *mut Pool {
    if crate::rt::parallel() {
        LOCAL.with(|p| p.get())
    } else {
        kv_pool.0.get()
    }
}

/// Memory for an object of `size` bytes with `n` slots (`n <= MAX_SLOTS`),
/// aligned to 8 bytes. Its contents are undefined.
#[allow(unsafe_code)]
pub fn take(n: usize, size: usize) -> *mut u8 {
    // SAFETY: the pool is this thread's, or the global one while only one
    // thread runs (see above); no reference to it lives past this call.
    let pool = unsafe { &mut *pool() };
    take_from(pool, n, size, crate::rt::parallel())
}

/// `take` from `pool`, which is this thread's own when `own`.
#[allow(unsafe_code)]
#[inline]
fn take_from(pool: &mut Pool, n: usize, size: usize, own: bool) -> *mut u8 {
    let first = pool.free[n];
    if !first.is_null() {
        // SAFETY: a free object's first word holds the next free object.
        pool.free[n] = unsafe { *(first as *mut *mut u8) };
        return first;
    }
    if (pool.end as usize) - (pool.next as usize) < size {
        if own && let Some(p) = from_left(pool, n, size) {
            return p;
        }
        let layout = Layout::from_size_align(BLOCK, BLOCK_ALIGN).expect("a block layout");
        // SAFETY: the layout's size is not zero. Blocks are never freed:
        // their objects are reused through the free lists.
        let block = unsafe { std::alloc::alloc(layout) };
        if block.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        pool.next = block;
        // SAFETY: `end` is one past the block's last byte.
        pool.end = unsafe { block.add(BLOCK) };
    }
    let p = pool.next;
    // SAFETY: `size` bytes fit between `next` and `end` (checked above).
    pool.next = unsafe { p.add(size) };
    p
}

/// A thread's block ran out: an object from what ended threads left, a
/// list of free objects with `n` slots or the rest of a block, or `None`
/// when they left nothing that fits. Either way, what this thread holds
/// is left when it ends.
#[allow(unsafe_code)]
#[cold]
fn from_left(pool: &mut Pool, n: usize, size: usize) -> Option<*mut u8> {
    let _ = LEAVING.try_with(|_| {});
    let mut left = left();
    if let Some(first) = left.free[n].pop() {
        let first = first as *mut u8;
        // SAFETY: a free object's first word holds the next free object;
        // this thread's list for `n` is empty (`take` came here).
        pool.free[n] = unsafe { *(first as *mut *mut u8) };
        return Some(first);
    }
    while let Some((next, end)) = left.blocks.pop() {
        if end - next >= size {
            pool.next = (next + size) as *mut u8;
            pool.end = end as *mut u8;
            return Some(next as *mut u8);
        }
    }
    None
}

/// Gives back an object taken with `take(n, …)`.
#[allow(unsafe_code)]
pub fn give(p: *mut u8, n: usize) {
    // SAFETY: as in `take`; `p` is at least 8 bytes and no longer used.
    unsafe {
        let pool = &mut *pool();
        *(p as *mut *mut u8) = pool.free[n];
        pool.free[n] = p;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A thread that ends leaves the rest of its block, and the next
    /// thread cuts its objects from it instead of taking a new block.
    #[test]
    #[allow(unsafe_code)]
    fn an_ending_thread_leaves_its_block() {
        let in_thread = || {
            std::thread::spawn(|| {
                // SAFETY: this thread's own pool, used only here.
                LOCAL.with(|p| take_from(unsafe { &mut *p.get() }, 3, 32, true) as usize)
            })
            .join()
            .expect("the thread ends")
        };
        let first = in_thread();
        let second = in_thread();
        // Only this test leaves blocks (no test runs tasks on threads),
        // so the second object comes right after the first.
        assert_eq!(second, first + 32, "{first:#x} then {second:#x}");
    }
}
