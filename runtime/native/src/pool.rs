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
//! built by `kavach build`), each thread has its own pool.

use std::alloc::Layout;
use std::cell::UnsafeCell;
use std::ptr::null_mut;

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
    let first = pool.free[n];
    if !first.is_null() {
        // SAFETY: a free object's first word holds the next free object.
        pool.free[n] = unsafe { *(first as *mut *mut u8) };
        return first;
    }
    if (pool.end as usize) - (pool.next as usize) < size {
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
