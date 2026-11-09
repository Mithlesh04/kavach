//! Running out of memory is a panic (spec 5.6): an allocation the system
//! refuses ends the program with `panic: out of memory` and exit code
//! 101, as other panics do, instead of Rust's abort. Every allocation goes
//! through here, the runtime's and the interpreter's alike, since the
//! `kavach` command links this library too.
//!
//! Big blocks freed are kept a while, for the next of the same size: a new
//! one comes from the system with its pages unmapped, and the first write
//! to each page costs far more than the write. A loop that makes a list of
//! 64 MB each round spent 36 of 42 ms on that (benchmarks/gpu).
//!
//! Small blocks come from `small_blocks.rs`, for the same reason.

use crate::small_blocks;
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write;
use std::sync::Mutex;

/// The sizes of block kept, and the most kept at once.
const BIG: usize = 1 << 20;
const BIGGEST: usize = 256 << 20;
const MOST_BLOCKS: usize = 4;
const MOST_BYTES: usize = 512 << 20;

/// A block kept: its address, size and alignment. `size` 0 is a free place.
#[derive(Clone, Copy)]
struct Kept {
    at: usize,
    size: usize,
    align: usize,
}

const NONE: Kept = Kept {
    at: 0,
    size: 0,
    align: 0,
};

/// The blocks kept, the oldest first.
static KEPT: Mutex<[Kept; MOST_BLOCKS]> = Mutex::new([NONE; MOST_BLOCKS]);

/// Big blocks asked for as zeros, by address. Their pages may never have
/// been written (a list only the GPU holds), so they aren't kept: the next
/// block would still fault in every page, and to make room a block whose
/// pages are all in use would go, which is slow to give back.
static ZEROED: Mutex<[usize; MOST_BLOCKS * 2]> = Mutex::new([0; MOST_BLOCKS * 2]);

fn note_zeroed(at: *mut u8) {
    let mut zeroed = ZEROED.lock().unwrap_or_else(|e| e.into_inner());
    zeroed.copy_within(1.., 0);
    zeroed[MOST_BLOCKS * 2 - 1] = at as usize;
}

/// Whether the block at `at` was asked for as zeros: forgets it.
fn was_zeroed(at: *mut u8) -> bool {
    let mut zeroed = ZEROED.lock().unwrap_or_else(|e| e.into_inner());
    let found = zeroed.iter().position(|z| *z == at as usize);
    if let Some(i) = found {
        zeroed[i] = 0;
    }
    found.is_some()
}

fn kept() -> std::sync::MutexGuard<'static, [Kept; MOST_BLOCKS]> {
    KEPT.lock().unwrap_or_else(|e| e.into_inner())
}

/// The newest block kept of this layout, taken out.
fn take_kept(layout: Layout) -> Option<*mut u8> {
    let mut blocks = kept();
    let at = blocks
        .iter()
        .rposition(|b| b.size == layout.size() && b.align == layout.align())?;
    let block = blocks[at];
    blocks.copy_within(at + 1.., at);
    blocks[MOST_BLOCKS - 1] = NONE;
    Some(block.at as *mut u8)
}

/// Keeps a block, and gives the blocks that must go to make room: none,
/// one or all, as `NONE` where none goes.
fn keep(block: Kept) -> [Kept; MOST_BLOCKS] {
    let mut gone = [NONE; MOST_BLOCKS];
    let mut blocks = kept();
    let mut bytes: usize = blocks.iter().map(|b| b.size).sum();
    let mut n = 0;
    while blocks[MOST_BLOCKS - 1].size != 0 || bytes + block.size > MOST_BYTES {
        gone[n] = blocks[0];
        n += 1;
        bytes -= blocks[0].size;
        blocks.copy_within(1.., 0);
        blocks[MOST_BLOCKS - 1] = NONE;
        if bytes == 0 {
            break;
        }
    }
    let place = blocks.iter().position(|b| b.size == 0).unwrap_or(0);
    blocks[place] = block;
    gone
}

/// Gives the blocks kept back to the system. Whether there were any.
fn give_back() -> bool {
    let blocks = std::mem::replace(&mut *kept(), [NONE; MOST_BLOCKS]);
    free_all(&blocks);
    blocks.iter().any(|b| b.size != 0)
}

