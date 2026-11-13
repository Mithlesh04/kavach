//! `task.cancel()` (spec 9.6, decision 0075): a task cancelled this way
//! stops at once, at its next wait, the end of a loop's round or the
//! start of a call, even when it only computes. Code checks `must_stop`
//! there; `any_stop_now` makes the check one read when nothing was ever
//! cancelled this way, so loops pay almost nothing.
//!
//! `spawn … within d` stops a task the same way once its time is up: it
//! checks its own deadline at those points, so it stops even when it
//! holds the turn and nothing else can run to cancel it, and its waits
//! end at the deadline (`wait.rs`).

use super::{SLOT_MASK, Scheduler, TaskId};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How many tasks were ever cancelled with `cancel()` or given a time
/// limit: until then, the checks ask the scheduler nothing.
static STOP_NOW: AtomicUsize = AtomicUsize::new(0);

/// Whether any task was ever cancelled with `cancel()` or given a time
/// limit.
pub fn any_stop_now() -> bool {
    STOP_NOW.load(Ordering::Relaxed) != 0
}

impl Scheduler {
    /// `task.cancel()`: cancels `task` so that it stops at once. A task
    /// that has ended is left as it is.
    pub fn cancel_now(&self, task: TaskId) {
        {
            let mut st = self.lock_of(task);
            if st.task(task).is_none_or(|t| t.done) {
                return;
            }
            st.tasks[task & super::SLOT_MASK].stop_now = true;
        }
        STOP_NOW.fetch_add(1, Ordering::Relaxed);
        self.cancel(task);
    }

    /// Whether task `me` must stop now: it was cancelled with `cancel()`,
    /// and it holds no `Shared` value's lock (an `update` runs to its end
    /// first, spec 9.6).
    pub fn must_stop(&self, me: TaskId) -> bool {
        any_stop_now() && !super::holding() && (self.stops_now(me) || self.out_of_time(me))
    }

    fn stops_now(&self, me: TaskId) -> bool {
        self.lock_of(me).task(me).is_some_and(|t| t.stop_now)
    }

    /// `spawn … within ms`: task `task` stops once `ms` milliseconds have
    /// passed from now.
    pub fn set_within(&self, task: TaskId, ms: i64) {
        let at = Instant::now() + Duration::from_millis(ms.max(0) as u64);
        let mut st = self.lock_of(task);
        if st.task(task).is_none_or(|t| t.done) {
            return;
        }
        st.tasks[task & SLOT_MASK].deadline = Some((at, ms));
        STOP_NOW.fetch_add(1, Ordering::Relaxed);
    }

    /// When task `me`'s time is up, if it has a limit.
    pub(super) fn deadline(&self, me: TaskId) -> Option<Instant> {
        self.lock_of(me).task(me)?.deadline.map(|(at, _)| at)
    }

    /// Whether task `me`'s time is up: then it is cancelled, and stopped
    /// for its time (`timed_out`).
    pub(super) fn out_of_time(&self, me: TaskId) -> bool {
        let mut st = self.lock_of(me);
        let up = st
            .task(me)
            .and_then(|t| t.deadline)
            .is_some_and(|(at, _)| Instant::now() >= at);
        if up {
            let t = &mut st.tasks[me & SLOT_MASK];
            (t.cancelled, t.stop_now, t.timed_out) = (true, true, true);
        }
        up
    }

    /// The time limit task `task` stopped at, in milliseconds, if its time
    /// ran out: `.wait()` on it fails with `timed out after … ms`.
    pub fn timed_out(&self, task: TaskId) -> Option<i64> {
        let st = self.lock_of(task);
        let t = st.tasks.get(task & SLOT_MASK)?;
        (t.timed_out && t.generation == super::generation(task)).then_some(t.deadline?.1)
    }

    /// Task `me` has stopped and runs its `drop`s (spec 9.6): no cancel
    /// or time limit stops them. Whether its time ran out is kept, for
    /// `.wait()`'s error.
    pub fn cleaning_up(&self, me: TaskId) {
        let mut st = self.lock_of(me);
        if st.task(me).is_some() {
            let t = &mut st.tasks[me & SLOT_MASK];
            (t.cancelled, t.stop_now) = (false, false);
            // The limit's milliseconds stay for the error; its end moves
            // out of the way.
            let later = Instant::now() + Duration::from_secs(10 * 365 * 86_400);
            t.deadline = t.deadline.map(|(_, ms)| (later, ms));
        }
    }

    /// Cancels `task`, a task that `me` spawned and that ends with it: as
    /// `me` was (spec 9.6), so `task.cancel()` stops its tasks at once too.
    pub fn cancel_with(&self, me: TaskId, task: TaskId) {
        if self.must_stop(me) {
            self.cancel_now(task);
        } else {
            self.cancel(task);
        }
    }
}
