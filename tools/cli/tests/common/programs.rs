// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What the test targets that run many programs natively share
//! (`native.rs`, `passes_off.rs`): finding the programs, their
//! arguments, the first line of a panic, and running a check on each
//! program, several at once. A module of its own (`#[path]`), so the
//! targets that don't run programs this way don't see it unused.

use std::fs;
use std::path::{Path, PathBuf};

/// Every program under `dir`. A project folder (with `kavach.toml` and
/// `src/`) is one program: its `src/main.kav`, compiled with the other
/// files of `src/`.
pub fn kav_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if dir.join("kavach.toml").is_file() && dir.join("src").is_dir() {
        out.push(dir.join("src").join("main.kav"));
        return;
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            kav_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "kav") {
            out.push(path);
        }
    }
}

/// The first line of a panic or of `main`'s error, and the line number
/// it gives, without the path (which differs only in how it is written).
pub fn panic_line(stderr: &str) -> Option<(&str, Option<&str>)> {
    let mut lines = stderr.lines();
    let first = lines.find(|l| l.starts_with("panic:") || l.starts_with("error:"))?;
    let at = lines
        .next()
        .filter(|l| l.trim_start().starts_with("at "))
        .and_then(|l| l.rsplit(':').next());
    // Out of stack, the line is wherever the stack ran out: in a mutual
    // recursion, either function, as each mode's frames differ in size.
    let at = at.filter(|_| !first.contains("stack overflow"));
    Some((first, at))
}

/// The arguments a test's `// args:` header gives the program.
pub fn test_args(src: &str) -> Vec<String> {
    src.lines()
        .find_map(|l| l.strip_prefix("// args: "))
        .map_or(Vec::new(), |w| {
            w.split_whitespace().map(String::from).collect()
        })
}

/// How many programs are checked at once: at most 8, or
/// `KAVACH_TEST_WORKERS`. Each build runs its own linker, and 24 at once
/// took a shared 32 GB machine down to 0.5 GB free (2026-11-25).
pub fn workers() -> usize {
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::env::var("KAVACH_TEST_WORKERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(cores.min(8))
}

/// `check(n, path)` for each of `files`, several at once (most of the
/// time goes to waiting for `kavach` and the linker), in the order of
/// `files`. Each worker has a large stack, as the interpreter runs deep
/// recursion in it; a program still running after a minute is named.
pub fn each<T: Send>(files: &[PathBuf], check: impl Fn(usize, &Path) -> T + Sync) -> Vec<T> {
    let workers = workers();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::new());
    let running = crate::common::Running::new(workers);
    // The watch ends once every worker has, or one panicked.
    std::thread::scope(|outer| {
        outer.spawn(|| running.watch());
        let _finishing = running.finishing();
        std::thread::scope(|scope| {
            for w in 0..workers {
                let (next, results, running, check) = (&next, &results, &running, &check);
                std::thread::Builder::new()
                    .stack_size(256 * 1024 * 1024)
                    .spawn_scoped(scope, move || {
                        loop {
                            let n = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let Some(path) = files.get(n) else {
                                break;
                            };
                            running.set(w, Some(path.display().to_string()));
                            let checked = check(n, path);
                            running.set(w, None);
                            results.lock().unwrap().push((n, checked));
                        }
                    })
                    .unwrap();
            }
        });
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|(n, _)| *n);
    results.into_iter().map(|(_, r)| r).collect()
}
