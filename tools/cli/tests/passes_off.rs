// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The differential test of the optimization passes: a program built
//! with the passes that only make it faster turned off
//! (`KAVACH_PASSES=none`, spec 10.11) must print the same, exit with the
//! same code and panic with the same message at the same line as built
//! with every pass. A difference is a pass that changed what a program
//! does. Out of stack, the line is ignored: inlining changes how big
//! each frame is, so the stack can run out in another function.
//!
//! It covers a selection, so it runs in minutes: the conformance tests
//! of control flow and of memory (spec 4 and 6, where the passes do the
//! most) and the loop benchmarks. Some speedups happen during lowering
//! and stay on (`compiler/kir/README.md`).

// For `Running`, which `programs::each` uses; not its interpreter run.
#[allow(dead_code)]
mod common;
#[path = "common/programs.rs"]
mod programs;

use kavach_driver::{Options, compile};
use programs::{kav_files, panic_line, test_args};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// What one program printed, for the message when they differ.
fn said(how: &str, out: &Output) -> String {
    format!(
        "{how}: exit {:?} stdout {:?} stderr {:?}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Builds the program at `path` with `passes` (`None`: unset) as `exe`
/// and runs it with `args`, its input closed; or why it didn't build.
fn build_and_run(
    path: &Path,
    passes: Option<&str>,
    exe: &Path,
    args: &[String],
) -> Result<Output, String> {
    let mut build = Command::new(env!("CARGO_BIN_EXE_kavach"));
    build
        .env_remove("KAVACH_PASSES")
        .env_remove("KAVACH_SROA_ONLY");
    if let Some(p) = passes {
        build.env("KAVACH_PASSES", p);
    }
    let built = build
        .arg("build")
        .arg(path)
        .arg("-o")
        .arg(exe)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    if !built.status.success() {
        return Err(String::from_utf8_lossy(&built.stderr).into());
    }
    Ok(Command::new(exe)
        .env("KAVACH_CHECK_LEAKS", "1")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap())
}

/// Program `n` at `path` built both ways and compared: `None` when it
/// doesn't build natively, else its name and how the two runs differ.
fn check(n: usize, path: &Path, root: &Path, out_dir: &Path) -> Option<(String, Vec<String>)> {
    let src = fs::read_to_string(path).unwrap();
    if src.contains("// expect compile error") || src.contains("// expect test") {
        return None;
    }
    let compiled = match kavach_driver::project_sources(path) {
        Some(sources) => kavach_driver::compile_sources(sources, Options::default()),
        None => compile(&src, &path.to_string_lossy(), Options::default()),
    };
    if compiled.has_errors() || kavach_kir::lower(&compiled).is_err() {
        return None;
    }
    let name = path
        .strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string();
    let args = test_args(&src);
    let exe = |how: &str| out_dir.join(format!("p{n}-{how}{}", std::env::consts::EXE_SUFFIX));
    let on = build_and_run(path, None, &exe("on"), &args);
    let off = build_and_run(path, Some("none"), &exe("off"), &args);
    let _ = fs::remove_file(exe("on"));
    let _ = fs::remove_file(exe("off"));
    let (on, off) = match (on, off) {
        (Ok(on), Ok(off)) => (on, off),
        (on, off) => {
            let why = [("passes on", on.err()), ("passes off", off.err())];
            let failed = why
                .iter()
                .filter_map(|(how, e)| Some(format!("{how}: {}", e.as_ref()?)));
            let failed: Vec<String> = failed.collect();
            return Some((
                name.clone(),
                vec![format!(
                    "{name}: kavach build failed: {}",
                    failed.join("; ")
                )],
            ));
        }
    };
    let mut found = Vec::new();
    let stderr = |o: &Output| String::from_utf8_lossy(&o.stderr).to_string();
    for (how, out) in [("passes on", &on), ("passes off", &off)] {
        if stderr(out).contains("were not freed") {
            found.push(format!("{name} ({how}): leaked memory: {}", stderr(out)));
        }
    }
    if on.stdout != off.stdout
        || on.status.code() != off.status.code()
        || panic_line(&stderr(&on)) != panic_line(&stderr(&off))
    {
        found.push(format!(
            "{name}\n  {}\n  {}",
            said("passes on ", &on),
            said("passes off", &off)
        ));
    }
    Some((name, found))
}

#[test]
fn programs_do_the_same_with_the_passes_off() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in [
        "tests/conformance/04-control-flow",
        "tests/conformance/06-memory",
        "benchmarks/loops/kavach",
    ] {
        kav_files(&root.join(dir), &mut files);
    }
    // Cargo's folder for a test's own files, on the build drive.
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("kavach-passes-off-{}", std::process::id()));
    // A working folder of its own, for the files programs write.
    let work = out_dir.join("work");
    fs::create_dir_all(&work).unwrap();
    let home = std::env::current_dir().unwrap();
    std::env::set_current_dir(&work).unwrap();
    let results = programs::each(&files, |n, path| check(n, path, &root, &out_dir));
    std::env::set_current_dir(home).unwrap();
    let _ = fs::remove_dir_all(&out_dir);
    let (mut built, mut mismatches) = (Vec::new(), Vec::new());
    for (name, found) in results.into_iter().flatten() {
        built.push(name);
        mismatches.extend(found);
    }
    println!(
        "{} programs built with the passes on and off:\n  {}",
        built.len(),
        built.join("\n  ")
    );
    assert!(
        mismatches.is_empty(),
        "programs differ with the passes off:\n{}",
        mismatches.join("\n")
    );
    // Guard against the selection silently shrinking.
    assert!(built.len() >= 120, "only {} programs built", built.len());
}
