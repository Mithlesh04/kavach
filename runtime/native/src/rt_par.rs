//! Chunked loops on all cores (decision 0031). `kv_par_for` runs a
//! compiler-made chunk function over `lo..hi` split across threads, and
//! returns 0 when every chunk ran clean. It returns 1 — "run the plain
//! loop yourself" — when the loop is small, one core is all there is,
//! `KAVACH_SERIAL=1` is set, or a chunk's check failed; the caller's
//! plain loop then keeps every panic at its exact round and line.
//!
//! A chunk function does nothing but arithmetic and loads and stores on
//! buffers the loop owns (`compiler/kir/src/par.rs` proves it), so the
//! threads share no reference counts, no allocator and no runtime
//! state.

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};

/// Fewer rounds than this run serially: waking the pool costs more
/// than the work.
const MIN_ROUNDS: i64 = 1 << 16;

fn serial() -> bool {
    static SERIAL: OnceLock<bool> = OnceLock::new();
    *SERIAL.get_or_init(|| std::env::var_os("KAVACH_SERIAL").is_some())
}

/// A loop the pool is running: the chunk function, its captures by
/// value (at most six, `compiler/kir/src/par/analyze.rs`), and how the
/// rounds are grabbed.
#[derive(Clone, Copy)]
struct Job {
    f: usize,
    caps: [i64; 6],
    n: usize,
    lo: i64,
    hi: i64,
    /// Rounds one grab takes.
    step: i64,
    /// A reduction (decision 0031): the chunk also gets its grab's
    /// slot number, `(a - lo) / step`, always under `SLOTS`.
    red: bool,
}

struct State {
    job: Option<Job>,
    /// Counts the jobs, so a worker never reruns one it finished.
    stamp: u64,
}

/// The kept workers: loops come often and threads are not free, so the
/// workers start once and park between loops. One loop runs at a time;
/// a second caller waits its turn.
struct Pool {
    workers: usize,
    state: Mutex<State>,
    /// Wakes the workers for a new job.
    go: Condvar,
    /// Wakes the caller when the job's last worker leaves, and the next
    /// caller when the job slot clears.
    done: Condvar,
    /// Where the next grab starts.
    next: AtomicI64,
    /// How many workers are inside the current job.
    busy: AtomicI64,
    failed: AtomicBool,
}

fn pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    static START: std::sync::Once = std::sync::Once::new();
    let p = POOL.get_or_init(|| Pool {
        workers: std::thread::available_parallelism().map_or(1, |p| p.get()),
        state: Mutex::new(State {
            job: None,
            stamp: 0,
        }),
        go: Condvar::new(),
        done: Condvar::new(),
        next: AtomicI64::new(0),
        busy: AtomicI64::new(0),
        failed: AtomicBool::new(false),
    });
    // Outside the init: a worker's first `pool()` call must not nest.
    START.call_once(|| {
        for _ in 0..p.workers {
            std::thread::spawn(work);
        }
    });
    p
}

/// One kept worker: waits for a job it hasn't run, helps until no
/// rounds are left, and tells the caller when it leaves.
fn work() {
    let p = pool();
    let mut seen = 0;
    loop {
        let job = {
            let mut state = p.state.lock().unwrap();
            loop {
                if state.stamp != seen
                    && let Some(job) = state.job
                {
                    seen = state.stamp;
                    // Committed under the lock: once the caller
                    // clears the job, no worker can still join it.
                    p.busy.fetch_add(1, Ordering::AcqRel);
                    break job;
                }
                state = p.go.wait(state).unwrap();
            }
        };
        run_shares(p, &job);
        // Leaving under the lock too, so the caller's check and this
        // wakeup can't miss each other.
        let _state = p.state.lock().unwrap();
        p.busy.fetch_sub(1, Ordering::AcqRel);
        p.done.notify_all();
    }
}

/// Grabs and runs ranges of `job` until the loop is used up.
fn run_shares(p: &Pool, job: &Job) {
    loop {
        let a = p.next.fetch_add(job.step, Ordering::Relaxed);
        if a >= job.hi {
            return;
        }
        let b = (a + job.step).min(job.hi);
        let caps = &job.caps[..job.n];
        let bad = if job.red {
            let k = (a - job.lo) / job.step;
            call_red(job.f as *const u8, a, b, k, caps)
        } else {
            call(job.f as *const u8, a, b, caps)
        };
        if bad != 0 {
            p.failed.store(true, Ordering::Relaxed);
        }
    }
}

/// How many grabs a loop can have: the slot lists of a reduction hold
/// this many partial sums.
pub const SLOTS: i64 = 256;

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_par_for(
    lo: i64,
    hi: i64,
    f: *const u8,
    caps: *const i64,
    n: i64,
) -> i64 {
    fan_out(lo, hi, f, caps, n, false)
}

/// A reduction: like `kv_par_for`, and each grab's chunk call also gets
/// its slot number. The last three captures are the slot lists for the
/// partial sums and the prefix bounds, zeroed here first, so slots no
/// grab wrote read as "no rounds ran".
#[unsafe(no_mangle)]
#[expect(clippy::not_unsafe_ptr_arg_deref, reason = "generated code's ABI")]
pub extern "C-unwind" fn kv_par_for_red(
    lo: i64,
    hi: i64,
    f: *const u8,
    caps: *const i64,
    n: i64,
) -> i64 {
    if n < 3 {
        return 1;
    }
    for back in 1..=3 {
        // SAFETY: the compiler passes `n` capture slots next to each
        // other on the caller's stack; the last three hold the lists.
        let list = unsafe { *caps.add((n - back) as usize) };
        let slots = crate::heap::slots_mut(list);
        if slots.len() < SLOTS as usize {
            return 1;
        }
        slots[..SLOTS as usize].fill(0);
    }
    fan_out(lo, hi, f, caps, n, true)
}

