//! Tasks taking turns (spec 9.7, decision 0020): the one scheduler of
//! the interpreter and native code, so both run tasks in the same order.
//!
//! Each task is a thread, but only the one holding the turn runs. A task
//! hands the turn on when it waits: for another task, for a time, or for
//! a channel (`park`). The next to run is the task that became ready
//! first; when none is ready, the scheduler sleeps until the earliest
//! time a task waits for.
//!
//! A cancelled task (spec 9.6) doesn't wait any more: its waits end at
//! once with `Stop::Cancelled`, and a wait it is in when cancelled ends
//! too. Only the wait for the tasks a function spawned (`join`) can't be
//! cut short, so a task never outlives its function (9.2).
//!
//! Starting threads is the caller's job (the interpreter makes scoped
//! threads, native code plain ones): `add` gives the new task's id, and
//! the thread's first call is `begin(id)`, its last `end(id)`. Task 0 is
//! the program's main task, running from the start.
//!
//! A task waiting for a socket (`wait_io`) is woken by whichever thread
//! waits on the system's readiness queue (`wake_wait`). When no task is
//! ready but some wait for a socket, the scheduler's thread is that one
//! (`set_io_idle`): so no task running is not a deadlock then.
//!
//! With several workers (decision 0049), each has a shard of its own: its
//! tasks, the one it runs, its ready ones and their timers, under its own
//! lock, so workers don't wait for each other's turns. A task's id says
//! its worker. Nothing holds two shards' locks at once: a change to
//! another worker's task takes that shard's lock alone.
//!
//! A task that has ended gives its slot to the next new task of its
//! worker, so a program that spawns tasks forever (a server) keeps a list
//! as long as its tasks alive at once. An id holds the slot in its low 24
//! bits, the worker in the next 8, and the slot's generation above them:
//! an old id no longer names the slot, and the scheduler treats it as a
//! task that has ended.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Instant;

pub type TaskId = usize;

/// Why a wait ended without what it waited for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Every task waits for another one, so none can run again.
    Deadlock,
    /// Another task panicked, so the program is ending: this task should
    /// stop at once (`Scheduler::stop`).
    Stopped,
    /// This task was cancelled (spec 9.6).
    Cancelled,
    /// The time given for the wait has passed (`.timeout`, spec 9.6).
    TimedOut,
    /// This task holds a `Shared` value's lock (inside `update`), where
    /// it may not wait (spec 6.2, decision 0052): the message is `HELD`.
    Held,
}

/// Why a wait inside `Shared.update` is refused (spec 6.2).
pub const HELD: &str = "can't wait inside `update`: the `Shared` value stays locked until its function returns. Get what the update needs first, then update (spec 6.2)";

