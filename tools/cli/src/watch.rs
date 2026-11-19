// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `kavach run --watch` (spec 10.5): runs the program, and each time a
//! `.kav` file or `kavach.toml` of the project changes, stops it and runs
//! it again. The files are looked at a few times a second, so no outside
//! library is needed to watch them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode};
use std::time::{Duration, SystemTime};

/// `kavach run --watch …`; `None` for any other command.
pub(crate) fn watch(args: &[String]) -> Option<ExitCode> {
    if args.first().map(String::as_str) != Some("run") {
        return None;
    }
    // `--watch` is Kavach's own only before the file (as `--native` is).
    let at = args[1..]
        .iter()
        .take_while(|a| a.starts_with("--") && *a != "--")
        .position(|a| a == "--watch")?
        + 1;
    let mut once = args.to_vec();
    once.remove(at);
    let me = std::env::current_exe().ok()?;
    let looked_at = folders(&once);
    let mut seen = times(&looked_at);
    let mut child = start(&me, &once);
    loop {
        std::thread::sleep(Duration::from_millis(300));
        let now = times(&looked_at);
        if now == seen {
            continue;
        }
        let changed: Vec<String> = now
            .iter()
            .filter(|(p, t)| seen.get(*p) != Some(t))
            .map(|(p, _)| p.to_string_lossy().replace('\\', "/"))
            .collect();
        seen = now;
        if let Some(c) = child.as_mut() {
            let _ = c.kill();
            let _ = c.wait();
        }
        eprintln!("-- changed: {}, running again", changed.join(", "));
        child = start(&me, &once);
    }
}

fn start(me: &Path, args: &[String]) -> Option<Child> {
    Command::new(me).args(args).spawn().ok()
}

/// What to watch: the folder of the file given, or the project's `src/`
/// and `kavach.toml`.
fn folders(args: &[String]) -> Vec<PathBuf> {
    if let Some(file) = args.iter().find(|a| a.ends_with(".kav")) {
        let folder = Path::new(file).parent().unwrap_or(Path::new("."));
        let folder = if folder.as_os_str().is_empty() {
            Path::new(".")
        } else {
            folder
        };
        return vec![folder.to_path_buf()];
    }
    match crate::project::root() {
        Some(root) => vec![root.join("src"), root.join("kavach.toml")],
        None => vec![PathBuf::from(".")],
    }
}

/// When each watched file last changed.
fn times(paths: &[PathBuf]) -> BTreeMap<PathBuf, SystemTime> {
    let mut out = BTreeMap::new();
    for p in paths {
        add(p, &mut out);
    }
    out
}

fn add(p: &Path, out: &mut BTreeMap<PathBuf, SystemTime>) {
    if p.is_dir() {
        let Ok(entries) = std::fs::read_dir(p) else {
            return;
        };
        for e in entries.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            let folder = path.is_dir() && !name.starts_with('.');
            if folder || name.ends_with(".kav") || name == "kavach.toml" {
                add(&path, out);
            }
        }
        return;
    }
    if let Ok(t) = std::fs::metadata(p).and_then(|m| m.modified()) {
        out.insert(p.to_path_buf(), t);
    }
}

/// `kavach run --hot file.kav [args…]` (spec 10.5): runs the program in
/// the interpreter, and each time a file changes, has it swap in its
/// changed functions; a change of more than function bodies runs it again
/// from its start.
pub(crate) fn hot(path: &str, rest: &[&str]) -> ExitCode {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    let rest = rest.strip_prefix(&["--"]).unwrap_or(rest);
    let args: Vec<String> = rest.iter().map(|a| a.to_string()).collect();
    let looked_at = folders(&[path.to_string()]);
    let files = Arc::new(Mutex::new(String::new()));
    loop {
        let program = match crate::load(path) {
            Ok(c) => &*Box::leak(Box::new(c)),
            Err(code) => return code,
        };
        let (changed, restart, done) = (
            Arc::<AtomicBool>::default(),
            Arc::<AtomicBool>::default(),
            Arc::<AtomicBool>::default(),
        );
        let watched = (
            changed.clone(),
            done.clone(),
            files.clone(),
            looked_at.clone(),
        );
        std::thread::spawn(move || {
            let (changed, done, files, looked_at): (
                Arc<AtomicBool>,
                Arc<AtomicBool>,
                Arc<Mutex<String>>,
                Vec<PathBuf>,
            ) = watched;
            let mut seen = times(&looked_at);
            while !done.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(300));
                let now = times(&looked_at);
                if now != seen {
                    let names: Vec<String> = (now.iter())
                        .filter(|(p, t)| seen.get(*p) != Some(t))
                        .map(|(p, _)| p.to_string_lossy().replace('\\', "/"))
                        .collect();
                    *files.lock().unwrap_or_else(|e| e.into_inner()) = names.join(", ");
                    seen = now;
                    changed.store(true, Ordering::Release);
                }
            }
        });
        let (source, named) = (path.to_string(), files.clone());
        let load = Box::new(move || {
            let names = named.lock().unwrap_or_else(|e| e.into_inner()).clone();
            match crate::load(&source) {
                Ok(c) => Some((&*Box::leak(Box::new(c)), names)),
                Err(_) => {
                    eprintln!(
                        "-- changed: {names}, which has errors: the program goes on as it was"
                    );
                    None
                }
            }
        });
        let reloader = kavach_interp::Reloader {
            changed,
            load,
            restart: restart.clone(),
        };
        let output = kavach_interp::run_hot(program, &args, reloader);
        done.store(true, Ordering::Release);
        if !restart.load(Ordering::Acquire) {
            return ExitCode::from(u8::try_from(output.exit_code).unwrap_or(1));
        }
        let names = files.lock().unwrap_or_else(|e| e.into_inner()).clone();
        eprintln!("-- changed: {names}, the program's types changed: running again");
    }
}
