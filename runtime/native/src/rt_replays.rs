//! Events run again after a restart, in native code (spec 8.3, decision
//! 0072), as the interpreter runs them: each in a task, which `start`
//! waits for until it ends or waits for a person. One that waits goes
//! on as a task of `start`'s caller; a send of the same event joins it.
//! Every one's outcome goes to the audit log. The code that calls these
//! ops is made by `compiler/kir/src/lower_replay.rs`.

use crate::heap::{obj, release, retain, slots};
use crate::rt_tasks::{ME, sched};
use kavach_kir::{RtOp, Shape};
use kavach_runtime_core::json::Tree;
use kavach_runtime_core::tasks::TaskId;
use std::sync::Mutex;

/// A replayed event's task: its agent (the guard's handle), the event's
/// number, handler and arguments, and the task object, kept while it
/// waits for a person.
struct Replay {
    agent: i64,
    task: TaskId,
    object: i64,
    event: i64,
    handler: String,
    args: Vec<Tree>,
}

/// Replayed events: the ones being started, then those left waiting.
static REPLAYS: Mutex<Vec<Replay>> = Mutex::new(Vec::new());
/// The tasks waiting for a person's answer now (`rt_agents.rs`).
static ASKING: Mutex<Vec<TaskId>> = Mutex::new(Vec::new());

thread_local! {
    /// The task a send joins, between `AgentJoining` and `AgentJoined`.
    static JOINED: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
}

/// This task starts (`true`) or stops waiting for a person's answer.
pub(crate) fn asking(on: bool) {
    let me = ME.get();
    let mut all = ASKING.lock().unwrap_or_else(|e| e.into_inner());
    all.retain(|t| *t != me);
    if on {
        all.push(me);
    }
}

fn replays() -> std::sync::MutexGuard<'static, Vec<Replay>> {
    REPLAYS.lock().unwrap_or_else(|e| e.into_inner())
}

/// A handler panicked in this task. A replayed event has no sender: the
/// panic was printed and written to the audit log, and its error needs
/// printing no second time.
pub(crate) fn handler_panicked() {
    let me = ME.get();
    if replays().iter().any(|r| r.task == me) {
        crate::rt_tasks::with_task(crate::rt_tasks::TASK.get(), |t| t.quiet.set(true));
    }
}

/// The items of tuple `t` (owned) as JSON, as the journal keeps them.
fn trees(t: i64) -> Vec<Tree> {
    let types = match crate::shapes::shape(obj(t).ty()) {
        Shape::Tuple(ts) => ts.clone(),
        _ => Vec::new(),
    };
    let out = (types.iter().zip(slots(t)))
        .map(|(ty, s)| crate::rt_json::encode(*s as i64, *ty))
        .collect();
    release(t);
    out
}

/// The ops (operands owned). `code` is the result type.
pub(crate) fn run(op: RtOp, a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    match op {
        // The event's task, from closure `a`, of agent `b`.
        RtOp::AgentReplay => {
            let (event, handler, args) = crate::rt_journal::replaying(b).unwrap_or_default();
            let task = crate::rt_tasks::spawn_task(a, code, line);
            let named = format!("event #{event} of {handler}");
            crate::rt_tasks::with_task(task, |t| *t.event.borrow_mut() = Some(named));
            replays().push(Replay {
                agent: b,
                task: crate::rt_tasks::task_id(task),
                object: 0,
                event,
                handler,
                args,
            });
            task
        }
        // `start` waits for task `b` until it ends or asks a person.
        RtOp::AgentReplayWait => {
            let id = crate::rt_tasks::task_id(b);
            loop {
                if sched().is_done(id) || crate::rt_tasks::CANCELLING.get() {
                    replays().retain(|r| r.task != id);
                    release(b);
                    return 0;
                }
                if ASKING
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .contains(&id)
                    && let Some(r) = replays().iter_mut().find(|r| r.task == id)
                {
                    r.object = b;
                    return 0;
                }
                let soon = std::time::Instant::now() + std::time::Duration::from_millis(10);
                crate::rt_waits::sleep_until(soon, line);
            }
        }
        // The event ended with value (`b` is 1) or error (2) in tuple `c`.
        RtOp::AgentReplayEnd => {
            let me = ME.get();
            let left = {
                let mut all = replays();
                let at = all.iter().position(|r| r.task == me);
                at.map(|i| all.remove(i))
            };
            let shown = left.as_ref().map(|_| {
                let ty = match crate::shapes::shape(obj(c).ty()) {
                    Shape::Tuple(ts) => ts.first().copied().unwrap_or(kavach_kir::Type::Unit),
                    _ => kavach_kir::Type::Unit,
                };
                match b {
                    1 => crate::display::display(slots(c)[0] as i64, ty, false),
                    _ => crate::display::show_error(slots(c)[0] as i64),
                }
            });
            release(c);
            if let (Some(r), Some(shown)) = (left, shown) {
                let outcome = if b == 1 { "ok" } else { "error" };
                let text = format!(
                    "finished after a restart: event #{} of {}: {outcome} {shown}",
                    r.event, r.handler
                );
                crate::rt_agents::note(a, &text);
                if r.object != 0 {
                    release(r.object);
                }
            }
            0
        }
        // A send of handler `b` with the arguments in tuple `c`: whether a
        // replayed event of it still waits for a person.
        RtOp::AgentJoining => {
            let handler = crate::shapes::with_str(b, str::to_string);
            release(b);
            let args = trees(c);
            let all = replays();
            // A replayed event runs for itself, even when an equal one waits.
            let me = ME.get();
            let found = (!all.iter().any(|r| r.task == me))
                .then(|| {
                    all.iter().find(|r| {
                        r.agent == a && r.object != 0 && r.handler == handler && r.args == args
                    })
                })
                .flatten()
                .map(|r| r.object);
            drop(all);
            if let Some(task) = found {
                retain(task);
                JOINED.set(task);
            }
            i64::from(found.is_some())
        }
        _ => JOINED.replace(0),
    }
}
