//! `kavach run --watch` (spec 10.5): the program runs, and runs again
//! when its file changes.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn a_changed_file_runs_again() {
    let dir = std::env::temp_dir().join(format!("kavach-watch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("app.kav");
    std::fs::write(&file, "print(\"one\")\n").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["run", "--watch", file.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let out = child.stdout.take().unwrap();
    let (send, lines) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            let _ = send.send(line);
        }
    });
    let wait = Duration::from_secs(60);
    assert_eq!(lines.recv_timeout(wait).as_deref(), Ok("one"));
    // Some file systems keep times to the second: a change a moment later
    // is seen for sure.
    std::thread::sleep(Duration::from_millis(1100));
    std::fs::write(&file, "print(\"two\")\n").unwrap();
    assert_eq!(lines.recv_timeout(wait).as_deref(), Ok("two"));
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}
