//! Small blocks for everything the runtime and the compiler allocate: a
//! list's header, a short buffer, a map's table. Each thread takes them from
//! a free list per size, or cuts them from a span of its own, with no lock
//! and no header per block.
//!
//! Spans are cut from regions of whole 2 MB pages, so the system can back
//! them with huge pages: one page fault per 2 MB instead of one per 4 KB,
//! which is most of what a fresh small allocation costs (`pool.rs` does the
//! same for objects). A thread that frees far more than it takes passes
//! blocks on to the shared lists, and a thread that ends leaves all of its
//! own there, so nothing is lost to a thread that has gone. Regions are never
//! returned to the system: their blocks are reused by later blocks of the
//! same size.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::UnsafeCell;
use std::ptr::null_mut;
use std::sync::Mutex;

/// The largest size served here, and the sizes it comes in. Blocks are
/// aligned to `STEP`, which is as much as the system's allocator promises.
const MOST: usize = 512;
const STEP: usize = 16;
const SIZES: usize = MOST / STEP + 1;

/// How much a thread cuts at a time: a thread that makes little holds
/// little, and hands back what it didn't use when it ends.
const SPAN: usize = 64 << 10;

/// What is asked of the system at a time, and its alignment.
const REGION: usize = 4 << 20;
const REGION_ALIGN: usize = 2 << 20;

/// A thread passes blocks on once it holds this many bytes of free blocks
/// of one size, and passes half of them.
const MOST_HELD: usize = 1 << 20;

struct Local {
    /// For each size, the first free block; each holds the next's address.
    free: [*mut u8; SIZES],
    /// How many blocks each list holds.
    held: [u32; SIZES],
    /// The unused part of this thread's span.
    next: *mut u8,
    end: *mut u8,
}

thread_local! {
    // No `Drop` here, so reaching it is a plain thread-local read.
    static LOCAL: UnsafeCell<Local> = const {
        UnsafeCell::new(Local {
            free: [null_mut(); SIZES],
            held: [0; SIZES],
            next: null_mut(),
            end: null_mut(),
        })
    };
    static LEAVING: Leaving = const { Leaving };
}

/// Reached by a thread's first span: when the thread ends, its blocks go to
/// the shared lists.
struct Leaving;

impl Drop for Leaving {
    fn drop(&mut self) {
        let _ = LOCAL.try_with(|l| {
            // SAFETY: only this thread reaches its `Local`, and no other
            // reference to it is alive: this runs outside any allocator call.
            unsafe { leave(&mut *l.get()) }
        });
    }
}

/// What threads share: a list of free blocks per size, spans a thread left
/// unfinished (each holds the next span's address, then its own end), and
/// the unused part of the current region.
struct Shared {
    free: [usize; SIZES],
    spans: usize,
    next: usize,
    end: usize,
}

static SHARED: Mutex<Shared> = Mutex::new(Shared {
    free: [0; SIZES],
    spans: 0,
    next: 0,
    end: 0,
});

fn shared() -> std::sync::MutexGuard<'static, Shared> {
    SHARED.lock().unwrap_or_else(|e| e.into_inner())
}

/// Whether a block of this layout comes from here.
#[inline]
pub(crate) fn serves(layout: Layout) -> bool {
    layout.size() <= MOST && layout.align() <= STEP
}

/// The list a block of `bytes` belongs to; its blocks are `index * STEP`
/// bytes long.
#[inline]
pub(crate) fn size_index(bytes: usize) -> usize {
    bytes.div_ceil(STEP).max(1)
}

/// What a free block holds in its first word: the next block on its list.
///
/// # Safety
/// `p` is a free block (or span) nothing else is using.
#[inline]
unsafe fn next_of(p: *mut u8) -> *mut u8 {
    // SAFETY: blocks are aligned to 16 bytes and at least 16 bytes long.
    unsafe { *(p as *mut *mut u8) }
}

/// # Safety
/// As for `next_of`.
#[inline]
unsafe fn set_next(p: *mut u8, next: *mut u8) {
    // SAFETY: as in `next_of`.
    unsafe { *(p as *mut *mut u8) = next }
}

/// A block of `bytes` (at most `MOST`), or null when the system has no
/// memory left.
///
/// # Safety
/// The block goes back through `give`, with the same `bytes`.
#[inline]
pub(crate) unsafe fn take(bytes: usize) -> *mut u8 {
    let i = size_index(bytes);
    LOCAL.with(|l| {
        // SAFETY: only this thread reaches its `Local`, and this is the only
        // reference to it: nothing below calls the allocator.
        let l = unsafe { &mut *l.get() };
        let first = l.free[i];
        if !first.is_null() {
            // SAFETY: a block on a free list is free.
            l.free[i] = unsafe { next_of(first) };
            l.held[i] -= 1;
            return first;
        }
        let size = i * STEP;
        if (l.end as usize) - (l.next as usize) < size {
            return refill(l, i);
        }
        let p = l.next;
        // SAFETY: `size` bytes fit between `next` and `end` (checked above).
        l.next = unsafe { p.add(size) };
        p
    })
}

