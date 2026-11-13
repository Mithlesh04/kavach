//! Tasks in native code (spec 9, decisions 0020 and 0048). They take
//! turns through the scheduler the interpreter uses too
//! (`kavach_runtime_core::tasks`), so both run tasks in the same order.
//!
//! A task spawned on the main task's thread is a coroutine on that
//! thread (`rt_coroutines.rs`): passing the turn switches stacks. A task
//! spawned elsewhere (a library call's thread, decision 0035) is a thread
//! of its own, and the turn passes through a mutex, which orders memory
//! between the threads. Only the task holding the turn runs, so
//! reference counts, the allocation pool and `kv_stack_limit` need no
//! atomics; the stack limit is set again whenever a task's turn comes.
//!
//! Off the main thread, in a program built by `kavach build`
//! (`rt::parallel`), a new task whose values can be copied (`send.rs`)
//! runs ahead: its thread starts computing at once, on its own copies,
//! and waits for its turn only before its first effect another task
//! could see (`turn`): printing, a panic, files, channels, spawning,
//! waiting, ending.
//!
//! `spawn` hands the runtime a closure around the task's *thunk*
//! (`compiler/kir/src/lower_tasks.rs`), which runs the task and hands its
//! value or error back through `TaskDone` or `TaskFail`.

use crate::heap::{Data, alloc, release, release_slot, retain};
use crate::rt::{panic_with, type_of};
use crate::shapes::shape;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_core::tasks::{Scheduler, TaskId};
use std::cell::{Cell, RefCell};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

pub use crate::rt_waits::sleep_until;
pub(crate) use crate::rt_waits::{
    cancelling, me, note_cancel, park, refuse_if_held, resumed, unpark, wait_outside,
};

/// What a `Task` object holds: the task, and once it has ended, its
/// value and its error (one of them 0).
pub struct TaskObj {
    id: TaskId,
    pub(crate) result: Cell<Option<(u64, i64)>>,
    /// A `.wait()` read the result; the task was cancelled (so its error
    /// is none); the line it was spawned at: for an error nobody reads.
    pub(crate) read: Cell<bool>,
    pub(crate) quiet: Cell<bool>,
    pub(crate) line: i32,
    /// The event it runs again after a restart: `event #17 of request`.
    pub(crate) event: std::cell::RefCell<Option<String>>,
}

pub(crate) static SCHED: OnceLock<Arc<Scheduler>> = OnceLock::new();

pub(crate) fn sched() -> &'static Arc<Scheduler> {
    SCHED.get_or_init(Scheduler::new)
}

thread_local! {
    /// The task this thread runs: 0 is `main`.
    pub(crate) static ME: Cell<TaskId> = const { Cell::new(0) };
    /// The `Task` object of this thread's task (0 for `main`).
    pub(crate) static TASK: Cell<i64> = const { Cell::new(0) };
    /// This thread's stack limit, for `kv_stack_limit` when its turn comes.
    pub(crate) static LIMIT: Cell<usize> = const { Cell::new(0) };
    /// The task is cancelled and on its way out (spec 9.6): a wait ended
    /// because of it, and generated code returns from each function
    /// until the task's thunk (`compiler/kir/src/cancel.rs`).
    pub(crate) static CANCELLING: Cell<bool> = const { Cell::new(false) };
    /// Whether this thread holds its task's turn: false while a task runs
    /// ahead, before its first visible effect.
    pub(crate) static HAS_TURN: Cell<bool> = const { Cell::new(true) };
    /// The tasks this task spawned that no join has waited for yet
    /// (each holds a reference).
    pub(crate) static SPAWNED: RefCell<Vec<i64>> = const { RefCell::new(Vec::new()) };
    /// The results of the sends to agents running now, innermost last
    /// (`rt_events.rs`): an event's thunk hands its value or error here.
    pub(crate) static EVENTS: RefCell<Vec<Option<(u64, i64)>>> = const { RefCell::new(Vec::new()) };
    /// The marks of the functions running on this task that spawn tasks,
    /// innermost last: `SPAWNED` from the last one on is that function's.
    pub(crate) static MARKS: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

/// This thread's task is `me` (a worker's placeholder, `rt_workers.rs`).
pub(crate) fn set_me(me: TaskId) {
    ME.set(me);
    HAS_TURN.set(true);
}

/// The scheduler, for the coroutines' host (`rt_coroutines.rs`).
pub(crate) fn scheduler() -> &'static Arc<Scheduler> {
    sched()
}

