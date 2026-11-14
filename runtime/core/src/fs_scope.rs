//! Folders that file access is kept inside (`uses Fs("./data")`, spec
//! 8.1), and programs that may be run (`uses Process("git")`, 14.23): one
//! check for the interpreter and native code. Each call of a function so
//! narrowed adds its [`Scope`] while it runs; a task keeps its spawner's.
//!
//! A path is turned into the absolute path it really names: the part that
//! exists is resolved by the operating system, so `..` and symbolic links
//! are followed, and the part that doesn't exist yet (a file about to be
//! written) is added after it. A path is inside a folder when the folder,
//! resolved the same way, starts it.

use crate::seeded::Seeded;
use crate::trace::{Finished, SpanId};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The absolute path `path` names, `..` and symbolic links resolved as far
/// as the path exists.
pub fn resolve(path: &str) -> PathBuf {
    let p = Path::new(path);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    };
    let parts: Vec<Component> = abs.components().collect();
    for n in (1..=parts.len()).rev() {
        let existing: PathBuf = parts[..n].iter().collect();
        if let Ok(mut out) = existing.canonicalize() {
            for c in &parts[n..] {
                match c {
                    Component::ParentDir => {
                        out.pop();
                    }
                    Component::Normal(name) => out.push(name),
                    _ => {}
                }
            }
            return out;
        }
    }
    abs
}

/// What one narrowed `uses` allows while its function runs, or what a
/// block given to a call set up (spec 4.8).
#[derive(Clone, Debug)]
pub enum Scope {
    /// `Fs("./data")`, `resolve`d.
    Folder(PathBuf),
    /// `Process("git")`, as written.
    Program(String),
    /// `log.span(name) { … }` (14.12): the span, its parent, its name and
    /// when it started.
    Span {
        id: SpanId,
        parent: Option<u64>,
        name: String,
        start_nanos: u128,
    },
    /// `random.seeded(seed) { … }` (14.15): the task's generator there.
    Seeded(Arc<Mutex<Seeded>>),
}

/// The span the code runs in, if any.
pub fn current_span(scopes: &[Scope]) -> Option<SpanId> {
    scopes.iter().rev().find_map(|s| match s {
        Scope::Span { id, .. } => Some(*id),
        _ => None,
    })
}

/// A new span named `name`: the child of the current one, or of `remote`
/// (a `traceparent` a request brought) when there is none.
pub fn span(scopes: &[Scope], name: &str, remote: Option<SpanId>) -> Scope {
    let parent = current_span(scopes).or(remote);
    Scope::Span {
        id: crate::trace::child_of(parent),
        parent: parent.map(|p| p.span),
        name: name.to_string(),
        start_nanos: crate::trace::now_nanos(),
    }
}

/// What a task spawned here keeps: all but the seeds, which belong to the
/// task that runs their block.
pub fn inherited(scopes: &[Scope]) -> Vec<Scope> {
    (scopes.iter())
        .filter(|s| !matches!(s, Scope::Seeded(_)))
        .cloned()
        .collect()
}

/// 64 bits from the innermost `random.seeded` block, if the code is in one.
pub fn seeded_bits(scopes: &[Scope]) -> Option<u64> {
    scopes.iter().rev().find_map(|s| match s {
        Scope::Seeded(g) => Some(g.lock().unwrap_or_else(|e| e.into_inner()).next_u64()),
        _ => None,
    })
}

/// Ends the scopes above `depth`: gives the spans among them, ended now,
/// marked with `error` when their block failed.
pub fn leave(scopes: &mut Vec<Scope>, depth: usize, error: Option<&str>) -> Vec<Finished> {
    let end_nanos = crate::trace::now_nanos();
    let gone = scopes.split_off(depth.min(scopes.len()));
    gone.into_iter()
        .rev()
        .filter_map(|s| match s {
            Scope::Span {
                id,
                parent,
                name,
                start_nanos,
            } => Some(Finished {
                name,
                id,
                parent,
                start_nanos,
                end_nanos,
                error: error.map(str::to_string),
            }),
            _ => None,
        })
        .collect()
}

/// Whether `path` is inside every folder of `scopes`.
pub fn inside(scopes: &[Scope], path: &str) -> bool {
    let mut folders = scopes.iter().filter_map(|s| match s {
        Scope::Folder(f) => Some(f),
        _ => None,
    });
    let Some(first) = folders.next() else {
        return true;
    };
    let p = resolve(path);
    p.starts_with(first) && folders.all(|f| p.starts_with(f))
}

/// Whether `program` is each program of `scopes`: a name matches only the
/// same name, a path only the same path (spec 14.23).
pub fn may_run(scopes: &[Scope], program: &str) -> bool {
    scopes.iter().all(|s| match s {
        Scope::Program(p) => p == program,
        _ => true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_stay_inside_their_folder() {
        let base = std::env::temp_dir().join(format!("kavach-scope-{}", std::process::id()));
        let data = base.join("data");
        std::fs::create_dir_all(data.join("inner")).unwrap();
        let scopes = [Scope::Folder(resolve(data.to_str().unwrap()))];
        let at = |rest: &str| format!("{}/{rest}", data.display());
        assert!(inside(&scopes, &at("a.txt")));
        assert!(inside(&scopes, &at("inner/../b.txt")));
        assert!(inside(&scopes, &at("new/folder/c.txt")));
        assert!(!inside(&scopes, &at("../secrets.txt")));
        assert!(!inside(&scopes, &at("inner/../../x")));
        assert!(!inside(&scopes, &at("new/../../y")));
        // Folders nest: a path must be in all of them.
        let both = [scopes[0].clone(), Scope::Folder(resolve(&at("inner")))];
        assert!(inside(&both, &at("inner/d.txt")));
        assert!(!inside(&both, &at("e.txt")));
        // A program's name narrows nothing about files.
        let named = [scopes[0].clone(), Scope::Program("git".into())];
        assert!(inside(&named, &at("a.txt")));
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn programs_nest_and_each_must_match() {
        let named = |n: &[&str]| {
            n.iter()
                .map(|s| Scope::Program(s.to_string()))
                .collect::<Vec<_>>()
        };
        assert!(may_run(&[], "git"));
        assert!(may_run(&named(&["git"]), "git"));
        assert!(!may_run(&named(&["git"]), "/usr/bin/git"));
        assert!(!may_run(&named(&["git", "make"]), "git"));
        assert!(may_run(&[Scope::Folder(PathBuf::from("/data"))], "git"));
    }
}