thread_local! {
    /// How many `Shared` locks this thread's task holds (`hold`).
    static HOLDING: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Whether the running task holds a `Shared` value's lock: then waits
/// that don't go through the scheduler (sleeping or a network wait
/// without tasks, `ask`) must refuse too, with `HELD`.
pub fn holding() -> bool {
    HOLDING.get() > 0
}

/// The running task takes (`true`) or lets go of a `Shared` value's lock:
/// while it holds one, its waits end at once with `Stop::Held`.
pub fn hold(taken: bool) {
    HOLDING.set(if taken {
        HOLDING.get() + 1
    } else {
        HOLDING.get().saturating_sub(1)
    });
}

/// The error of a wait whose time limit of `ms` passed (`.timeout`).
pub fn timed_out(ms: i64) -> String {
    format!("timed out after {ms} ms")
}

pub struct Scheduler {
    shards: Vec<Shard>,
    /// The program is ending (`stop`).
    stopping: AtomicBool,
    /// With several workers: how many sleep with nothing to run, and
    /// whether the last of them found every task waiting for another one
    /// (`pick`).
    asleep: AtomicUsize,
    deadlocked: AtomicBool,
    spinners: AtomicUsize,
    max_spinners: usize,
}

/// One worker's part of the scheduler.
struct Shard {
    state: Mutex<State>,
    /// Wakes the worker's thread waiting in `pick`: for the next timer, a
    /// socket, or a task another worker made ready.
    turn: Condvar,
    /// `State::running`, read without the lock by threads spinning for
    /// their turn (`tasks/turns.rs`).
    running_now: AtomicUsize,
}

/// No task: the turn of a worker that hasn't started yet.
const IDLE: TaskId = usize::MAX;

/// The most workers an id can name.
const MAX_WORKERS: usize = 1 << WORKER_BITS;

struct State {
    /// Whose shard this is.
    worker: usize,
    /// The task the worker runs.
    running: TaskId,
    /// The worker's tasks that are ready, first come first.
    ready: VecDeque<TaskId>,
    /// Tasks waiting for a time, earliest first: (time, order, task, the
    /// task's wait). `order` keeps tasks that wait for the same time in
    /// the order they started waiting.
    timers: BinaryHeap<Reverse<(Instant, u64, TaskId, u64)>>,
    order: u64,
    tasks: Vec<Task>,
    /// Slots of tasks that have ended, for new tasks to take.
    free: Vec<usize>,
    /// Nothing could run, so the turn went to the main task to report a
    /// deadlock.
    stuck: bool,
    /// How many tasks wait for a socket, to be woken from outside.
    io_waiting: usize,
    /// With several workers: how the worker sleeps, waiting for a task,
    /// so a wake-up signals it only then, and in the right way.
    sleeping: Sleep,
}

/// How a worker with nothing to run waits.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Sleep {
    #[default]
    Awake,
    /// On its shard's condition variable.
    OnTurn,
    /// On its socket queue (`set_io_idle`).
    OnQueue,
}

#[derive(Default)]
struct Task {
    /// How many tasks had this slot before this one.
    generation: usize,
    done: bool,
    cancelled: bool,
    /// Cancelled with `task.cancel()`: it stops at loops and calls too,
    /// not only at waits (`stop.rs`, decision 0075).
    stop_now: bool,
    /// `spawn … within d`: when its time is up, and `d` in milliseconds;
    /// then it stops as if cancelled, and has `timed_out`.
    deadline: Option<(Instant, i64)>,
    timed_out: bool,
    /// Counts the task's waits: a wake-up meant for an earlier wait (a
    /// timer that is no longer needed) is ignored.
    wait: u64,
    waiting: bool,
    /// Whether cancelling ends the wait the task is in.
    cancellable: bool,
    /// Whether the wait ended because its time came.
    timer_fired: bool,
    /// A wait woken before it began (`wake_wait` from another thread,
    /// just before the task got to waiting): it ends at once.
    early: u64,
    /// The tasks waiting for this one to end, with their wait.
    waiters: Vec<(TaskId, u64)>,
    /// The task's thread, once it has waited for its turn.
    thread: Option<std::thread::Thread>,
}

impl Scheduler {
    pub fn new() -> Arc<Scheduler> {
        Scheduler::with_workers(1)
    }

    /// A scheduler whose tasks run `workers` at a time, one per worker
    /// thread; the main task is worker 0's.
    pub fn with_workers(workers: usize) -> Arc<Scheduler> {
        let shards = (0..workers.clamp(1, MAX_WORKERS))
            .map(|worker| Shard {
                state: Mutex::new(State {
                    worker,
                    running: if worker == 0 { 0 } else { IDLE },
                    ready: VecDeque::new(),
                    timers: BinaryHeap::new(),
                    order: 0,
                    // Worker 0's first slot is the main task's.
                    tasks: if worker == 0 {
                        vec![Task::default()]
                    } else {
                        Vec::new()
                    },
                    free: Vec::new(),
                    stuck: false,
                    io_waiting: 0,
                    sleeping: Sleep::Awake,
                }),
                turn: Condvar::new(),
                running_now: AtomicUsize::new(if worker == 0 { 0 } else { IDLE }),
            })
            .collect();
        Arc::new(Scheduler {
            shards,
            stopping: AtomicBool::new(false),
            asleep: AtomicUsize::new(0),
            deadlocked: AtomicBool::new(false),
            spinners: AtomicUsize::new(0),
            max_spinners: turns::max_spinners(),
        })
    }

    /// How many tasks run at once.
    pub fn workers(&self) -> usize {
        self.shards.len()
    }