/// Before an effect other tasks could see: a task running ahead waits
/// for its turn, so effects happen in the order of turns (spec 9.7).
pub(crate) fn turn() {
    crate::lib_calls::turn();
    if !HAS_TURN.get() {
        let _ = sched().begin(ME.get());
        HAS_TURN.set(true);
    }
}

/// A time limit on a wait (`.timeout`, spec 9.6): when it ends, and the
/// milliseconds it was given (for the error).
pub(crate) type Limit = Option<(Instant, i64)>;

/// The limit of `ms` milliseconds from now. In a durable handler its end
/// is journaled, so after a restart it ends when it would have (spec 8.5,
/// decision 0062).
pub(crate) fn limit(ms: i64, line: i32) -> Limit {
    let ms = ms.max(0);
    let left = crate::rt_std::journaled_wait("time.timeout", ms, line);
    Some((Instant::now() + std::time::Duration::from_millis(left), ms))
}

/// The task ops (all operands owned). `code` is the result type.
pub fn task_op(op: RtOp, a: i64, b: i64, code: i64, line: i32) -> i64 {
    // A function's start or a loop's round (`stop_points.rs`): one read
    // of a flag unless some task was cancelled with `cancel()`.
    if op == RtOp::TaskStopPoint {
        let stop = kavach_runtime_core::tasks::any_stop_now()
            && SCHED.get().is_some_and(|s| s.must_stop(ME.get()));
        if stop {
            CANCELLING.set(true);
        }
        return i64::from(stop);
    }
    if !matches!(op, RtOp::TaskMark | RtOp::TaskCancelling) {
        turn();
    }
    match op {
        RtOp::TaskSpawn => spawn(a, b, code, line),
        // First of all (`lower_drops.rs`): the scheduler gets a worker
        // per core. Here, not in `sched`, so a program without it doesn't
        // link the workers (decision 0024).
        RtOp::TaskParallel => {
            let workers = crate::rt_workers::count();
            if workers > 1 && SCHED.set(Scheduler::with_workers(workers)).is_ok() {
                crate::rt_workers::start(sched());
            }
            0
        }
        RtOp::TaskCancelling => i64::from(CANCELLING.get()),
        // `task.cancel()` (spec 9.6): it stops at its next wait or stop
        // point, even while it computes.
        RtOp::TaskCancel => {
            sched().cancel_now(task_of(a).0);
            release(a);
            0
        }
        // `spawn … within b` (9.6): task `a`, given back, stops once `b`
        // milliseconds have passed.
        RtOp::TaskWithin => {
            sched().set_within(task_of(a).0, b);
            a
        }
        RtOp::TaskWait | RtOp::TaskWaitFor => {
            let (id, value_ty) = task_of(a);
            let limit = if op == RtOp::TaskWaitFor {
                limit(b, line)
            } else {
                None
            };
            if !wait(id, limit, line) {
                release(a);
                return 0;
            }
            with_task(a, |t| t.read.set(true));
            let result = match data_of(a) {
                Some((value, 0)) => {
                    if value_ty.is_ref() {
                        retain(value as i64);
                    }
                    value as i64
                }
                Some((_, error)) => {
                    retain(error);
                    crate::rt_errors::set_error(error);
                    0
                }
                None => panic_with("the task has no result", line),
            };
            release(a);
            result
        }
        RtOp::TaskMark => {
            let mark = SPAWNED.with(|s| s.borrow().len());
            MARKS.with(|m| m.borrow_mut().push(mark));
            mark as i64
        }
        // Waiting is in `turn` order, and copies share the task's cell.
        RtOp::TaskJoin => {
            MARKS.with(|m| m.borrow_mut().pop());
            join(a.max(0) as usize, line);
            0
        }
        RtOp::TaskDone | RtOp::TaskFail => {
            let task = TASK.get();
            let pair = if op == RtOp::TaskDone {
                (a as u64, 0)
            } else {
                (0, a)
            };
            // The thunk of a send to an agent gives its sender the result.
            let event = EVENTS.with(|e| e.borrow_mut().last_mut().map(|r| *r = Some(pair)));
            if event.is_some() {
                return 0;
            }
            if task != 0 {
                with_task(task, |t| t.result.set(Some(pair)));
                if op == RtOp::TaskFail {
                    crate::rt_task_errors::note_failed(task);
                }
            }
            0
        }
        _ => {
            let _ = code;
            0
        }
    }
}

/// A new task running `closure` (owned), as a `Task` of type `code`.
/// How a task runs (spec 9.8, decision 0069): the second operand of
/// `TaskSpawn`.
const THREAD: i64 = 1;
const CORES: i64 = 2;

