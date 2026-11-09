//! A task waiting: for another task to end, for a time, or to be
//! unparked. The next task of its worker runs meanwhile.

use super::{SLOT_MASK, Scheduler, State, Stop, TaskId, wake, worker};
use std::cmp::Reverse;
use std::sync::MutexGuard;
use std::time::Instant;

impl Scheduler {
    /// Task `me` waits until task `other` has ended.
    pub fn wait_for(&self, me: TaskId, other: TaskId) -> Result<(), Stop> {
        self.wait(me, Some(other), None, true)
    }

    /// Like `wait_for`, but only until `at`: then `Stop::TimedOut`.
    pub fn wait_for_until(&self, me: TaskId, other: TaskId, at: Instant) -> Result<(), Stop> {
        self.wait(me, Some(other), Some(at), true)
    }

    /// Like `wait_for`, but even when `me` is cancelled: a function waits
    /// for the tasks it spawned before it returns (spec 9.2).
    pub fn join(&self, me: TaskId, other: TaskId) -> Result<(), Stop> {
        self.wait(me, Some(other), None, false)
    }

    /// Task `me` waits until `at`.
    pub fn sleep_until(&self, me: TaskId, at: Instant) -> Result<(), Stop> {
        self.wait(me, None, Some(at), true).or_else(|s| match s {
            Stop::TimedOut => Ok(()),
            other => Err(other),
        })
    }

    /// Task `me` waits until another task calls `unpark(me)`: for room
    /// in a channel, or a value from one (spec 9.4).
    pub fn park(&self, me: TaskId) -> Result<(), Stop> {
        self.wait(me, None, None, true)
    }

    /// Like `park`, but only until `at`: then `Stop::TimedOut`.
    pub fn park_until(&self, me: TaskId, at: Instant) -> Result<(), Stop> {
        self.wait(me, None, Some(at), true)
    }

    /// Task `me` waits: for task `on` to end, for the time `until`, or
    /// (with neither) for `unpark`. The next task runs meanwhile.
    pub(super) fn wait(
        &self,
        me: TaskId,
        on: Option<TaskId>,
        until: Option<Instant>,
        cancellable: bool,
    ) -> Result<(), Stop> {
        if on.is_some_and(|o| self.is_done(o)) {
            return Ok(());
        }
        if self.stopping() {
            return Err(Stop::Stopped);
        }
        if super::HOLDING.get() > 0 {
            return Err(Stop::Held);
        }
        let mine = me & SLOT_MASK;
        // A task given a time limit (`within`) waits until it at most.
        let limit = self.deadline(me).filter(|_| cancellable);
        let until = match (until, limit) {
            (Some(u), Some(l)) => Some(u.min(l)),
            (u, l) => u.or(l),
        };
        loop {
            let mut st = self.lock_of(me);
            if cancellable && st.tasks[mine].cancelled {
                return Err(Stop::Cancelled);
            }
            let t = &mut st.tasks[mine];
            t.wait += 1;
            if std::mem::take(&mut t.early) == t.wait {
                return Ok(());
            }
            t.waiting = true;
            t.cancellable = cancellable;
            t.timer_fired = false;
            let wait = t.wait;
            if let Some(at) = until {
                let order = st.order;
                st.order += 1;
                st.timers.push(Reverse((at, order, me, wait)));
            }
            if let Some(o) = on {
                st = self.wait_on(st, me, wait, o);
            }
            let st = self.hand_on(st, me)?;
            if self.stopping() {
                return Err(Stop::Stopped);
            }
            if cancellable && st.tasks[mine].cancelled {
                return Err(Stop::Cancelled);
            }
            let fired = st.tasks[mine].timer_fired;
            drop(st);
            if fired && limit.is_some() && self.out_of_time(me) {
                return Err(Stop::Cancelled);
            }
            let done = on.is_none_or(|o| self.is_done(o));
            if fired && (!done || on.is_none()) {
                return Err(Stop::TimedOut);
            }
            if done {
                return Ok(());
            }
            // Woken for nothing (an `unpark` meant for an earlier wait):
            // wait again.
        }
    }

    /// Wait `wait` of task `me` lasts until task `o` ends: `me` joins
    /// `o`'s waiters, or becomes ready if `o` has ended already. Another
    /// worker's `o` is changed under its own shard's lock alone.
    fn wait_on<'s>(
        &'s self,
        mut st: MutexGuard<'s, State>,
        me: TaskId,
        wait: u64,
        o: TaskId,
    ) -> MutexGuard<'s, State> {
        let added = if worker(o) == st.worker {
            add_waiter(&mut st, o, me, wait)
        } else {
            drop(st);
            let added = add_waiter(&mut self.lock_of(o), o, me, wait);
            st = self.lock_of(me);
            added
        };
        if !added {
            wake(&mut st, me, wait, false);
        }
        st
    }

    /// Task `me` has started waiting: the next task runs, and `me` runs
    /// again when its turn comes.
    fn hand_on<'s>(
        &'s self,
        st: MutexGuard<'s, State>,
        me: TaskId,
    ) -> Result<MutexGuard<'s, State>, Stop> {
        let worker = st.worker;
        let (mut st, next) = self.pick(st, worker);
        let Some(next) = next else {
            // Nothing can run, `me` included.
            st.tasks[me & SLOT_MASK].waiting = false;
            return Err(if self.stopping() {
                Stop::Stopped
            } else {
                Stop::Deadlock
            });
        };
        if next != me {
            self.give_turn(&mut st, next);
            st = self.await_turn(st, me);
        }
        if std::mem::take(&mut st.stuck) {
            st.tasks[me & SLOT_MASK].waiting = false;
            return Err(Stop::Deadlock);
        }
        Ok(st)
    }
}

/// Task `me`'s wait `wait` joins the waiters of `o`, in `o`'s shard;
/// `false` when `o` has ended.
fn add_waiter(st: &mut State, o: TaskId, me: TaskId, wait: u64) -> bool {
    if st.done(o) {
        return false;
    }
    st.tasks[o & SLOT_MASK].waiters.push((me, wait));
    true
}