    /// The worker that runs task `t`.
    pub fn worker_of(&self, t: TaskId) -> usize {
        worker(t).min(self.shards.len() - 1)
    }

    fn stopping(&self) -> bool {
        self.stopping.load(Ordering::Acquire)
    }

    /// Wakes worker `w`, whose task became ready, the way it sleeps
    /// (taken from its shard by `wake`).
    fn woke(&self, woken: Option<(usize, Sleep)>) {
        match woken {
            // The worker's own thread, waking a task from its queue, is
            // awake.
            Some((w, _)) if io::is_worker(w) => {}
            Some((w, Sleep::OnTurn)) => self.shards[w].turn.notify_one(),
            Some((w, Sleep::OnQueue)) => io::interrupt(w),
            _ => {}
        }
    }

    /// The shard of worker `worker`, locked.
    fn lock(&self, worker: usize) -> MutexGuard<'_, State> {
        // A task thread that panicked inside the scheduler can't leave
        // its state half changed: every change is complete before any
        // call that could panic.
        let shard = &self.shards[worker.min(self.shards.len() - 1)];
        shard.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The shard of task `t`, locked.
    fn lock_of(&self, t: TaskId) -> MutexGuard<'_, State> {
        self.lock(worker(t))
    }

    /// A new task, ready to run after those already ready.
    pub fn add(&self) -> TaskId {
        self.add_on(0)
    }

    /// A new task run by worker `worker`, ready after its ready tasks.
    pub fn add_on(&self, worker: usize) -> TaskId {
        let worker = worker.min(self.shards.len() - 1);
        let mut st = self.lock(worker);
        let (slot, generation) = match st.free.pop() {
            Some(slot) => (slot, st.tasks[slot].generation + 1),
            None => {
                st.tasks.push(Task::default());
                (st.tasks.len() - 1, 0)
            }
        };
        st.tasks[slot] = Task {
            generation,
            ..Task::default()
        };
        let id = slot | worker << SLOT_BITS | generation.checked_shl(ID_BITS as u32).unwrap_or(0);
        let mut sleep = Sleep::Awake;
        if st.running == IDLE {
            // A worker's first task gets its turn at once.
            self.give_turn(&mut st, id);
        } else {
            st.ready.push_back(id);
            sleep = std::mem::take(&mut st.sleeping);
        }
        drop(st);
        self.woke(Some((worker, sleep)));
        id
    }

    /// Waits until task `me` (just added) may run. `Err` when the program
    /// is ending: the task should not run at all, only `end`.
    pub fn begin(&self, me: TaskId) -> Result<(), Stop> {
        drop(self.await_turn(self.lock_of(me), me));
        if self.stopping() {
            Err(Stop::Stopped)
        } else {
            Ok(())
        }
    }

    /// Task `me` has ended: the tasks waiting for it become ready, and
    /// the next task runs.
    pub fn end(&self, me: TaskId) {
        let mut st = self.lock_of(me);
        let slot = me & SLOT_MASK;
        st.tasks[slot].done = true;
        let waiters = std::mem::take(&mut st.tasks[slot].waiters);
        if me != 0 {
            st.free.push(slot);
        }
        let (here, away): (Vec<_>, Vec<_>) = waiters
            .into_iter()
            .partition(|(t, _)| worker(*t) == st.worker);
        for (t, wait) in here {
            wake(&mut st, t, wait, false);
        }
        if !away.is_empty() {
            drop(st);
            for (t, wait) in away {
                self.wake_one(t, wait);
            }
            st = self.lock_of(me);
        }
        let worker = st.worker;
        let (mut st, next) = self.pick(st, worker);
        match next {
            Some(t) => self.give_turn(&mut st, t),
            // Tasks are left, but all of them wait for each other: the
            // main task reports it.
            None if self.shards.len() == 1 && !st.done(0) => {
                st.stuck = true;
                self.give_turn(&mut st, 0);
            }
            None => {}
        }
    }

    pub fn is_done(&self, task: TaskId) -> bool {
        self.lock_of(task).done(task)
    }

    /// Cancels `task` (spec 9.6): a wait it is in ends, and so will its
    /// next ones.
    pub fn cancel(&self, task: TaskId) {
        let mut st = self.lock_of(task);
        if st.task(task).is_none_or(|t| t.done || t.cancelled) {
            return;
        }
        let t = &mut st.tasks[task & SLOT_MASK];
        t.cancelled = true;
        if t.cancellable {
            let wait = t.wait;
            let w = wake(&mut st, task, wait, false);
            drop(st);
            self.woke(w);
        }
    }

    pub fn is_cancelled(&self, task: TaskId) -> bool {
        self.lock_of(task).task(task).is_some_and(|t| t.cancelled)
    }

    /// The parked task `task` becomes ready (if it still waits).
    pub fn unpark(&self, task: TaskId) {
        let mut st = self.lock_of(task);
        let Some(wait) = st.task(task).map(|t| t.wait) else {
            return;
        };
        let w = wake(&mut st, task, wait, false);
        drop(st);
        self.woke(w);
    }

    /// Wait `wait` of `task`, another worker's, is over.
    fn wake_one(&self, task: TaskId, wait: u64) {
        let w = wake(&mut self.lock_of(task), task, wait, false);
        self.woke(w);
    }

    /// The program is ending (a task panicked): from now on every task
    /// that waits, or is waiting, stops instead.
    pub fn stop(&self) {
        self.stopping.store(true, Ordering::Release);
        self.wake_everyone();
    }

    /// Wakes every sleeping worker, to look at `stopping` or `deadlocked`.
    fn wake_everyone(&self) {
        for (w, shard) in self.shards.iter().enumerate() {
            // Under the lock, so a worker that just found it false is
            // already waiting.
            let mut st = self.lock(w);
            shard.turn.notify_all();
            if std::mem::take(&mut st.sleeping) == Sleep::OnQueue {
                io::interrupt(w);
            }
        }
    }
}

