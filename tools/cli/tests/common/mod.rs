// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What several test targets share: the interpreter's run of a test
//! program, and a watch that names a program still running after a
//! minute, so a run that hangs says where.

use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The programs running now, one place per worker: each its name and
/// when it started.
pub struct Running {
    now: Mutex<Vec<Option<(String, Instant)>>>,
    done: AtomicBool,
}

impl Running {
    pub fn new(workers: usize) -> Self {
        Running {
            now: Mutex::new(vec![None; workers]),
            done: AtomicBool::new(false),
        }
    }

    /// Worker `w` starts `name`, or (`None`) finished its program.
    pub fn set(&self, w: usize, name: Option<String>) {
        let mut now = self.now.lock().unwrap_or_else(|e| e.into_inner());
        now[w] = name.map(|n| (n, Instant::now()));
    }

    /// Ends `watch` when dropped, even when the test panics, so a scope
    /// waiting for the watch still ends.
    pub fn finishing(&self) -> Finishing<'_> {
        Finishing(self)
    }

    /// Until `finish`: each minute a program is still running, its name on
    /// the terminal, past the test's captured output.
    pub fn watch(&self) {
        let mut said: Vec<(String, u64)> = Vec::new();
        while !self.done.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(500));
            let now = self.now.lock().unwrap_or_else(|e| e.into_inner());
            for (name, since) in now.iter().flatten() {
                let minutes = since.elapsed().as_secs() / 60;
                if minutes > 0 && !said.contains(&(name.clone(), minutes)) {
                    said.push((name.clone(), minutes));
                    let _ = writeln!(
                        std::io::stderr(),
                        "still running after {minutes} min: {name}"
                    );
                }
            }
        }
    }
}

/// Ends a `Running`'s watch when dropped.
pub struct Finishing<'a>(&'a Running);

impl Drop for Finishing<'_> {
    fn drop(&mut self) {
        self.0.done.store(true, Ordering::Relaxed);
    }
}

/// The interpreter's run of the test program at `path`. A program that
/// starts itself again (`spawn process`, spec 9.8) or reads standard
/// input (`io.read`, or `std.term`'s questions) runs through `kavach
/// run`: inside this test, it would have only the test to start again,
/// and would read the test's own input, which may never end (a gate once
/// hung 20 minutes on `term.confirm`); a child's input is closed, so a
/// read gets `none`. So does a program that makes a GPU pipeline: only
/// `kavach run` gives the interpreter its shaders (spec 15.3).
pub fn interpret(
    compiled: &kavach_driver::Compiled,
    path: &std::path::Path,
    src: &str,
    args: &[String],
) -> kavach_interp::Output {
    let reads = src.contains("io.read") || src.contains("use std.term");
    if !src.contains("spawn process") && !reads && !src.contains("gpu.pipeline") {
        return kavach_interp::run_with_args(compiled, args);
    }
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("run")
        .arg(path)
        .args(args)
        .output()
        .unwrap();
    kavach_interp::Output {
        stdout: String::from_utf8_lossy(&out.stdout).into(),
        stderr: String::from_utf8_lossy(&out.stderr).into(),
        exit_code: out.status.code().unwrap_or(-1),
    }
}
