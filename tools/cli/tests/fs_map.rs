// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `fs.map` in native code (spec 14.3): the file is mapped, not read.
//! Replacing it while it is held works on every system, and the mapping
//! keeps the old contents; on Windows, emptying it is refused.

use std::path::{Path, PathBuf};
use std::process::Command;

const PROGRAM: &str = r#"use std.cli
use std.fs

fn look(path: String) ! FsError uses Fs, Console {
    const data = try fs.map(path)
    try fs.write_text(path, "new")
    print(data.text() ?? "?")
}

fn empty(path: String) ! FsError uses Fs, Console {
    const data = try fs.map(path)
    const _w = fs.create_writer(path) catch e => {
        match e {
            fs.Failed(_, _) => print("emptying refused")
            _ => print(e)
        }
        return
    }
    print(`emptied {data.len() > 0}`)
}

fn main() ! FsError uses Fs, Console {
    const path = cli.args()[0]
    try fs.write_text(path, "old")
    try look(path)
    print(try fs.read_text(path))
    if cli.args().len() > 1 {
        try empty(path)
    }
}
"#;

/// A folder of its own under Cargo's folder for test files, empty.
fn folder() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("kavach-fs-map-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn output(cmd: &mut Command) -> String {
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
}

#[test]
fn a_mapped_file_keeps_its_contents_when_the_file_is_replaced() {
    let dir = folder();
    let file = dir.join("map.kav");
    std::fs::write(&file, PROGRAM).unwrap();
    let data = dir.join("data.txt");
    let data = data.to_str().unwrap();
    let want = "old\nnew\n";
    let kavach = env!("CARGO_BIN_EXE_kavach");
    let native = output(
        Command::new(kavach)
            .args(["run", "--native"])
            .arg(&file)
            .arg(data),
    );
    assert_eq!(native, want, "kavach run --native");
    let exe = dir.join(format!("map{}", std::env::consts::EXE_SUFFIX));
    output(
        Command::new(kavach)
            .arg("build")
            .arg(&file)
            .arg("-o")
            .arg(&exe),
    );
    assert_eq!(output(Command::new(&exe).arg(data)), want, "kavach build");
    // Emptying a mapped file: Windows refuses it. Elsewhere, reading the
    // mapping past the new end would end the program, so it isn't tried.
    if cfg!(windows) {
        let out = output(Command::new(&exe).args([data, "empty"]));
        assert_eq!(out, "old\nnew\nemptying refused\n");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