/// The low bits of an id: its slot in `State::tasks`.
const SLOT_BITS: usize = 24;
const SLOT_MASK: usize = (1 << SLOT_BITS) - 1;
/// The bits above the slot: the worker.
const WORKER_BITS: usize = 8;
/// The bits of the slot and worker; the generation is above them.
const ID_BITS: usize = SLOT_BITS + WORKER_BITS;

/// The worker of task `t`.
fn worker(t: TaskId) -> usize {
    (t >> SLOT_BITS) & (MAX_WORKERS - 1)
}

/// The generation packed above the worker. A 32-bit target has no room
/// for it, and none to run tasks either: without threads (the browser
/// playground, decision 0078) the scheduler never starts.
pub(crate) fn generation(t: TaskId) -> usize {
    t.checked_shr(ID_BITS as u32).unwrap_or(0)
}

impl State {
    /// Task `id`, while its slot still holds it (not a later task).
    fn task(&self, id: TaskId) -> Option<&Task> {
        self.tasks
            .get(id & SLOT_MASK)
            .filter(|t| t.generation == generation(id) && worker(id) == self.worker)
    }

    /// Whether task `id` has ended: an id whose slot moved on has.
    fn done(&self, id: TaskId) -> bool {
        self.task(id).is_none_or(|t| t.done)
    }
}

/// Wait `wait` of task `t` is over: `t` becomes ready, if it is still in
/// that wait (and `t` still names its slot). When it became ready, gives
/// its worker and how that sleeps, for `Scheduler::woke`.
fn wake(st: &mut State, t: TaskId, wait: u64, by_timer: bool) -> Option<(usize, Sleep)> {
    st.task(t)?;
    let task = &mut st.tasks[t & SLOT_MASK];
    if task.waiting && task.wait == wait {
        task.waiting = false;
        task.timer_fired = by_timer;
        st.ready.push_back(t);
        return Some((st.worker, std::mem::take(&mut st.sleeping)));
    }
    None
}

mod io;
mod pick;
mod stop;
pub use io::{
    interrupt_queue_of, many_workers, set_io_idle, set_many_workers, set_this_worker, this_worker,
    worker_count,
};
pub use stop::any_stop_now;
pub use turns::{Host, set_host};
#[cfg(test)]
mod tests;
mod turns;
mod wait;