/// A coroutine task running closure `closure`: an event run again after
/// a restart (`rt_replays.rs`).
pub(crate) fn spawn_task(closure: i64, code: i64, line: i32) -> i64 {
    spawn(closure, 0, code, line)
}

/// The scheduler's id of task object `task`.
pub(crate) fn task_id(task: i64) -> TaskId {
    task_of(task).0
}

fn spawn(closure: i64, kind: i64, code: i64, line: i32) -> i64 {
    let s = sched();
    let copyable = crate::send::copyable(closure as u64, crate::heap::obj(closure).ty());
    // A coroutine when it can be one: on the host, unless it asked for a
    // thread of its own.
    let host = kind != THREAD && crate::rt_coroutines::on_host();
    // `spawn cores`, with a worker per core (decision 0049): the next
    // worker in turn, when the task's values can be copied there.
    let away = (kind == CORES && s.workers() > 1 && copyable && host)
        .then(|| crate::rt_workers::place(s.workers()))
        .filter(|w| *w != s.worker_of(ME.get()));
    // A task that runs ahead gets its own copy of everything its closure
    // holds, so the two threads share no object, as does a task on
    // another worker. On the host, a task is a coroutine instead.
    let ahead = away.is_none() && crate::rt::parallel() && !host && copyable;
    let closure = if ahead || away.is_some() {
        let copied = crate::send::copy(closure as u64, crate::heap::obj(closure).ty());
        release(closure);
        copied as i64
    } else {
        closure
    };
    // Addresses can cross threads; the objects behind them are only used
    // by the thread holding the turn, or by a task running ahead on its
    // own copies.
    let closure = closure as usize;
    // A task keeps the folders file access is kept inside (spec 8.1).
    let scopes = crate::rt_files::scopes();
    // And runs inside the agent running where it was spawned (spec 8.3).
    let agent = crate::rt_agents::inherited();
    let ty = type_of(code);
    // The task object, and the task's own reference to it: on another
    // worker, a copy around the same cell, so no thread shares an object.
    let objects = move |id: TaskId| -> (i64, i64) {
        let task = alloc(
            ty,
            Data::Task(std::sync::Arc::new(TaskObj {
                id,
                result: Cell::new(None),
                read: Cell::new(false),
                quiet: Cell::new(false),
                line,
                event: std::cell::RefCell::new(None),
            })),
        );
        // One reference each for the program and the join of the function
        // that spawned it; the task's own comes next.
        retain(task);
        let theirs = if away.is_some() {
            crate::send::copy(task as u64, ty) as i64
        } else {
            retain(task);
            task
        };
        (task, theirs)
    };
    let body = move |id: TaskId, theirs: i64| {
        let object = theirs as usize;
        move || {
            crate::rt_files::set_scopes(scopes);
            crate::rt_agents::inherit(agent);
            run(id, closure as i64, object as i64, ahead)
        }
    };
    if let Some(worker) = away {
        let task = crate::rt_workers::send_to(s, worker, |id| {
            let (task, theirs) = objects(id);
            (task, Box::new(body(id, theirs)))
        });
        let_go_of_finished();
        SPAWNED.with(|s| s.borrow_mut().push(task));
        return task;
    }
    let id = s.add_on(s.worker_of(ME.get()));
    let (task, theirs) = objects(id);
    let_go_of_finished();
    SPAWNED.with(|s| s.borrow_mut().push(task));
    let body = body(id, theirs);
    // A coroutine on the host (decision 0048); a thread elsewhere (a
    // library call's thread, a task running ahead).
    let started = if !ahead && host {
        crate::rt_coroutines::start(id, body)
    } else {
        std::thread::Builder::new()
            .stack_size(crate::STACK_SIZE)
            .spawn(body)
            .is_ok()
    };
    if !started {
        panic_with("can't start the task", 0);
    }
    task
}