/// Gives back a block from `take(bytes)`.
///
/// # Safety
/// `p` came from `take(bytes)`, on any thread, and is no longer used.
#[inline]
pub(crate) unsafe fn give(p: *mut u8, bytes: usize) {
    let i = size_index(bytes);
    LOCAL.with(|l| {
        // SAFETY: as in `take`.
        let l = unsafe { &mut *l.get() };
        // SAFETY: `p` is free now.
        unsafe { set_next(p, l.free[i]) };
        l.free[i] = p;
        l.held[i] += 1;
        if l.held[i] as usize * (i * STEP) > MOST_HELD {
            pass_on(l, i);
        }
    })
}

/// This thread has no block of list `i` and its span is too short: a block
/// from the shared list if it holds any, with more for this thread's list;
/// otherwise one cut from another span. Null when the system has no memory.
#[cold]
fn refill(l: &mut Local, i: usize) -> *mut u8 {
    // So this thread's blocks are handed back when it ends.
    let _ = LEAVING.try_with(|_| {});
    let mut s = shared();
    let size = i * STEP;
    if s.free[i] != 0 {
        let first = s.free[i] as *mut u8;
        let most = (MOST_HELD / 2 / size).max(1);
        let mut last = first;
        let mut taken = 1;
        // SAFETY: blocks on the shared list are free, and the lock is held.
        unsafe {
            while taken < most && !next_of(last).is_null() {
                last = next_of(last);
                taken += 1;
            }
            s.free[i] = next_of(last) as usize;
            set_next(last, null_mut());
            // Only an empty list gets here, so these are all it holds.
            l.free[i] = next_of(first);
        }
        l.held[i] = taken as u32 - 1;
        return first;
    }
    // The end of the old span is too short for this block: it serves the
    // list of its own size. Every block size is a multiple of `STEP`.
    let left = (l.end as usize) - (l.next as usize);
    if left >= STEP {
        let at = l.next;
        // SAFETY: the `left` bytes at `at` are this thread's and unused.
        unsafe { set_next(at, l.free[left / STEP]) };
        l.free[left / STEP] = at;
        l.held[left / STEP] += 1;
    }
    l.next = null_mut();
    l.end = null_mut();
    let (start, end) = if s.spans != 0 {
        let at = s.spans as *mut u8;
        // SAFETY: an unfinished span holds the next's address and its end,
        // and is longer than any block (`leave`); the lock is held.
        unsafe {
            s.spans = next_of(at) as usize;
            (at, *(at as *mut usize).add(1) as *mut u8)
        }
    } else {
        if s.end - s.next < SPAN {
            let layout = Layout::from_size_align(REGION, REGION_ALIGN).expect("a region layout");
            // SAFETY: the layout's size is not zero. Regions are never freed.
            let region = unsafe { System.alloc(layout) } as usize;
            if region == 0 {
                return null_mut();
            }
            s.next = region;
            s.end = region + REGION;
        }
        let start = s.next;
        s.next += SPAN;
        (start as *mut u8, (start + SPAN) as *mut u8)
    };
    // SAFETY: every span is longer than the largest block.
    l.next = unsafe { start.add(size) };
    l.end = end;
    start
}

/// Half of list `i` goes to the shared list.
#[cold]
fn pass_on(l: &mut Local, i: usize) {
    let keep = l.held[i] / 2;
    let mut last = l.free[i];
    // SAFETY: `held` counts the blocks on the list, all of them free.
    unsafe {
        for _ in 1..keep {
            last = next_of(last);
        }
        let passed = next_of(last);
        set_next(last, null_mut());
        let mut tail = passed;
        while !next_of(tail).is_null() {
            tail = next_of(tail);
        }
        let mut s = shared();
        set_next(tail, s.free[i] as *mut u8);
        s.free[i] = passed as usize;
    }
    l.held[i] = keep;
}

/// A thread ends: everything it holds goes to the shared lists.
fn leave(l: &mut Local) {
    let mut s = shared();
    for i in 1..SIZES {
        let first = l.free[i];
        if first.is_null() {
            continue;
        }
        let mut tail = first;
        // SAFETY: the blocks on this thread's lists are free.
        unsafe {
            while !next_of(tail).is_null() {
                tail = next_of(tail);
            }
            set_next(tail, s.free[i] as *mut u8);
        }
        s.free[i] = first as usize;
        l.free[i] = null_mut();
        l.held[i] = 0;
    }
    let left = (l.end as usize) - (l.next as usize);
    if left > MOST {
        // SAFETY: the unused bytes of the span are this thread's, and more
        // than the two words written.
        unsafe {
            set_next(l.next, s.spans as *mut u8);
            *(l.next as *mut usize).add(1) = l.end as usize;
        }
        s.spans = l.next as usize;
    } else if left >= STEP {
        // SAFETY: as above; a block of `left` bytes.
        unsafe { set_next(l.next, s.free[left / STEP] as *mut u8) };
        s.free[left / STEP] = l.next as usize;
    }
    l.next = null_mut();
    l.end = null_mut();
}

#[cfg(test)]
mod tests;
