//! `std.process` (spec 14.23, decision 0105): other programs, started
//! with no shell, as trees that end together. The interpreter and native
//! code both call these, by the child's number, as `std.window` does.
//!
//! A child's standard output and standard error are each read by a
//! thread of their own into the child's inbox, line by line; a third
//! thread waits for the end once both are closed, and adds it last. How a
//! tree is held, ended and measured is each system's own (`windows.rs`,
//! `unix.rs`): the only `unsafe` code here.

use kavach_runtime_core::inbox::{Inbox, Waker};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as system;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as system;

/// How a program ended, and what it cost (spec 14.23's `Exit`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exit {
    pub code: i64,
    pub cpu_ms: i64,
    pub peak_bytes: i64,
}

/// What a child's task hears.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A line of standard output, without its line end.
    Out(String),
    /// A line of standard error.
    Err(String),
    /// The program ended; always the last event.
    Ended(Exit),
}

/// Why a program didn't start (spec 14.23's `ProcessError`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fail {
    NotFound(String),
    Failed(String),
}

/// A running program: its tree, its inbox, and its end once taken.
struct Child {
    tree: Arc<system::Tree>,
    inbox: Inbox<Event>,
    exit: Option<Exit>,
}

fn children() -> std::sync::MutexGuard<'static, HashMap<i64, Child>> {
    static CHILDREN: OnceLock<Mutex<HashMap<i64, Child>>> = OnceLock::new();
    CHILDREN
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Starts `program` with `args`; `setup` is the folder it runs in (`""`:
/// this program's own), then names and values to add to its environment,
/// in turn, as `process.start_id` gets them. Gives the child's number.
pub fn start(program: &str, args: &[String], setup: &[String]) -> Result<i64, Fail> {
    let folder = setup.first().map_or("", String::as_str);
    let env = setup.get(1..).unwrap_or_default().as_chunks::<2>().0;
    if !folder.is_empty() && !std::path::Path::new(folder).is_dir() {
        return Err(Fail::Failed(format!("there is no folder `{folder}`")));
    }
    let mut command = Command::new(program);
    command
        .args(args)
        .envs(env.iter().map(|[k, v]| (k, v)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !folder.is_empty() {
        command.current_dir(folder);
    }
    let (mut child, tree) = system::spawn(&mut command).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => Fail::NotFound(program.to_string()),
        _ => Fail::Failed(format!("`{program}` didn't start: {e}")),
    })?;
    let tree = Arc::new(tree);
    let inbox = Inbox::default();
    let readers = [
        child
            .stdout
            .take()
            .map(|o| read_lines(o, inbox.clone(), Event::Out)),
        child
            .stderr
            .take()
            .map(|e| read_lines(e, inbox.clone(), Event::Err)),
    ];
    let (waiting, told) = (Arc::clone(&tree), inbox.clone());
    std::thread::spawn(move || {
        for r in readers.into_iter().flatten() {
            let _ = r.join();
        }
        told.tell(Event::Ended(system::wait(child, &waiting)));
    });
    static NEXT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    children().insert(
        id,
        Child {
            tree,
            inbox,
            exit: None,
        },
    );
    Ok(id)
}

/// A thread that reads `from` line by line into `inbox`.
fn read_lines(
    from: impl Read + Send + 'static,
    inbox: Inbox<Event>,
    event: fn(String) -> Event,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut lines = BufReader::new(from);
        let mut line = Vec::new();
        while matches!(lines.read_until(b'\n', &mut line), Ok(n) if n > 0) {
            let text = String::from_utf8_lossy(&line);
            inbox.tell(event(text.trim_end_matches(['\n', '\r']).to_string()));
            line.clear();
        }
    })
}

fn inbox(id: i64) -> Option<Inbox<Event>> {
    children().get(&id).map(|c| c.inbox.clone())
}

/// How many events child `id` has had: noted before looking for one.
pub fn count(id: i64) -> u64 {
    inbox(id).map_or(0, |i| i.count())
}

/// Child `id`'s next event, if one came; `None` for a closed child too.
/// Once its end came, it is the end every time after.
pub fn next(id: i64) -> Option<Event> {
    let mut all = children();
    let child = all.get_mut(&id)?;
    match child.inbox.next() {
        Some(Event::Ended(exit)) => {
            child.exit = Some(exit);
            Some(Event::Ended(exit))
        }
        Some(e) => Some(e),
        None => child.exit.map(Event::Ended),
    }
}

/// Calls `wake` at child `id`'s next event after count `seen`; `false`
/// when one came already, or the child is closed (look again now).
pub fn on_event(id: i64, seen: u64, wake: Waker) -> bool {
    inbox(id).is_some_and(|i| i.on_event(seen, wake))
}

/// Whether child `id` is still open: `next` can still give something.
pub fn is_open(id: i64) -> bool {
    children().contains_key(&id)
}

/// Ends child `id`'s whole tree at once; its end still comes as an event.
pub fn kill(id: i64) {
    let tree = children().get(&id).map(|c| Arc::clone(&c.tree));
    if let Some(t) = tree {
        t.kill();
    }
}

/// Lets child `id` go: what is left of its tree is ended (spec 14.23).
pub fn close(id: i64) {
    let child = children().remove(&id);
    if let Some(c) = child {
        c.tree.kill();
    }
}

/// The most memory this program has held so far, in bytes.
pub fn peak_bytes() -> i64 {
    system::peak_self()
}

#[cfg(test)]
mod tests;
