// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `KAVACH_TIMES=1` (spec 10.11): each stage of compiling prints its
//! time to standard error, in order, for `kavach build` and `kavach run
//! --native`; without it, nothing.

use std::process::{Command, Stdio};

/// The stages `kavach <args> <file> <after>` printed times for, with
/// `times` set or not, and what the program printed.
fn stages(args: &[&str], after: &[&str], times: bool) -> (Vec<String>, String) {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "kavach-times-{}-{}",
        args[0],
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.kav");
    std::fs::write(&file, "fn main() {\n    print(6 * 7)\n}\n").unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    cmd.env_remove("KAVACH_TIMES").current_dir(&dir);
    if times {
        cmd.env("KAVACH_TIMES", "1");
    }
    let out = cmd
        .args(args)
        .arg(&file)
        .args(after)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    let names = stderr
        .lines()
        .filter_map(|l| l.strip_prefix("time: "))
        .map(|l| {
            let (name, ms) = l.rsplit_once(' ').and_then(|(rest, unit)| {
                assert_eq!(unit, "ms", "{l}");
                rest.rsplit_once(' ')
            })?;
            assert!(ms.parse::<f64>().is_ok(), "{l}");
            Some(name.to_string())
        })
        .collect::<Option<Vec<_>>>()
        .unwrap();
    (names, String::from_utf8_lossy(&out.stdout).into())
}

#[test]
fn each_stage_prints_its_time_in_order() {
    let front = [
        "parse", "resolve", "check", "rewrites", "lowering", "passes",
    ];
    let (built, _) = stages(&["build"], &["-o", "times-program"], true);
    let mut want: Vec<&str> = front.to_vec();
    want.extend(["codegen", "link"]);
    assert_eq!(built, want);
    let (ran, printed) = stages(&["run", "--native"], &[], true);
    assert_eq!(ran, [&front[..], &["codegen"]].concat());
    assert_eq!(printed, "42\n");
}

#[test]
fn without_the_setting_nothing_is_timed() {
    let (ran, printed) = stages(&["run", "--native"], &[], false);
    assert!(ran.is_empty(), "{ran:?}");
    assert_eq!(printed, "42\n");
}
