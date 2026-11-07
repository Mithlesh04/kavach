//! Tasks waiting for sockets (decision 0046): a task waits with
//! `wait_io`, and whichever thread waits on the system's readiness queue
//! wakes it (`wake_wait`). When no task is ready but some wait for a
//! socket, the scheduler's thread is that one (`set_io_idle`).
//!
//! With several workers (decision 0049), each has a queue of its own,
//! for the sockets its tasks made, and waits on it whenever it has
//! nothing to run; a task made ready by another thread ends that wait.

use super::{SLOT_MASK, Scheduler, State, Stop, TaskId, wake};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{MutexGuard, OnceLock};
use std::time::{Duration, Instant};

/// How the scheduler reaches the readiness queues (`net::poller`).
struct Queues {
    /// Waits on worker `w`'s queue up to a time (`None`: as long as it
    /// takes), waking the tasks whose sockets are ready.
    idle: fn(usize, Option<Duration>),
    /// Ends a wait on worker `w`'s queue now.
    interrupt: fn(usize),
}

static QUEUES: OnceLock<Queues> = OnceLock::new();

/// How many workers run tasks; 0 until the program says (one).
static WORKERS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// The worker this thread is, once it says (`set_this_worker`).
    static WORKER: Cell<usize> = const { Cell::new(usize::MAX) };
}

/// Tasks run on `n` workers (decision 0049).
pub fn set_many_workers(n: usize) {
    WORKERS.store(n, Ordering::Relaxed);
}

/// Whether tasks run on several workers.
pub fn many_workers() -> bool {
    worker_count() > 1
}

/// How many workers run tasks: so how many readiness queues there are.
pub fn worker_count() -> usize {
    WORKERS.load(Ordering::Relaxed).max(1)
}

/// This thread is worker `w`'s.
pub fn set_this_worker(w: usize) {
    WORKER.set(w);
}

/// The worker this thread is: its sockets go on that worker's queue.
/// Threads that aren't workers count as the main task's, worker 0.
pub fn this_worker() -> usize {
    WORKER.get().min(worker_count() - 1)
}

/// Whether this thread is worker `w`'s, so it is awake.
pub(super) fn is_worker(w: usize) -> bool {
    WORKER.get() == w
}

/// Sets how the scheduler waits on a worker's readiness queue, and ends
/// such a wait; the network's code sets it, so programs without
/// networking don't link it.
pub fn set_io_idle(idle: fn(usize, Option<Duration>), interrupt: fn(usize)) {
    let _ = QUEUES.set(Queues { idle, interrupt });
}

/// Whether the readiness queues exist.
pub(super) fn has_queue() -> bool {
    QUEUES.get().is_some()
}

/// Ends a wait on the readiness queue of task `t`'s worker: after waking
/// `t` from a thread of the runtime's own (a child process's reader).
pub fn interrupt_queue_of(t: TaskId) {
    interrupt(super::worker(t));
}

/// Ends a wait on worker `w`'s queue.
pub(super) fn interrupt(w: usize) {
    if let Some(q) = QUEUES.get() {
        (q.interrupt)(w);
    }
}

impl Scheduler {
    /// Task `me` waits until `wake_wait(me, w)` with the number `w` that
    /// `next_wait(me)` gave just before, or until `at`: for a socket to
    /// be ready (`net::poller`). The waker runs on another thread.
    pub fn wait_io(&self, me: TaskId, at: Option<Instant>) -> Result<(), Stop> {
        self.lock_of(me).io_waiting += 1;
        let waited = self.wait(me, None, at, true);
        self.lock_of(me).io_waiting -= 1;
        waited.or_else(|s| match s {
            Stop::TimedOut => Ok(()),
            other => Err(other),
        })
    }

    /// `wake_wait`, from a thread of the runtime's own (a child process's
    /// reader): it also ends a wait on the task's worker's readiness
    /// queue, which the program's sockets may have it in.
    pub fn wake_from_thread(&self, task: TaskId, wait: u64) {
        self.wake_wait(task, wait);
        interrupt_queue_of(task);
    }

    /// The number task `me`'s next wait will have (only `me` starts its
    /// waits, so it can't change before then).
    pub fn next_wait(&self, me: TaskId) -> u64 {
        self.lock_of(me).task(me).map_or(0, |t| t.wait + 1)
    }

    /// Wait `wait` of task `task` is over, if the task is still in it:
    /// from any thread.
    pub fn wake_wait(&self, task: TaskId, wait: u64) {
        let mut st = self.lock_of(task);
        let woken = match st.task(task) {
            Some(t) if t.wait < wait => {
                st.tasks[task & SLOT_MASK].early = wait;
                None
            }
            Some(_) => wake(&mut st, task, wait, false),
            None => None,
        };
        if self.shards.len() == 1 {
            // Under the lock: the thread either hasn't checked yet, or
            // waits.
            self.shards[0].turn.notify_all();
            return;
        }
        drop(st);
        self.woke(woken);
    }

    /// Lets go of the state and waits on the worker's readiness queue up
    /// to `timeout`; `false` when no network code set it, and nothing was
    /// done.
    pub(super) fn io_idle<'s>(
        &'s self,
        st: MutexGuard<'s, State>,
        timeout: Option<Duration>,
    ) -> (MutexGuard<'s, State>, bool) {
        let worker = st.worker;
        let Some(q) = QUEUES.get() else {
            return (st, false);
        };
        drop(st);
        (q.idle)(worker, timeout);
        (self.lock(worker), true)
    }
}
