//! Finishing before a stop (spec 5.8 and 14.11, decision 0088): a program
//! holding its stop finishes its work after `SIGTERM`, and a server
//! answers the request under way before the program ends, in the
//! interpreter, in native code and in a built program. Signals are sent
//! with `kill`, so these run on Unix.

#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// A folder of its own for test `name`, empty.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kavach-stopping-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The program at `file` started the three ways: interpreted, native and
/// built.
fn ways(file: &Path, dir: &Path) -> Vec<(&'static str, Command)> {
    let exe = dir.join("built");
    let built = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["build", file.to_str().unwrap(), "-o", exe.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let mut run = Command::new(env!("CARGO_BIN_EXE_kavach"));
    run.arg("run").arg(file);
    let mut native = Command::new(env!("CARGO_BIN_EXE_kavach"));
    native.args(["run", "--native"]).arg(file);
    vec![
        ("interpreted", run),
        ("native", native),
        ("built", Command::new(exe)),
    ]
}

/// Starts `cmd` and waits until it wrote `ready`: what it wrote there.
fn start(cmd: &mut Command, ready: &Path) -> (Child, String) {
    let _ = fs::remove_file(ready);
    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let begun = Instant::now();
    while begun.elapsed() < Duration::from_secs(60) {
        if let Ok(text) = fs::read_to_string(ready)
            && !text.is_empty()
        {
            return (child, text);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("the program never got ready");
}

fn term(child: &Child) {
    let sent = Command::new("kill")
        .args(["-s", "TERM", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(sent.success());
}

#[test]
fn a_hold_lets_the_program_finish_its_work() {
    let dir = folder("hold");
    let ready = dir.join("ready");
    let src = format!(
        "use std.cli\nuse std.fs\nuse std.time\n\nfn main() ! Error {{\n    const hold = cli.on_stop(grace: 20.seconds)\n    print(\"started\")\n    try fs.write_text(\"{}\", \"yes\")\n    while not hold.asked() {{\n        time.sleep(20.ms)\n    }}\n    print(\"saving\")\n    hold.done()\n}}\n",
        ready.display()
    );
    let file = dir.join("main.kav");
    fs::write(&file, src).unwrap();
    for (name, mut cmd) in ways(&file, &dir) {
        let (child, _) = start(&mut cmd, &ready);
        term(&child);
        let out = child.wait_with_output().unwrap();
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "started\nsaving\n",
            "{name}"
        );
        assert_eq!(out.status.code(), Some(143), "{name}");
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_server_answers_the_request_under_way_before_it_stops() {
    let dir = folder("drain");
    let ready = dir.join("ready");
    let src = format!(
        "use std.fs\nuse std.http\nuse std.net\nuse std.time\n\nfn main() ! Error {{\n    const listener = try net.listen(\"127.0.0.1:0\")\n    const slow = (r: Request) => {{\n        time.sleep(600.ms)\n        http.text(200, \"finished {{r.path}}\")\n    }}\n    try fs.write_text(\"{}\", listener.address())\n    try http.serve(listener, slow)\n}}\n",
        ready.display()
    );
    let file = dir.join("main.kav");
    fs::write(&file, src).unwrap();
    for (name, mut cmd) in ways(&file, &dir) {
        let (child, address) = start(&mut cmd, &ready);
        let mut conn = std::net::TcpStream::connect(address.trim()).unwrap();
        conn.write_all(b"GET /work HTTP/1.1\r\nhost: x\r\n\r\n")
            .unwrap();
        // The handler has begun; then the stop comes.
        std::thread::sleep(Duration::from_millis(200));
        term(&child);
        let mut answer = String::new();
        conn.set_read_timeout(Some(Duration::from_secs(20)))
            .unwrap();
        let _ = conn.read_to_string(&mut answer);
        assert!(answer.starts_with("HTTP/1.1 200"), "{name}: {answer}");
        assert!(answer.ends_with("finished /work"), "{name}: {answer}");
        let out = child.wait_with_output().unwrap();
        assert_eq!(out.status.code(), Some(143), "{name}");
    }
    let _ = fs::remove_dir_all(&dir);
}
