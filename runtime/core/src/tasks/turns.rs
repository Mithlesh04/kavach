//! Passing the turn between task threads (decision 0020). The thread
//! whose turn it is wakes alone: a server with a thousand connections
//! wakes one thread per hand-over, not a thousand. A thread whose turn
//! may come at once spins for a moment before it sleeps, when few
//! threads spin: waking a sleeping thread takes the system several
//! microseconds, a spinning one sees its turn in well under one.
//!
//! In native code, most tasks are coroutines on the main task's thread,
//! the host (decision 0048, `runtime/native/src/rt_coroutines.rs`): one
//! waiting for its turn goes back to the host, which runs the coroutine
//! whose turn it is, with no system call.

use super::{SLOT_MASK, Scheduler, State, TaskId};
use std::sync::atomic::Ordering;
use std::sync::{MutexGuard, OnceLock};
use std::time::{Duration, Instant};

/// How tasks that aren't threads of their own take turns: set by native
/// code, which runs them as coroutines on one thread, the host.
pub struct Host {
    /// Whether task `t` is a coroutine on the host.
    pub hosts: fn(TaskId) -> bool,
    /// Whether this thread is the host, outside any coroutine.
    pub on_host: fn() -> bool,
    /// From a coroutine: back to the host, until it runs this one again.
    pub suspend: fn(),
    /// On the host: runs coroutine `t` until it suspends or ends.
    pub resume: fn(TaskId),
}

static HOST: OnceLock<Host> = OnceLock::new();

/// Sets how coroutine tasks take turns (once, by native code).
pub fn set_host(host: Host) {
    let _ = HOST.set(host);
}

/// How long a thread spins for its turn before it sleeps.
const SPIN: Duration = Duration::from_micros(50);

/// How many threads may spin at once: none on one core, at most 4.
pub(super) fn max_spinners() -> usize {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    (cores / 2).min(4)
}

impl Scheduler {
    /// Whether it is task `t`'s turn on its worker now.
    pub fn has_turn(&self, t: TaskId) -> bool {
        self.shards[super::worker(t)]
            .running_now
            .load(Ordering::Acquire)
            == t
    }

    /// The turn goes to task `next`: its thread wakes, alone.
    pub(super) fn give_turn(&self, st: &mut State, next: TaskId) {
        st.running = next;
        self.shards[st.worker]
            .running_now
            .store(next, Ordering::Release);
        let task = st.tasks.get(next & SLOT_MASK);
        if let Some(thread) = task.and_then(|t| t.thread.as_ref()) {
            thread.unpark();
        }
    }

    /// Waits until it is task `me`'s turn (`give_turn`). The thread is
    /// noted first, under the lock, so a turn given while it lets go of
    /// the lock still wakes it.
    pub(super) fn await_turn<'s>(
        &'s self,
        mut st: MutexGuard<'s, State>,
        me: TaskId,
    ) -> MutexGuard<'s, State> {
        st.tasks[me & SLOT_MASK].thread = Some(std::thread::current());
        let worker = st.worker;
        while st.running != me {
            let running = st.running;
            drop(st);
            match HOST.get() {
                // A coroutine goes back to the host, which runs the next.
                Some(h) if (h.hosts)(me) => (h.suspend)(),
                Some(h) if (h.hosts)(running) && (h.on_host)() => (h.resume)(running),
                _ => {
                    self.spin_for(me, worker);
                    if self.shards[worker].running_now.load(Ordering::Acquire) != me {
                        // An `unpark` that came first makes this return
                        // at once.
                        std::thread::park();
                    }
                }
            }
            st = self.lock(worker);
        }
        st
    }

    /// Task `t`'s turns wake `thread`: the host's, for a coroutine that
    /// hasn't waited yet.
    pub fn set_thread(&self, t: TaskId, thread: std::thread::Thread) {
        if let Some(task) = self.lock_of(t).tasks.get_mut(t & SLOT_MASK) {
            task.thread = Some(thread);
        }
    }

    /// Spins until it is `me`'s turn, for `SPIN` at most, unless enough
    /// threads spin already.
    fn spin_for(&self, me: TaskId, worker: usize) {
        if self.spinners.fetch_add(1, Ordering::AcqRel) < self.max_spinners {
            let start = Instant::now();
            let now = &self.shards[worker].running_now;
            while now.load(Ordering::Acquire) != me && start.elapsed() < SPIN {
                std::hint::spin_loop();
            }
        }
        self.spinners.fetch_sub(1, Ordering::AcqRel);
    }
}
