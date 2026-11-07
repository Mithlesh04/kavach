//! `spawn process f(args)` (spec 9.8, decision 0075): the program starts
//! itself again, and the copy runs only that call. Both runtimes use this
//! file: the parent starts the child and reads what it prints, and the
//! child learns which call to run.
//!
//! The child is told which call by an environment variable, gets the
//! call's arguments as JSON on its standard input, and prints as any
//! program does. Its last line, after a marker, is the call's outcome as
//! JSON; how a value or an error is written there is each runtime's own.
//! The parent hands every other line to the program's output, in turn.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

/// The environment variable that tells a child which call to run.
const CALL: &str = "KAVACH_PROCESS_CALL";

/// What starts the child's last line, its outcome: a character no
/// program prints by chance.
const OUTCOME: &str = "\u{1}kavach-outcome ";

/// In a child: the number of the `spawn process` call it runs.
pub fn site() -> Option<i64> {
    std::env::var(CALL).ok()?.parse().ok()
}

/// In a child: the call's arguments, as JSON.
pub fn arguments() -> String {
    let mut text = String::new();
    let _ = std::io::stdin().read_to_string(&mut text);
    text
}

/// In a child: the line that gives the call's outcome (`json`) back.
pub fn outcome_line(json: &str) -> String {
    format!("{OUTCOME}{json}\n")
}

/// What the parent hears from a child.
pub enum Event {
    /// A line the child printed (without its line end).
    Line(String),
    /// The child ended: with its outcome, or without one (it panicked
    /// or exited), and its exit code.
    Ended {
        outcome: Option<String>,
        code: Option<i32>,
    },
}

/// Wakes the task waiting for the next event, from the reading thread.
pub type Waker = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct Heard {
    events: VecDeque<Event>,
    /// How many events came: a task notes it before it looks.
    count: u64,
    waiter: Option<Waker>,
}

/// A child running one call.
pub struct Running {
    child: Arc<Mutex<Child>>,
    heard: Arc<Mutex<Heard>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The command that started this program: how a built program, or the
/// command line that runs one, starts it again.
pub fn this_command() -> Vec<OsString> {
    let me = std::env::current_exe().map_or_else(|_| OsString::new(), |p| p.into_os_string());
    std::iter::once(me)
        .chain(std::env::args_os().skip(1))
        .collect()
}

/// Starts the program again with `command` (program and arguments), to
/// run call `site` with `arguments` (JSON).
pub fn start(command: &[OsString], site: i64, arguments: &str) -> std::io::Result<Running> {
    let Some((program, args)) = command.split_first() else {
        return Err(std::io::Error::other(
            "no command starts this program again",
        ));
    };
    let mut child = Command::new(program)
        .args(args)
        .env(CALL, site.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    if let Some(mut input) = child.stdin.take() {
        // A child that ended at once can't read them; it says so itself.
        let _ = input.write_all(arguments.as_bytes());
    }
    let output = child.stdout.take();
    let child = Arc::new(Mutex::new(child));
    let heard = Arc::new(Mutex::new(Heard::default()));
    let (theirs, told) = (Arc::clone(&child), Arc::clone(&heard));
    std::thread::spawn(move || {
        let mut outcome = None;
        if let Some(output) = output {
            let mut lines = BufReader::new(output);
            let mut line = Vec::new();
            while matches!(lines.read_until(b'\n', &mut line), Ok(n) if n > 0) {
                let text = String::from_utf8_lossy(&line);
                let text = text.trim_end_matches(['\n', '\r']);
                match text.strip_prefix(OUTCOME) {
                    Some(o) => outcome = Some(o.to_string()),
                    None => tell(&told, Event::Line(text.to_string())),
                }
                line.clear();
            }
        }
        // Its output closed, so it is ending; the lock is held a moment at
        // a time, so `kill` is never kept waiting.
        let code = loop {
            match lock(&theirs).try_wait() {
                Ok(Some(status)) => break status.code(),
                Ok(None) => {}
                Err(_) => break None,
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        };
        tell(&told, Event::Ended { outcome, code });
    });
    Ok(Running { child, heard })
}

fn tell(heard: &Mutex<Heard>, event: Event) {
    let waiter = {
        let mut h = lock(heard);
        h.events.push_back(event);
        h.count += 1;
        h.waiter.take()
    };
    if let Some(wake) = waiter {
        wake();
    }
}

impl Running {
    /// How many events came so far: noted before looking for one.
    pub fn count(&self) -> u64 {
        lock(&self.heard).count
    }

    /// The next event, if one came.
    pub fn next(&self) -> Option<Event> {
        lock(&self.heard).events.pop_front()
    }

    /// Calls `wake` at the next event after the count `seen`; `false`
    /// when one came already (then the caller looks again now).
    pub fn on_event(&self, seen: u64, wake: Waker) -> bool {
        let mut h = lock(&self.heard);
        if h.count != seen {
            return false;
        }
        h.waiter = Some(wake);
        true
    }

    /// Ends the child at once (`cancel()`, a time limit): it shares
    /// nothing, so nothing is left half-changed.
    pub fn kill(&self) {
        let _ = lock(&self.child).kill();
    }
}