/// The thread of task `id`: runs the thunk of `closure` when its turn
/// comes.
fn run(id: TaskId, closure: i64, task: i64, ahead: bool) {
    let s = sched();
    ME.set(id);
    HAS_TURN.set(!ahead);
    if ahead || s.begin(id).is_ok() {
        TASK.set(task);
        crate::rt::set_stack_limit(crate::STACK_SIZE);
        let thunk = crate::heap::slots(closure)[0] as usize;
        // SAFETY: slot 0 of a closure is its function's address, and the
        // code generator compiled a task's thunk as `fn(closure)`; the
        // code lives as long as the program. The thunk borrows the closure
        // (`kavach_kir::borrows`), which is let go of after.
        let thunk: extern "C-unwind" fn(i64) = unsafe { std::mem::transmute(thunk) };
        // A panic stops only this task (spec 9.1, decision 0065): it
        // unwinds to here, and `.wait()` gets it as an error.
        let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| thunk(closure)));
        release(closure);
        match ran {
            // What it held is let go of, and the `drop`s run now, even when
            // it was cancelled: no cancel check stops them (spec 9.6).
            Ok(()) => {
                let was = CANCELLING.replace(false);
                if was {
                    s.cleaning_up(id);
                }
                crate::rc::run_waiting_drops();
                CANCELLING.set(was);
            }
            Err(payload) => crate::rt_task_errors::panicked(task, payload),
        }
        // The thunk's body joined its own tasks; nothing is left.
        turn();
        join(0, 0);
        crate::rt_agents::leave();
        if CANCELLING.replace(false) {
            crate::rt_task_errors::cancelled(task);
        }
        release(task);
    }
    crate::heap::flush_live();
    s.end(id);
}

/// Waits (letting other tasks run) until task `id` has ended; `false`
/// when this task is cancelled instead.
fn wait(id: TaskId, limit: Limit, line: i32) -> bool {
    let me = ME.get();
    let waited = match limit {
        Some((at, _)) => sched().wait_for_until(me, id, at),
        None => sched().wait_for(me, id),
    };
    resumed(waited, limit, line)
}

/// A loop that never returns (a server's) would keep every task it
/// spawned until its function returns. As the running function's share of
/// `SPAWNED` doubles, its finished tasks are let go of: the join would
/// skip them anyway, and a handle the program holds keeps the result.
fn let_go_of_finished() {
    let from = MARKS.with(|m| m.borrow().last().copied().unwrap_or(0));
    SPAWNED.with(|s| {
        let mut s = s.borrow_mut();
        let mine = s.len().saturating_sub(from);
        if mine < 64 || !mine.is_power_of_two() {
            return;
        }
        let sched = sched();
        let kept: Vec<i64> = s
            .drain(from..)
            .filter(|&task| {
                let done = sched.is_done(task_of(task).0);
                if done {
                    release(task);
                }
                !done
            })
            .collect();
        s.extend(kept);
    });
}

/// The tasks this task spawned after the first `mark`: those still
/// running are cancelled, then each is waited for, in order, and let go
/// of (spec 9.2).
fn join(mark: usize, line: i32) {
    let tasks = SPAWNED.with(|s| {
        let mut s = s.borrow_mut();
        let from = mark.min(s.len());
        s.split_off(from)
    });
    let s = sched();
    // A task cancelled with `cancel()` stops its tasks at once too.
    for &task in &tasks {
        s.cancel_with(ME.get(), task_of(task).0);
    }
    for task in tasks {
        let (id, _) = task_of(task);
        let joined = s.join(ME.get(), id);
        resumed(joined, None, line);
        release(task);
    }
}

pub(crate) fn with_task<R>(task: i64, f: impl FnOnce(&TaskObj) -> R) -> R {
    match &*crate::heap::data(task).borrow() {
        Data::Task(t) => f(t),
        _ => unreachable!("a task object"),
    }
}

/// The task's id and value type.
fn task_of(task: i64) -> (TaskId, Type) {
    let value_ty = match shape(crate::heap::obj(task).ty()) {
        Shape::Task(t) => *t,
        _ => Type::Unit,
    };
    (with_task(task, |t| t.id), value_ty)
}

fn data_of(task: i64) -> Option<(u64, i64)> {
    with_task(task, |t| t.result.get())
}

/// Releases what a `Task` object holds, when it is freed. An error nobody
/// read is printed then.
pub fn free_task(t: &TaskObj, value_ty: Type) {
    crate::rt_task_errors::unread(t);
    if let Some((value, error)) = t.result.get() {
        release_slot(value as i64, value_ty);
        release(error);
    }
}

// The group's runtime entry (`RtOp::entry`, decision 0024).
crate::rt_data::entry!(kv_rt_tasks, |op, a, b, c, code, line| {
    if matches!(
        op,
        RtOp::ChanNew
            | RtOp::ChanPut
            | RtOp::ChanClose
            | RtOp::ChanTake
            | RtOp::ChanTakeValue
            | RtOp::ChanEnded
            | RtOp::ChanTakeFor
            | RtOp::ChanPutFor
            | RtOp::ChanTryPut
    ) {
        crate::rt_channels::chan_op(op, a, b, c, code, line)
    } else {
        task_op(op, a, b, code, line)
    }
});
