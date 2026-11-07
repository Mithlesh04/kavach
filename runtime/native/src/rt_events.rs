//! A send to an agent (`bot.request(order)`, spec 8.8, decision 0076):
//! the handler runs at once, in the sender's own task, and a panic in it
//! stops only the handler. The send fails with `panicked: …`, the panic
//! goes to the agent's audit log, and the agent goes on serving. The code
//! that calls this is made by `compiler/kir/src/lower_events.rs`.
//!
//! The handler runs through a thunk, as a task's body does: it hands its
//! value or error over with `TaskDone` or `TaskFail`, which go to the top
//! of `rt_tasks::EVENTS` while an event runs (and to the task otherwise).

use crate::heap::release;
use crate::rt_tasks::{EVENTS, MARKS};

/// `AgentRun`: runs closure `closure` (owned), the thunk of a send of
/// handler `name` (text, owned) to agent `agent`. Its value, or 0 with
/// its error left for `kv_take_error`.
pub(crate) fn run(closure: i64, agent: i64, name: i64, line: i32) -> i64 {
    let handler = crate::shapes::with_str(name, str::to_string);
    release(name);
    let marks = MARKS.with(|m| m.borrow().len());
    let running = running_depth();
    EVENTS.with(|e| e.borrow_mut().push(None));
    let thunk = crate::heap::slots(closure)[0] as usize;
    // SAFETY: slot 0 of a closure is its function's address, and the code
    // generator compiled an event's thunk as `fn(closure)`, which borrows
    // the closure; the code lives as long as the program.
    #[allow(unsafe_code)]
    let thunk: extern "C-unwind" fn(i64) = unsafe { std::mem::transmute(thunk) };
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| thunk(closure)));
    release(closure);
    let given = EVENTS.with(|e| e.borrow_mut().pop()).flatten();
    match (ran, given) {
        (Err(payload), _) => {
            // What the handler's unwound frames left: their marks of spawned
            // tasks, their agents and their `update`s.
            MARKS.with(|m| m.borrow_mut().truncate(marks));
            let panicked = &crate::rt_task_errors::PANICKED;
            panicked.store(true, std::sync::atomic::Ordering::Relaxed);
            pop_to(running);
            crate::send::let_go_after_panic();
            let why = crate::rt_task_errors::panic_text(payload);
            crate::rt_agents::note(agent, &format!("{handler} {why}"));
            crate::rt_replays::handler_panicked();
            let text = crate::heap::first_of(&kavach_kir::Shape::Str);
            crate::rt_errors::set_error(crate::shapes::new_text(&why, text));
            0
        }
        (Ok(()), Some((value, 0))) => value as i64,
        (Ok(()), Some((_, error))) => {
            crate::rt_errors::set_error(error);
            0
        }
        (Ok(()), None) => crate::rt::panic_with("the event gave no result", line),
    }
}

/// How many agents are running now (pushed by the events under way).
pub(crate) fn running_depth() -> usize {
    crate::rt_agents::RUNNING.with(|r| r.borrow().len())
}

/// Pops the agents past the first `depth`: a panic skipped their pops.
pub(crate) fn pop_to(depth: usize) {
    while let Some((p, ..)) = crate::rt_agents::RUNNING.with(|r| {
        let mut r = r.borrow_mut();
        (r.len() > depth).then(|| r.pop()).flatten()
    }) {
        crate::heap::release(p);
    }
}
