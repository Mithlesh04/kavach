//! Tasks on all cores (decision 0049), with `tasks = "parallel"` in
//! `kavach.toml`: a worker thread per core, each hosting coroutines as
//! the main task's thread does (`rt_coroutines.rs`), and a shard of the
//! scheduler for each worker (`kavach_runtime_core::tasks`).
//!
//! A new task goes to the next worker in turn, when its values can be
//! copied there; its body waits in that worker's inbox until the worker
//! makes it a coroutine. Each worker runs a placeholder task that waits forever, so
//! the worker always has a task to run its coroutines from.

use kavach_runtime_core::tasks::{Scheduler, TaskId};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// How many workers, for a program that asked for all cores: a core
/// each when its code keeps a pool and stack limit per thread
/// (`rt::parallel`, LLVM builds), otherwise one.
pub(crate) fn count() -> usize {
    if !crate::rt::parallel() {
        return 1;
    }
    std::thread::available_parallelism().map_or(1, |n| n.get())
}

/// A task's body, waiting for its worker. Its values were copied for it,
/// so only that worker's thread will touch them.
struct Body(Box<dyn FnOnce()>);

// SAFETY: a body holds addresses of objects made for the new task alone
// (`rt_tasks::spawn` copies them), used only by the worker that runs it.
#[allow(unsafe_code)]
unsafe impl Send for Body {}

/// Bodies of a worker's new tasks, until it starts them.
type Inbox = Vec<(TaskId, Body)>;

/// Each worker's inbox. One lock per worker, so a worker checking its
/// own holds up no other.
static INBOXES: OnceLock<Vec<Mutex<Inbox>>> = OnceLock::new();

/// The inbox of worker `worker`; `None` before tasks run on all cores.
fn inbox(worker: usize) -> Option<MutexGuard<'static, Inbox>> {
    let inbox = INBOXES.get()?.get(worker)?;
    Some(inbox.lock().unwrap_or_else(|e| e.into_inner()))
}

fn worker_of(t: TaskId) -> usize {
    crate::rt_tasks::scheduler().worker_of(t)
}

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// The worker a new task goes to: the next one in turn.
pub(crate) fn place(workers: usize) -> usize {
    NEXT.fetch_add(1, Ordering::Relaxed) % workers
}

/// Adds a task for `worker`; `make` gets its id and gives the spawner's
/// value and the task's body. Under the inbox's lock, so the worker,
/// which may take the turn at once, finds the body.
pub(crate) fn send_to(
    s: &Scheduler,
    worker: usize,
    make: impl FnOnce(TaskId) -> (i64, Box<dyn FnOnce()>),
) -> i64 {
    let Some(mut waiting) = inbox(worker) else {
        crate::rt::panic_with("tasks don't run on all cores", 0);
    };
    let id = s.add_on(worker);
    let (value, body) = make(id);
    waiting.push((id, Body(body)));
    value
}

/// Whether task `t` waits in its worker's inbox.
pub(crate) fn inboxed(t: TaskId) -> bool {
    INBOXES.get().is_some() && inbox(worker_of(t)).is_some_and(|w| w.iter().any(|(id, _)| *id == t))
}

/// Task `t`'s body, taken from its worker's inbox.
pub(crate) fn take(t: TaskId) -> Option<Box<dyn FnOnce()>> {
    INBOXES.get()?;
    let mut waiting = inbox(worker_of(t))?;
    let at = waiting.iter().position(|(id, _)| *id == t)?;
    Some(waiting.remove(at).1.0)
}

/// Starts a thread for each worker but the first, the main task's.
pub(crate) fn start(s: &Arc<Scheduler>) {
    kavach_runtime_core::tasks::set_many_workers(s.workers());
    kavach_runtime_core::tasks::set_this_worker(0);
    let _ = INBOXES.set((0..s.workers()).map(|_| Mutex::new(Vec::new())).collect());
    for worker in 1..s.workers() {
        let s = Arc::clone(s);
        let started = std::thread::Builder::new()
            .name(format!("kavach-worker-{worker}"))
            .stack_size(crate::STACK_SIZE)
            .spawn(move || serve(&s, worker));
        if started.is_err() {
            crate::rt::panic_with("can't start a worker thread", 0);
        }
    }
}

/// A worker's thread: its placeholder task waits forever; meanwhile the
/// worker runs the coroutines whose turn comes.
fn serve(s: &Scheduler, worker: usize) {
    crate::rt_coroutines::make_host();
    kavach_runtime_core::tasks::set_this_worker(worker);
    crate::rt::set_stack_limit(crate::STACK_SIZE);
    let me = s.add_on(worker);
    crate::rt_tasks::set_me(me);
    if s.begin(me).is_err() {
        return;
    }
    while s.park(me).is_ok() {}
}
