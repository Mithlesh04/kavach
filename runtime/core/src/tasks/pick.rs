//! Choosing the next task a worker runs (`pick`): the tasks whose time
//! has come become ready, then the first ready one runs; with none, the
//! worker waits for a time, a socket, or another worker's wake-up.

use super::{Scheduler, Sleep, State, TaskId, wake};
use std::cmp::Reverse;
use std::sync::MutexGuard;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

impl Scheduler {
    /// Whether every worker has nothing to run and nothing to wait for
    /// that could make a task ready: no timer, no socket. Takes each
    /// shard's lock in turn, never two at once.
    fn all_quiet(&self) -> bool {
        (0..self.shards.len()).all(|w| {
            let st = self.lock(w);
            let live = st.timers.iter().any(|Reverse((_, _, t, wait))| {
                st.task(*t).is_some_and(|t| t.waiting && t.wait == *wait)
            });
            st.ready.is_empty() && st.io_waiting == 0 && !live
        })
    }

    /// The next task `worker` runs: tasks whose time has come join the
    /// ready ones, and the one ready first runs. With none ready, waits
    /// for the earliest time. `None` when no task can ever run again.
    /// With several workers, another may make a task ready, so it waits
    /// for that instead (sockets are then watched by a thread of their
    /// own, `set_io_idle`).
    pub(super) fn pick<'s>(
        &'s self,
        mut st: MutexGuard<'s, State>,
        worker: usize,
    ) -> (MutexGuard<'s, State>, Option<TaskId>) {
        loop {
            let many = self.shards.len() > 1;
            if st.io_waiting > 0 && st.ready.is_empty() && !many {
                // Sockets ready now, without waiting.
                (st, _) = self.io_idle(st, Some(Duration::ZERO));
            }
            let now = Instant::now();
            while let Some(&Reverse((at, _, t, wait))) = st.timers.peek() {
                if at > now && !self.stopping() {
                    break;
                }
                st.timers.pop();
                // A shard's timers are its own tasks'.
                wake(&mut st, t, wait, true);
            }
            if let Some(t) = st.ready.pop_front() {
                return (st, Some(t));
            }
            // Timers for waits that already ended don't count.
            let live = |s: &State, t: TaskId, wait: u64| {
                s.task(t).is_some_and(|t| t.waiting && t.wait == wait)
            };
            while let Some(&Reverse((_, _, t, wait))) = st.timers.peek() {
                if live(&st, t, wait) {
                    break;
                }
                st.timers.pop();
            }
            let next = st.timers.peek().map(|&Reverse((at, ..))| at);
            let turn = &self.shards[worker].turn;
            if many && !self.stopping() {
                // Another worker, or a socket, may make a task ready: the
                // worker sleeps on its own queue once it has one, and says
                // how, so a wake-up knows how to reach it.
                let left = next.map(|at| at.saturating_duration_since(now));
                if self.deadlocked.load(Ordering::Acquire) {
                    return (st, None);
                }
                let asleep = self.asleep.fetch_add(1, Ordering::AcqRel) + 1;
                if asleep == self.shards.len() && next.is_none() && st.io_waiting == 0 {
                    // The last worker to have nothing to do: if no task is
                    // ready, waits for a time or for a socket anywhere,
                    // every task waits for another one.
                    drop(st);
                    let quiet = self.all_quiet();
                    st = self.lock(worker);
                    let still = self.asleep.load(Ordering::Acquire) == self.shards.len();
                    if quiet && still && st.ready.is_empty() {
                        self.asleep.fetch_sub(1, Ordering::AcqRel);
                        self.deadlocked.store(true, Ordering::Release);
                        drop(st);
                        self.wake_everyone();
                        st = self.lock(worker);
                        return (st, None);
                    }
                }
                if super::io::has_queue() {
                    st.sleeping = Sleep::OnQueue;
                    (st, _) = self.io_idle(st, left);
                } else {
                    st.sleeping = Sleep::OnTurn;
                    st = match left {
                        Some(d) => turn
                            .wait_timeout(st, d)
                            .map(|r| r.0)
                            .unwrap_or_else(|e| e.into_inner().0),
                        None => turn.wait(st).unwrap_or_else(|e| e.into_inner()),
                    };
                }
                st.sleeping = Sleep::Awake;
                self.asleep.fetch_sub(1, Ordering::AcqRel);
                continue;
            }
            if st.io_waiting > 0 && !self.stopping() {
                // Wait for a socket, or the next time, on the queue.
                let left = next.map(|at| at.saturating_duration_since(now));
                let (back, polled) = self.io_idle(st, left);
                st = back;
                if !polled {
                    // No queue: the waker comes from another thread
                    // (`wake_wait`), through the condition variable.
                    st = match left {
                        Some(d) => turn
                            .wait_timeout(st, d)
                            .map(|r| r.0)
                            .unwrap_or_else(|e| e.into_inner().0),
                        None => turn.wait(st).unwrap_or_else(|e| e.into_inner()),
                    };
                }
                continue;
            }
            let Some(at) = next else {
                return (st, None);
            };
            // Only this thread runs, so sleeping here holds up no one.
            st = turn
                .wait_timeout(st, at.saturating_duration_since(now))
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }
}
