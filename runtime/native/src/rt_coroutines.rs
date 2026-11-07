//! Tasks as coroutines (decision 0048): the main task's thread, the
//! host, runs the tasks it spawns as stackful coroutines (`corosensei`),
//! so passing the turn between them switches stacks, with no system
//! call. The scheduler decides whose turn it is, as for threads
//! (`kavach_runtime_core::tasks::Host`): a coroutine waiting for its turn
//! suspends back to the host, and the host, waiting in its own task,
//! runs the coroutine whose turn has come.
//!
//! What a thread keeps for its task (`Context`) is the running task's:
//! each switch moves the host's out and the coroutine's in, and back.
//! Tasks spawned off the host (from a library call's thread, or by a
//! task that runs ahead there) stay threads (`rt_tasks.rs`).

use crate::rt_tasks;
use corosensei::stack::DefaultStack;
use corosensei::{Coroutine, CoroutineResult, Yielder};
use kavach_runtime_core::tasks::{Host, TaskId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

type Co = Coroutine<(), (), (), DefaultStack>;

/// What this thread keeps for its task, moved out and back in when
/// tasks switch on one thread (`rt_coroutines.rs`).
pub(crate) struct TaskState {
    me: TaskId,
    task: i64,
    limit: usize,
    cancelling: bool,
    has_turn: bool,
    spawned: Vec<i64>,
    marks: Vec<usize>,
    events: Vec<Option<(u64, i64)>>,
}

impl Default for TaskState {
    fn default() -> Self {
        TaskState {
            me: 0,
            task: 0,
            limit: 0,
            cancelling: false,
            has_turn: true,
            spawned: Vec::new(),
            marks: Vec::new(),
            events: Vec::new(),
        }
    }
}

impl TaskState {
    /// This thread's, leaving a fresh one in its place.
    pub(crate) fn take() -> TaskState {
        TaskState {
            me: rt_tasks::ME.replace(0),
            task: rt_tasks::TASK.replace(0),
            limit: rt_tasks::LIMIT.replace(0),
            cancelling: rt_tasks::CANCELLING.replace(false),
            has_turn: rt_tasks::HAS_TURN.replace(true),
            spawned: rt_tasks::SPAWNED.take(),
            marks: rt_tasks::MARKS.take(),
            events: rt_tasks::EVENTS.take(),
        }
    }

    /// Makes this the thread's; `shared`: its stack limit too, as the one
    /// all threads read when tasks take turns (`rt::restore_stack_limit`).
    pub(crate) fn put(self, shared: bool) {
        rt_tasks::ME.set(self.me);
        rt_tasks::TASK.set(self.task);
        rt_tasks::LIMIT.set(self.limit);
        if self.limit != 0 {
            crate::rt::restore_stack_limit(self.limit, shared);
        }
        rt_tasks::CANCELLING.set(self.cancelling);
        rt_tasks::HAS_TURN.set(self.has_turn);
        rt_tasks::SPAWNED.set(self.spawned);
        rt_tasks::MARKS.set(self.marks);
        rt_tasks::EVENTS.set(self.events);
    }
}

/// What a thread keeps for its task, while another runs.
#[derive(Default)]
struct Context {
    state: TaskState,
    error: i64,
    line: String,
    agents: Vec<crate::rt_agents::Inside>,
    scopes: Vec<std::path::PathBuf>,
    /// The coroutine's yielder, its way back to the host; 0 on the host.
    yielder: usize,
}

impl Context {
    /// This thread's, leaving a fresh one in its place.
    fn take() -> Context {
        Context {
            state: TaskState::take(),
            error: crate::rt_errors::swap_error(0),
            line: crate::rt::swap_line(String::new()),
            agents: crate::rt_agents::swap_agents(Vec::new()),
            scopes: crate::rt_files::swap_scopes(Vec::new()),
            yielder: YIELDER.replace(0),
        }
    }

    fn put(self, shared: bool) {
        self.state.put(shared);
        crate::rt_errors::swap_error(self.error);
        crate::rt::swap_line(self.line);
        crate::rt_agents::swap_agents(self.agents);
        crate::rt_files::swap_scopes(self.scopes);
        YIELDER.set(self.yielder);
    }
}

/// A coroutine task, and its context while it doesn't run.
struct Hosted {
    co: Co,
    context: Context,
}

thread_local! {
    /// The host's coroutines that aren't running.
    static HOSTED: RefCell<HashMap<TaskId, Hosted>> = RefCell::new(HashMap::new());
    /// The coroutine running now, if any.
    static CURRENT: Cell<Option<TaskId>> = const { Cell::new(None) };
    /// The running coroutine's yielder (a `*const Yielder<(), ()>`).
    static YIELDER: Cell<usize> = const { Cell::new(0) };
}

thread_local! {
    /// Whether this thread hosts coroutines: the main task's, and each
    /// worker's when tasks run on all cores (`rt_workers.rs`).
    static HOST: Cell<bool> = const { Cell::new(false) };
}

/// Makes this thread the host of the tasks it runs. Only the thread is
/// noted: the scheduler learns of coroutines at the first spawn
/// (`on_host`), so a program without tasks doesn't link them (decision
/// 0024).
pub(crate) fn make_host() {
    HOST.set(true);
}

fn is_host() -> bool {
    HOST.get()
}

/// Whether a task spawned now can become a coroutine: on the host.
/// `spawn thread` asks for a thread instead (spec 9.8).
pub(crate) fn on_host() -> bool {
    static SET: std::sync::Once = std::sync::Once::new();
    let here = is_host();
    if here {
        SET.call_once(|| {
            kavach_runtime_core::tasks::set_host(Host {
                hosts,
                on_host: outside_coroutines,
                suspend,
                resume,
            });
            crate::rt::at_exit(forget_waiting);
        });
    }
    here
}

/// The program ends early (`cli.exit`, a panic) while tasks wait: their
/// coroutines are left as they are. On Windows, ending the process drops
/// this thread's thread-locals, and dropping a waiting coroutine unwinds
/// its stack through native frames, which can't unwind: an abort.
fn forget_waiting() {
    let waiting: Vec<Hosted> = HOSTED.with(|h| h.borrow_mut().drain().map(|(_, c)| c).collect());
    waiting.into_iter().for_each(std::mem::forget);
}

fn hosts(t: TaskId) -> bool {
    CURRENT.get() == Some(t)
        || HOSTED.with(|h| h.borrow().contains_key(&t))
        || crate::rt_workers::inboxed(t)
}

fn outside_coroutines() -> bool {
    is_host() && CURRENT.get().is_none()
}

/// Starts task `id` as a coroutine running `body`, its first turn to come
/// from the scheduler. `false` when no stack could be made.
pub(crate) fn start(id: TaskId, body: impl FnOnce() + 'static) -> bool {
    let Ok(stack) = DefaultStack::new(crate::STACK_SIZE) else {
        return false;
    };
    let co = Coroutine::with_stack(stack, move |y: &Yielder<(), ()>, ()| {
        YIELDER.set(y as *const Yielder<(), ()> as usize);
        body();
    });
    let hosted = Hosted {
        co,
        context: Context::default(),
    };
    HOSTED.with(|h| h.borrow_mut().insert(id, hosted));
    // Its turn wakes the host, wherever the host waits.
    crate::rt_tasks::scheduler().set_thread(id, std::thread::current());
    true
}

/// From the running coroutine: back to the host.
fn suspend() {
    let y = YIELDER.get() as *const Yielder<(), ()>;
    if y.is_null() {
        return;
    }
    // SAFETY: `YIELDER` is the running coroutine's yielder, set when it
    // started and kept in its context while it doesn't run; it lives on
    // that coroutine's stack, which lives while the coroutine does, and a
    // coroutine suspends only itself, on the host thread.
    unsafe { (*y).suspend(()) }
}

/// On the host: runs coroutine `t` until it suspends or ends, with its
/// context in place of the host's.
fn resume(t: TaskId) {
    // A task another worker sent here starts now (`rt_workers.rs`).
    if let Some(body) = crate::rt_workers::take(t) {
        start(t, body);
    }
    let Some(mut hosted) = HOSTED.with(|h| h.borrow_mut().remove(&t)) else {
        return;
    };
    let host = Context::take();
    hosted.context.put(true);
    CURRENT.set(Some(t));
    let result = hosted.co.resume(());
    CURRENT.set(None);
    hosted.context = Context::take();
    // The coroutine may have given the turn to a task on another thread,
    // which now owns the shared stack limit: the host's goes back only if
    // the turn is its task's.
    let mine = host.state.me;
    host.put(rt_tasks::sched().has_turn(mine));
    // A finished coroutine is dropped, and its stack with it.
    if let CoroutineResult::Yield(()) = result {
        HOSTED.with(|h| h.borrow_mut().insert(t, hosted));
    }
}
