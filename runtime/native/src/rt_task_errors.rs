//! What an ended task leaves for whoever waits for it, when it didn't end
//! with its value: the error of a task that was cancelled or panicked
//! (spec 9.6, 9.1). And an error nobody reads is not lost (decision 0075):
//! a task that failed and whose error no `.wait()` read prints it, with
//! where it was spawned, when it is freed, or when the program ends,
//! since objects still held then aren't freed one by one. An event run
//! again after a restart was spawned by no line of the program: its error
//! says which event it is (spec 8.6).

use crate::heap::Data;
use crate::rt_tasks::TaskObj;

/// Tasks that failed, for the errors nobody reads by the program's end.
static FAILED: std::sync::Mutex<Vec<std::sync::Weak<TaskObj>>> = std::sync::Mutex::new(Vec::new());

pub(crate) fn note_failed(task: i64) {
    static AT_EXIT: std::sync::Once = std::sync::Once::new();
    AT_EXIT.call_once(|| crate::rt::at_exit(unread_at_exit));
    if let Data::Task(t) = &*crate::heap::data(task).borrow() {
        let mut failed = FAILED.lock().unwrap_or_else(|e| e.into_inner());
        failed.retain(|w| w.strong_count() > 0);
        failed.push(std::sync::Arc::downgrade(t));
    }
}

/// The program ends: the objects still held aren't freed one by one, so
/// the errors nobody read are printed here.
fn unread_at_exit() {
    let failed = std::mem::take(&mut *FAILED.lock().unwrap_or_else(|e| e.into_inner()));
    for t in failed.iter().filter_map(std::sync::Weak::upgrade) {
        unread(&t);
    }
}

/// Prints task `t`'s error if it failed and nobody read it (spec 9.1,
/// decision 0075), once.
pub(crate) fn unread(t: &TaskObj) {
    let Some((_, error)) = t.result.get() else {
        return;
    };
    if error != 0 && !t.read.replace(true) && !t.quiet.get() {
        let shown = crate::display::show_error(error);
        if let Some(event) = &*t.event.borrow() {
            eprintln!("error in an event run again after a restart: {event}: {shown}");
            return;
        }
        let (path, line) = crate::rt::location(t.line);
        eprintln!("error in a task nobody waited for: {shown} (spawned at {path}:{line})");
    }
}

/// Whether a task panicked: the values its frames held were never freed
/// (decision 0065), so `KAVACH_CHECK_LEAKS` doesn't count them as leaks.
pub(crate) static PANICKED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Whether the running code is a spawned task's, where a panic stops only
/// that task; not the main task's.
pub(crate) fn in_spawned_task() -> bool {
    crate::rt_tasks::TASK.get() != 0 || crate::rt_tasks::EVENTS.with(|e| !e.borrow().is_empty())
}

/// What a task that panicked leaves for whoever waits for it: the error
/// `panicked: … at …`, already printed when it happened. The functions it
/// unwound out of left their marks of spawned tasks; the task's end joins
/// those tasks all the same.
pub(crate) fn panicked(task: i64, payload: Box<dyn std::any::Any + Send>) {
    PANICKED.store(true, std::sync::atomic::Ordering::Relaxed);
    let why = panic_text(payload);
    crate::rt_tasks::MARKS.with(|m| m.borrow_mut().clear());
    crate::send::let_go_after_panic();
    crate::rt_tasks::CANCELLING.set(false);
    let text = crate::heap::first_of(&kavach_kir::Shape::Str);
    let error = crate::shapes::new_text(&why, text);
    crate::rt_tasks::with_task(task, |t| t.quiet.set(true));
    let old = crate::rt_tasks::with_task(task, |t| t.result.replace(Some((0, error))));
    if let Some((_, e)) = old {
        crate::heap::release(e);
    }
}

/// What a cancelled task leaves for whoever waits for it: an error, the
/// time limit's when its time ran out (`within`). (Its body handed over
/// a placeholder, 0, which holds nothing.)
pub(crate) fn cancelled(task: i64) {
    let text = crate::heap::first_of(&kavach_kir::Shape::Str);
    let why = crate::rt_tasks::sched()
        .timed_out(crate::rt_tasks::task_id(task))
        .map_or_else(
            || "the task was cancelled".to_string(),
            kavach_runtime_core::tasks::timed_out,
        );
    let error = crate::shapes::new_text(&why, text);
    crate::rt_tasks::with_task(task, |t| t.quiet.set(true));
    let old = crate::rt_tasks::with_task(task, |t| t.result.replace(Some((0, error))));
    if let Some((_, e)) = old {
        crate::heap::release(e);
    }
}

/// The error text of a panic's `payload`: `panicked: … at …`.
pub(crate) fn panic_text(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast::<crate::rt::TaskPanic>() {
        Ok(p) => p.0,
        Err(other) => match other.downcast::<String>() {
            Ok(s) => format!("panicked: {s}"),
            Err(other) => match other.downcast::<&str>() {
                Ok(s) => format!("panicked: {s}"),
                Err(_) => "panicked".to_string(),
            },
        },
    }
}
