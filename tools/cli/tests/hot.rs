// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `kavach run --hot` (spec 10.5): a function changed while the program
//! runs is swapped in at its next call, and the program's values are
//! kept; a new type runs the program again from its start.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const PROGRAM: &str = "use std.time

fn update(n: Int) -> Int {
    n + 1
}

fn main() uses Clock {
    let n = 0
    for _ in 0..400 {
        n = update(n)
        print(n)
        time.sleep(50.ms)
    }
}
";

/// The program's lines as they come, and its messages once it ends.
fn start(dir: &std::path::Path) -> (std::process::Child, mpsc::Receiver<String>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["run", "--hot", "main.kav"])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let (tx, rx) = mpsc::channel();
    let out = child.stdout.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            let _ = tx.send(line);
        }
    });
    (child, rx)
}

/// The next line, waiting at most half a minute.
fn next(rx: &mpsc::Receiver<String>) -> String {
    rx.recv_timeout(Duration::from_secs(30)).expect("a line")
}

#[test]
fn a_changed_function_is_swapped_in_keeping_values() {
    let dir = std::env::temp_dir().join(format!("kavach-hot-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.kav"), PROGRAM).unwrap();
    let (mut child, rx) = start(&dir);
    assert_eq!(next(&rx), "1");
    std::fs::write(dir.join("main.kav"), PROGRAM.replace("n + 1", "n + 1000")).unwrap();
    // Counted by ones until the swap, then by thousands, from where it was.
    let mut last: i64 = 1;
    let swapped = loop {
        let n: i64 = next(&rx).parse().unwrap();
        if n - last == 1000 {
            break last;
        }
        assert_eq!(n, last + 1, "counted by ones before the swap");
        last = n;
    };
    assert!(swapped >= 1, "the values were kept: {swapped}");
    // A new type can't keep the values: it runs again from its start.
    let typed = format!("{}\ntype Extra {{\n    x: Int\n}}\n", PROGRAM);
    std::fs::write(dir.join("main.kav"), typed).unwrap();
    let again = loop {
        let line = next(&rx);
        if line == "1" {
            break true;
        }
    };
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(again, "it ran again from its start");
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("functions swapped"), "{said}");
    assert!(
        said.contains("the program's types changed: running again"),
        "{said}"
    );
}
