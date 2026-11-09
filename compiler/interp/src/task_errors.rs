//! An error nobody reads is not lost (spec 9.1, decision 0075): when the
//! last holder of a task that failed lets it go and no `.wait()` read its
//! error, the error is printed to standard error, with where the task was
//! spawned. The program goes on. An event run again after a restart was
//! spawned by no line of the program: its error says which event it is
//! (spec 8.6).

use crate::tasks::{Baton, TaskHandle};
use crate::value::Flow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// What a task's handle needs to tell of an error nobody read.
#[derive(Default)]
pub(crate) struct Unread {
    /// A `.wait()` got the result.
    pub(crate) read: AtomicBool,
    /// The task was cancelled: its "error" is no error (spec 9.6).
    pub(crate) cancelled: AtomicBool,
    /// Where it was spawned: `main.kav:6`.
    pub(crate) at: String,
    /// The event it runs again after a restart: `event #17 of request`.
    pub(crate) event: Option<String>,
    /// Where the program's standard error goes, and whether it is shown
    /// as it comes; none without tasks (then straight to the terminal).
    pub(crate) sink: Option<(Arc<Mutex<String>>, bool)>,
}

/// The line for an error nobody read of an event run again after a restart.
pub(crate) fn replay_error(event: &str, error: &str) -> String {
    format!("error in an event run again after a restart: {event}: {error}\n")
}

impl Drop for TaskHandle<'_> {
    fn drop(&mut self) {
        let u = &self.unread;
        if u.read.load(Ordering::Relaxed) || u.cancelled.load(Ordering::Relaxed) {
            return;
        }
        let result = self.result.lock().unwrap_or_else(|e| e.into_inner());
        let Some(Baton(Err(Flow::Fail(e)))) = result.as_ref() else {
            return;
        };
        let line = match &u.event {
            Some(event) => replay_error(event, &e.display()),
            None => format!(
                "error in a task nobody waited for: {} (spawned at {})\n",
                e.display(),
                u.at
            ),
        };
        match &u.sink {
            Some((err, false)) => err
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push_str(&line),
            _ => eprint!("{line}"),
        }
    }
}
