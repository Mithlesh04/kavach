// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `fs.watch` (spec 14.3): a watched folder tells of a file written into
//! it, interpreted and in native code, and watching what isn't there
//! fails. Run in a folder of its own in the system's temporary folder.

use std::process::Command;

const WATCHED: &str = "use std.fs
use std.time

fn main() ! Error uses Fs, Clock {
    const w = try fs.watch(\"data\")
    try fs.write_text(\"data/level.txt\", \"1\")
    let seen: [FileChange] = []
    let rounds = 0
    while seen.is_empty() and rounds < 300 {
        seen = w.poll()
        time.sleep(10.ms)
        rounds += 1
    }
    print(seen.any(c => c.path == \"data/level.txt\"))
    const missing = fs.watch(\"nothing-here\") catch _ => {
        print(\"refused\")
        return
    }
    missing.close()
}
";

#[test]
fn a_watched_folder_tells_of_a_new_file() {
    for native in [false, true] {
        let dir =
            std::env::temp_dir().join(format!("kavach-fswatch-{native}-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("data")).unwrap();
        std::fs::write(dir.join("main.kav"), WATCHED).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd.arg("main.kav").current_dir(&dir).output().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let text = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            text,
            "true\nrefused\n",
            "native: {native}; {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