fn fan_out(lo: i64, hi: i64, f: *const u8, caps: *const i64, n: i64, red: bool) -> i64 {
    if hi.saturating_sub(lo) < MIN_ROUNDS || n > 6 || serial() {
        return 1;
    }
    let p = pool();
    if p.workers < 2 {
        return 1;
    }
    let mut slots = [0i64; 6];
    for (k, slot) in slots.iter_mut().take(n as usize).enumerate() {
        // SAFETY: the compiler passes `n` capture slots next to each
        // other on the caller's stack, alive across this call.
        *slot = unsafe { *caps.add(k) };
    }
    let step = ((hi - lo) / (p.workers as i64 * 4))
        .max(4096)
        .max((hi - lo).div_euclid(SLOTS) + 1);
    {
        let mut state = p.state.lock().unwrap();
        // One loop at a time: wait until the last one's slot clears.
        while state.job.is_some() {
            state = p.done.wait(state).unwrap();
        }
        p.next.store(lo, Ordering::Relaxed);
        p.failed.store(false, Ordering::Relaxed);
        state.job = Some(Job {
            f: f as usize,
            caps: slots,
            n: n as usize,
            lo,
            hi,
            step,
            red,
        });
        state.stamp += 1;
        p.go.notify_all();
    }
    // This thread helps too; the rounds are gone when it comes back.
    run_shares(
        p,
        &Job {
            f: f as usize,
            caps: slots,
            n: n as usize,
            lo,
            hi,
            step,
            red,
        },
    );
    let mut state = p.state.lock().unwrap();
    while p.busy.load(Ordering::Acquire) > 0 {
        state = p.done.wait(state).unwrap();
    }
    state.job = None;
    p.done.notify_all();
    drop(state);
    p.failed.load(Ordering::Relaxed) as i64
}

/// Calls a reduction chunk: its slot number, then the captures.
fn call_red(f: *const u8, lo: i64, hi: i64, k: i64, caps: &[i64]) -> i64 {
    // SAFETY: `f` is the chunk function the compiler built for this
    // very call site, taking `lo`, `hi`, the slot and these captures.
    unsafe {
        use std::mem::transmute as t;
        type P = *const u8;
        match *caps {
            [] => t::<P, extern "C-unwind" fn(i64, i64, i64) -> i64>(f)(lo, hi, k),
            [a] => t::<P, extern "C-unwind" fn(i64, i64, i64, i64) -> i64>(f)(lo, hi, k, a),
            [a, b] => {
                t::<P, extern "C-unwind" fn(i64, i64, i64, i64, i64) -> i64>(f)(lo, hi, k, a, b)
            }
            [a, b, c] => t::<P, extern "C-unwind" fn(i64, i64, i64, i64, i64, i64) -> i64>(f)(
                lo, hi, k, a, b, c,
            ),
            [a, b, c, d] => t::<P, extern "C-unwind" fn(i64, i64, i64, i64, i64, i64, i64) -> i64>(
                f,
            )(lo, hi, k, a, b, c, d),
            [a, b, c, d, e] => t::<
                P,
                extern "C-unwind" fn(i64, i64, i64, i64, i64, i64, i64, i64) -> i64,
            >(f)(lo, hi, k, a, b, c, d, e),
            [a, b, c, d, e, g] => t::<
                P,
                extern "C-unwind" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64,
            >(f)(lo, hi, k, a, b, c, d, e, g),
            _ => 1,
        }
    }
}

/// Calls the chunk with its captures spread as plain arguments.
fn call(f: *const u8, lo: i64, hi: i64, caps: &[i64]) -> i64 {
    // SAFETY: `f` is the chunk function the compiler built for this
    // very call site, taking `lo`, `hi` and exactly these captures.
    unsafe {
        use std::mem::transmute as t;
        type P = *const u8;
        match *caps {
            [] => t::<P, extern "C-unwind" fn(i64, i64) -> i64>(f)(lo, hi),
            [a] => t::<P, extern "C-unwind" fn(i64, i64, i64) -> i64>(f)(lo, hi, a),
            [a, b] => t::<P, extern "C-unwind" fn(i64, i64, i64, i64) -> i64>(f)(lo, hi, a, b),
            [a, b, c] => {
                t::<P, extern "C-unwind" fn(i64, i64, i64, i64, i64) -> i64>(f)(lo, hi, a, b, c)
            }
            [a, b, c, d] => t::<P, extern "C-unwind" fn(i64, i64, i64, i64, i64, i64) -> i64>(f)(
                lo, hi, a, b, c, d,
            ),
            [a, b, c, d, e] => {
                t::<P, extern "C-unwind" fn(i64, i64, i64, i64, i64, i64, i64) -> i64>(f)(
                    lo, hi, a, b, c, d, e,
                )
            }
            [a, b, c, d, e, g] => t::<
                P,
                extern "C-unwind" fn(i64, i64, i64, i64, i64, i64, i64, i64) -> i64,
            >(f)(lo, hi, a, b, c, d, e, g),
            _ => 1,
        }
    }
}
