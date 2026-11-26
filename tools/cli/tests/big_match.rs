// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A `match` with many arms builds in time that grows with its arms, not
//! doubling with each one. A `match` is a chain of `if`s in KIR, and
//! passes that looked at a branch's whole depth again at each `if` took
//! minutes for 30 arms (a font table, 2026-11-27).

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Far more than a build takes here (2 s in a debug build), far less
/// than the minutes 30 arms took before.
const LIMIT: Duration = Duration::from_secs(60);

const ARMS: usize = 100;

/// Builds `src` within `LIMIT` and gives what the program prints.
fn build_and_run(name: &str, src: &str) -> String {
    let dir = std::env::temp_dir().join(format!("kavach-big-match-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join(format!("{name}.kav"));
    std::fs::write(&source, src).unwrap();
    let exe = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    let mut child = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&exe)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let err = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_dir_all(&dir);
            panic!("building `{name}` ({ARMS} arms) took more than {LIMIT:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let err = err.join().unwrap();
    assert!(status.success(), "{err}");
    let out = Command::new(&exe).stdin(Stdio::null()).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// The 7 rows of key `i`'s glyph.
fn rows(i: usize) -> Vec<String> {
    (0..7)
        .map(|j| format!("{:05b}", (i * 7 + j * 13) % 32))
        .collect()
}

/// A font table: one arm a key, each giving a list of text. A program
/// that uses the standard library marks its calls into it (spec 5.6),
/// the pass that met the blow-up first.
#[test]
fn a_match_of_a_hundred_lists_builds_quickly() {
    let mut src =
        String::from("use std.math\n\nfn glyph(c: String) -> [String] {\n    match c {\n");
    for i in 0..ARMS {
        let quoted: Vec<String> = rows(i).iter().map(|r| format!("\"{r}\"")).collect();
        src += &format!("        \"k{i}\" => [{}],\n", quoted.join(", "));
    }
    src += "        _ => [],\n    }\n}\n\nfn main() {\n";
    src += "    for k in [\"k0\", \"k50\", \"k99\", \"zz\"] {\n";
    src += "        const rows = glyph(k)\n";
    src += "        print(`{k}: {rows.len()} {rows.last() ?? \"-\"}`)\n    }\n";
    src += "    print(math.gcd(12, 18))\n}\n";
    let want: Vec<String> = [0, 50, 99]
        .iter()
        .map(|&i| format!("k{i}: 7 {}", rows(i)[6]))
        .chain(["zz: 0 -".to_string(), "6".to_string()])
        .collect();
    assert_eq!(
        build_and_run("glyphs", &src).lines().collect::<Vec<_>>(),
        want
    );
}

/// A `match` in a loop's body, which the loop passes and LLVM's code for
/// loops look through.
#[test]
fn a_match_of_a_hundred_arms_in_a_loop_builds_quickly() {
    let mut src = String::from(
        "fn score(n: Int) -> Int {\n    let sum = 0\n    for i in 0..n {\n        sum += match i {\n",
    );
    for i in 0..ARMS {
        src += &format!("            {i} => {},\n", i % 10);
    }
    src += "            _ => 1,\n        }\n    }\n    sum\n}\n\n";
    src += &format!("fn main() {{\n    print(score({}))\n}}\n", ARMS + 5);
    // 0 to 9 ten times, then 1 for each of the 5 past the arms.
    assert_eq!(build_and_run("scores", &src), "455\n");
}
