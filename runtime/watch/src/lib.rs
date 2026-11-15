//! Watching files and folders for `std.fs` (spec 14.3, decision 0116),
//! through `notify`, by the watcher's number, as the interpreter and
//! native code call it. Each watcher keeps the changes that came until
//! the program reads them: one by one with `next`, or all at once with
//! `poll`, each path once.

use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

#[cfg(test)]
mod tests;

/// The kinds of change, numbered as `std.fs`'s `ChangeKind` variants are.
pub const CREATED: u8 = 0;
pub const CHANGED: u8 = 1;
pub const REMOVED: u8 = 2;

struct Watching {
    _watcher: RecommendedWatcher,
    events: Receiver<notify::Result<Event>>,
    /// The path as the program wrote it, and as `notify` gives it.
    given: String,
    full: PathBuf,
    /// Changes that came and weren't read yet.
    pending: VecDeque<(u8, String)>,
}

fn all() -> MutexGuard<'static, HashMap<i64, Arc<Mutex<Watching>>>> {
    static ALL: OnceLock<Mutex<HashMap<i64, Arc<Mutex<Watching>>>>> = OnceLock::new();
    ALL.get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn next_number() -> i64 {
    static NEXT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Starts watching the file or folder at `path`: its number.
pub fn watch(path: &str) -> std::io::Result<i64> {
    let full = std::path::absolute(path)?;
    if !full.exists() {
        return Err(std::io::ErrorKind::NotFound.into());
    }
    let (tx, events) = channel();
    let mut watcher = notify::recommended_watcher(move |e| {
        let _ = tx.send(e);
    })
    .map_err(std::io::Error::other)?;
    watcher
        .watch(&full, RecursiveMode::Recursive)
        .map_err(std::io::Error::other)?;
    let n = next_number();
    let w = Watching {
        _watcher: watcher,
        events,
        given: path.to_string(),
        full,
        pending: VecDeque::new(),
    };
    all().insert(n, Arc::new(Mutex::new(w)));
    Ok(n)
}

fn get(n: i64) -> Option<Arc<Mutex<Watching>>> {
    all().get(&n).cloned()
}

/// The next change of watcher `n`, waiting for one; `None` once it is
/// closed.
pub fn next(n: i64) -> Option<(u8, String)> {
    let w = get(n)?;
    let mut w = w.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        if let Some(c) = w.pending.pop_front() {
            return Some(c);
        }
        let event = w.events.recv().ok()?;
        take(&mut w, event);
    }
}

/// The changes of watcher `n` since the last read, each path once, in
/// the order of its first change, with the kind of its last.
pub fn poll(n: i64) -> Vec<(u8, String)> {
    let Some(w) = get(n) else {
        return Vec::new();
    };
    let mut w = w.lock().unwrap_or_else(|e| e.into_inner());
    while let Ok(event) = w.events.try_recv() {
        take(&mut w, event);
    }
    let mut out: Vec<(u8, String)> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();
    for (kind, path) in w.pending.drain(..) {
        match at.get(&path) {
            Some(&i) => out[i].0 = kind,
            None => {
                at.insert(path.clone(), out.len());
                out.push((kind, path));
            }
        }
    }
    out
}

/// Stops watcher `n`.
pub fn close(n: i64) {
    all().remove(&n);
}

/// Adds `event`'s changes to `w`'s.
fn take(w: &mut Watching, event: notify::Result<Event>) {
    let Ok(event) = event else {
        return;
    };
    for (kind, path) in changes(&event) {
        let shown = shown(&w.given, &w.full, &path);
        w.pending.push_back((kind, shown));
    }
}

/// The changes an event says, by path: a rename is the old path removed
/// and the new one created. Reading a file is no change.
pub(crate) fn changes(event: &Event) -> Vec<(u8, PathBuf)> {
    let each = |kind: u8| event.paths.iter().map(|p| (kind, p.clone())).collect();
    match &event.kind {
        EventKind::Create(_) => each(CREATED),
        EventKind::Remove(_) => each(REMOVED),
        EventKind::Modify(ModifyKind::Name(mode)) => match mode {
            RenameMode::From => each(REMOVED),
            RenameMode::To => each(CREATED),
            RenameMode::Both => {
                let mut out = Vec::new();
                if let Some(from) = event.paths.first() {
                    out.push((REMOVED, from.clone()));
                }
                if let Some(to) = event.paths.get(1) {
                    out.push((CREATED, to.clone()));
                }
                out
            }
            // The system didn't say which side: whether it is there now.
            _ => (event.paths.iter())
                .map(|p| (if p.exists() { CREATED } else { REMOVED }, p.clone()))
                .collect(),
        },
        EventKind::Modify(_) => each(CHANGED),
        EventKind::Access(_) => Vec::new(),
        EventKind::Any | EventKind::Other => each(CHANGED),
    }
}

/// `path`, under the watched path as the program wrote it.
pub(crate) fn shown(given: &str, full: &Path, path: &Path) -> String {
    match path.strip_prefix(full) {
        Ok(rest) if rest.as_os_str().is_empty() => given.to_string(),
        Ok(rest) => {
            let rest = rest.to_string_lossy().replace('\\', "/");
            format!("{}/{rest}", given.trim_end_matches(['/', '\\']))
        }
        Err(_) => path.to_string_lossy().replace('\\', "/"),
    }
}