fn free_all(blocks: &[Kept]) {
    for b in blocks.iter().filter(|b| b.size != 0) {
        // SAFETY: a block kept came from `System` with this layout, which
        // `dealloc` checked was valid, and nothing else holds it.
        unsafe {
            System.dealloc(
                b.at as *mut u8,
                Layout::from_size_align_unchecked(b.size, b.align),
            );
        }
    }
}

struct Allocator;

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

/// The program ends: nothing more may be asked of the allocator, so the
/// message is written without making text, and no end hooks run.
#[cold]
fn out_of_memory(size: usize) -> ! {
    let mut err = std::io::stderr().lock();
    let _ = writeln!(err, "panic: out of memory (asked for {size} bytes)");
    let _ = err.flush();
    std::process::exit(101)
}

// SAFETY: every method passes its arguments on to the system allocator
// unchanged, and gives back what it gave, so the contract of `GlobalAlloc`
// holds as it does for `System`; a null pointer, the only refusal, ends
// the program instead of being given back. A big block freed is kept
// instead of passed on, and given out again only as a block of the same
// size and alignment, uninitialized as `alloc` may be, so what `alloc`
// promises holds for it too; nothing else can reach it while it is kept.
// A layout `small_blocks::serves` goes there both ways, as it is given the
// same layout when freed, and gets a block at least as long and as aligned.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if small_blocks::serves(layout) {
            // SAFETY: `dealloc` gives it back with the same layout.
            let mut p = unsafe { small_blocks::take(layout.size()) };
            if p.is_null() && give_back() {
                // SAFETY: as above.
                p = unsafe { small_blocks::take(layout.size()) };
            }
            if p.is_null() {
                out_of_memory(layout.size());
            }
            return p;
        }
        if (BIG..=BIGGEST).contains(&layout.size())
            && let Some(p) = take_kept(layout)
        {
            return p;
        }
        // SAFETY: as above.
        let mut p = unsafe { System.alloc(layout) };
        if p.is_null() && give_back() {
            // SAFETY: as above.
            p = unsafe { System.alloc(layout) };
        }
        if p.is_null() {
            out_of_memory(layout.size());
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if small_blocks::serves(layout) {
            // SAFETY: as above, and `alloc` gave `size` bytes to write.
            unsafe {
                let p = self.alloc(layout);
                std::ptr::write_bytes(p, 0, layout.size());
                return p;
            }
        }
        // SAFETY: as above.
        let mut p = unsafe { System.alloc_zeroed(layout) };
        if p.is_null() && give_back() {
            // SAFETY: as above.
            p = unsafe { System.alloc_zeroed(layout) };
        }
        if p.is_null() {
            out_of_memory(layout.size());
        }
        if (BIG..=BIGGEST).contains(&layout.size()) {
            note_zeroed(p);
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if small_blocks::serves(layout) {
            // SAFETY: `alloc` took it there, with this size.
            unsafe { small_blocks::give(ptr, layout.size()) };
            return;
        }
        if (BIG..=BIGGEST).contains(&layout.size()) && !was_zeroed(ptr) {
            let gone = keep(Kept {
                at: ptr as usize,
                size: layout.size(),
                align: layout.align(),
            });
            free_all(&gone);
            return;
        }
        // SAFETY: as above.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller promises a valid size for this alignment.
        let new = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        let (small, new_small) = (small_blocks::serves(layout), small_blocks::serves(new));
        if small && new_small {
            let same =
                small_blocks::size_index(layout.size()) == small_blocks::size_index(new_size);
            if same {
                return ptr;
            }
        }
        if small || new_small {
            // SAFETY: both blocks are as long as what is copied, and apart.
            unsafe {
                let p = self.alloc(new);
                std::ptr::copy_nonoverlapping(ptr, p, layout.size().min(new_size));
                self.dealloc(ptr, layout);
                return p;
            }
        }
        // SAFETY: as above.
        let mut p = unsafe { System.realloc(ptr, layout, new_size) };
        if p.is_null() && give_back() {
            // SAFETY: as above.
            p = unsafe { System.realloc(ptr, layout, new_size) };
        }
        if p.is_null() {
            out_of_memory(new_size);
        }
        p
    }
}
