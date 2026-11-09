//! Waiting in native code (spec 9): for a time, for something outside the
//! program (a socket), or to be unparked (a channel), while other tasks
//! run; and what a wait that ended early means for the task (`resumed`).
//! Waits inside `Shared.update` are refused (spec 6.2).

use crate::rt::panic_with;
use crate::rt_tasks::{CANCELLING, LIMIT, Limit, ME, SCHED, sched, turn};
use kavach_kir::Shape;
use kavach_runtime_core::tasks::{Stop, TaskId};
use std::sync::Arc;
use std::time::Instant;

/// `time.sleep_ms` and the rounds of `time.every_ms`: other tasks run
/// meanwhile. Without tasks, the thread just sleeps.
pub fn sleep_until(at: Instant, line: i32) {
    turn();
    match SCHED.get() {
        Some(s) => {
            resumed(s.sleep_until(ME.get(), at), None, line);
        }
        None => {
            refuse_if_held(line);
            std::thread::sleep(at.saturating_duration_since(Instant::now()));
        }
    }
}

/// Panics when this task waits inside `Shared.update` (spec 6.2), for
/// waits that don't go through the scheduler, which refuses its own.
pub(crate) fn refuse_if_held(line: i32) {
    if kavach_runtime_core::tasks::holding() {
        panic_with(kavach_runtime_core::tasks::HELD, line);
    }
}

/// Waits for something outside the program (a socket, decision 0046)
/// while other tasks run: `register` gets the waker, which whoever sees
/// it happen calls, and gives `false` when there is no need to wait (it
/// happened already). Gives up at `at`, and the caller looks again.
/// `false` without tasks: then the caller waits alone.
pub(crate) fn wait_outside(
    at: Option<Instant>,
    line: i32,
    register: impl FnOnce(Box<dyn FnOnce() + Send>) -> bool,
) -> bool {
    turn();
    let Some(s) = SCHED.get() else {
        return false;
    };
    let (me, sched) = (ME.get(), Arc::clone(s));
    let wait = s.next_wait(me);
    if register(Box::new(move || sched.wake_wait(me, wait))) {
        resumed(s.wait_io(me, at), None, line);
    }
    true
}

/// Whether this task is being cancelled (its last wait ended for that).
pub(crate) fn cancelling() -> bool {
    CANCELLING.get()
}

/// The running task.
pub(crate) fn me() -> TaskId {
    ME.get()
}

/// Waits until another task calls `unpark` for this one (a channel);
/// `false` when this task is cancelled instead.
pub(crate) fn park(limit: Limit, line: i32) -> bool {
    let waited = match limit {
        Some((at, _)) => sched().park_until(ME.get(), at),
        None => sched().park(ME.get()),
    };
    resumed(waited, limit, line)
}

/// The parked task `task` becomes ready.
pub(crate) fn unpark(task: TaskId) {
    sched().unpark(task);
}

/// Back from waiting: this thread's turn has come. `false` when the
/// wait ended without what it waited for: this task is cancelled (then
/// it is on its way out), or the time `limit` passed (then the wait's
/// error is set).
pub(crate) fn resumed(waited: Result<(), Stop>, limit: Limit, line: i32) -> bool {
    crate::rt::kv_stack_limit.store(LIMIT.get(), std::sync::atomic::Ordering::Relaxed);
    match (waited, limit) {
        (Ok(()), _) => true,
        (Err(Stop::Cancelled), _) => {
            CANCELLING.set(true);
            false
        }
        (Err(Stop::Held), _) => panic_with(kavach_runtime_core::tasks::HELD, line),
        (Err(Stop::TimedOut), Some((_, ms))) => {
            let text = crate::heap::first_of(&Shape::Str);
            let message = kavach_runtime_core::tasks::timed_out(ms);
            crate::rt_errors::set_error(crate::shapes::new_text(&message, text));
            false
        }
        (Err(_), _) => panic_with("every task is waiting for another one (a deadlock)", line),
    }
}
