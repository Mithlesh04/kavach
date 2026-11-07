//! Native code must behave exactly like the interpreter (decision 0014).
//! For every conformance test and example the native compiler supports,
//! `kavach run --native` and the executable from `kavach build` must print
//! the same output, exit with the same code, and panic with the same
//! message at the same line.

use kavach_driver::{Options, compile};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn kav_files(dir: &Path, out: &mut Vec<PathBuf>) {
    // A project folder (with `kavach.toml` and `src/`) is one test: its
    // `src/main.kav`, compiled with the other files of `src/`.
    if dir.join("kavach.toml").is_file() && dir.join("src").is_dir() {
        out.push(dir.join("src").join("main.kav"));
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            kav_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "kav") {
            out.push(path);
        }
    }
}

/// The first line of a panic or of `main`'s error, and the line number
/// it gives, without the path (which differs only in how it is written).
fn panic_line(stderr: &str) -> Option<(&str, Option<&str>)> {
    let mut lines = stderr.lines();
    let first = lines.find(|l| l.starts_with("panic:") || l.starts_with("error:"))?;
    let at = lines
        .next()
        .filter(|l| l.trim_start().starts_with("at "))
        .and_then(|l| l.rsplit(':').next());
    Some((first, at))
}

/// The arguments a test's `// args:` header gives the program.
fn test_args(src: &str) -> Vec<String> {
    src.lines()
        .find_map(|l| l.strip_prefix("// args: "))
        .map_or(Vec::new(), |w| {
            w.split_whitespace().map(String::from).collect()
        })
}

/// What checking one program found: `None` when the native compiler
/// doesn't support it, else its name and how its native runs differ
/// from the interpreter.
type Checked = Option<(String, Vec<String>)>;

/// Runs program `n` at `path` in the interpreter, with `run --native` and
/// as an executable from `kavach build`, and compares them.
fn check(n: usize, path: &Path, root: &Path, out_dir: &Path) -> Checked {
    let src = fs::read_to_string(path).unwrap();
    if src.contains("// expect compile error") || src.contains("// expect test") {
        return None;
    }
    let compiled = match kavach_driver::project_sources(path) {
        Some(sources) => kavach_driver::compile_sources(sources, Options::default()),
        None => compile(&src, &path.to_string_lossy(), Options::default()),
    };
    if compiled.has_errors() {
        return None;
    }
    let name = path
        .strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string();
    // Every program the interpreter runs must build natively too: a gap
    // fails here instead of being skipped unnoticed.
    if let Err(gap) = kavach_kir::lower(&compiled) {
        return Some((
            name.clone(),
            vec![format!(
                "{name}: native code doesn't support {} yet (line {})",
                gap.feature, gap.line
            )],
        ));
    }
    let mut mismatches = Vec::new();
    let args = test_args(&src);
    let expected = interpret(&compiled, path, &src, &args);
    let jit = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .env("KAVACH_CHECK_LEAKS", "1")
        .args(["run", "--native"])
        .arg(path)
        .args(&args)
        .output()
        .unwrap();
    let exe = out_dir.join(format!("p{n}{}", std::env::consts::EXE_SUFFIX));
    let built = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("build")
        .arg(path)
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    if !built.status.success() {
        mismatches.push(format!(
            "{name}: kavach build failed: {}",
            String::from_utf8_lossy(&built.stderr)
        ));
        return Some((name, mismatches));
    }
    let executable = Command::new(&exe)
        .env("KAVACH_CHECK_LEAKS", "1")
        .args(&args)
        .output()
        .unwrap();
    for (how, native) in [("run --native", jit), ("build", executable)] {
        let stdout = String::from_utf8_lossy(&native.stdout);
        let stderr = String::from_utf8_lossy(&native.stderr);
        let code = native.status.code().unwrap_or(-1);
        if stderr.contains("were not freed") {
            mismatches.push(format!(
                "{name} ({how}): reference counting leaked memory: {stderr}"
            ));
        }
        if stdout != expected.stdout
            || code != expected.exit_code
            || panic_line(&stderr) != panic_line(&expected.stderr)
        {
            mismatches.push(format!(
                "{name} ({how})\n  interpreter: exit {} stdout {:?} stderr {:?}\n  native:      exit {code} stdout {stdout:?} stderr {stderr:?}",
                expected.exit_code, expected.stdout, expected.stderr
            ));
        }
    }
    Some((name, mismatches))
}

#[test]
fn native_code_matches_the_interpreter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    kav_files(&root.join("tests/conformance"), &mut files);
    kav_files(&root.join("examples"), &mut files);

    let out_dir = std::env::temp_dir().join(format!("kavach-native-test-{}", std::process::id()));
    // A working folder of its own: the conformance suite runs the same
    // programs at the same time, and two runs of a program that writes
    // `kavach-test-bytes.bin` would remove each other's file.
    let work = out_dir.join("work");
    fs::create_dir_all(&work).unwrap();
    let home = std::env::current_dir().unwrap();
    std::env::set_current_dir(&work).unwrap();
    // The programs are checked on all cores at once: most of the time goes
    // to waiting for `kavach` and the linker. Each worker has a large
    // stack, as the interpreter runs deep recursion in it.
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            std::thread::Builder::new()
                .stack_size(256 * 1024 * 1024)
                .spawn_scoped(scope, || {
                    loop {
                        let n = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(path) = files.get(n) else {
                            break;
                        };
                        let checked = check(n, path, &root, &out_dir);
                        results.lock().unwrap().push((n, checked));
                    }
                })
                .unwrap();
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|(n, _)| *n);
    let (mut supported, mut mismatches) = (Vec::new(), Vec::new());
    for (name, found) in results.into_iter().filter_map(|(_, c)| c) {
        supported.push(name);
        mismatches.extend(found);
    }
    std::env::set_current_dir(home).unwrap();
    let _ = fs::remove_dir_all(&out_dir);
    let list = supported.join("\n  ");
    println!("{} programs compile natively:\n  {list}", supported.len());
    assert!(
        mismatches.is_empty(),
        "native code differs from the interpreter:\n{}",
        mismatches.join("\n")
    );
    // Guard against the native subset silently shrinking.
    assert!(
        supported.len() >= 85,
        "only {} programs compile natively: {supported:?}",
        supported.len()
    );
}

/// Big lists and maps stay fast in native code: reading an item or a key
/// used to copy the whole list or map, so this took half a minute.
#[test]
fn big_collections_are_not_copied() {
    let dir = std::env::temp_dir().join(format!("kavach-big-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("big.kav");
    let src = "let m: [Int: Int] = [:]\nlet items = []\nfor i in 0..200000 {\n    m[i] = i\n    items.push(i)\n}\nlet total = 0\nfor i in 0..200000 {\n    total += (m[i] ?? 0) + items[i]\n}\nprint(total)\n";
    fs::write(&path, src).unwrap();
    let start = std::time::Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["run", "--native"])
        .arg(&path)
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(String::from_utf8_lossy(&out.stdout), "39999800000\n");
    assert!(start.elapsed() < std::time::Duration::from_secs(10));
}

/// A list passed on at its last use is moved, not shared, so changing it
/// doesn't copy it (`compiler/kir/src/moves.rs`). Without moves this
/// copied the whole list every round and took over a minute. A list that
/// is still used afterwards keeps its value (spec 6.1).
#[test]
fn last_use_moves_instead_of_copying() {
    let dir = std::env::temp_dir().join(format!("kavach-moves-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("moves.kav");
    let src = "fn add_one(items: [Int]) -> [Int] {\n    let copy = items\n    copy.push(1)\n    copy\n}\n\nlet xs = []\nfor _ in 0..100000 {\n    xs = add_one(xs)\n}\nprint(xs.len())\nconst original = [7]\nconst bigger = add_one(original)\nprint(\"{original.len()} {bigger.len()}\")\n";
    fs::write(&path, src).unwrap();
    let start = std::time::Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .env("KAVACH_CHECK_LEAKS", "1")
        .args(["run", "--native"])
        .arg(&path)
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(String::from_utf8_lossy(&out.stdout), "100000\n1 2\n");
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(start.elapsed() < std::time::Duration::from_secs(10));
}

/// Ctrl+C and `SIGTERM` end a program as `cli.exit` does (spec 5.8,
/// decision 0062): what it printed comes out, and the exit code says
/// which signal it was. The program says it is running by writing a file.
#[cfg(unix)]
#[test]
fn a_stop_signal_ends_the_program_cleanly() {
    let dir = std::env::temp_dir().join(format!("kavach-stop-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let ready = dir.join("ready");
    let src = format!(
        "use std.fs\nuse std.time\n\nfn main() ! Error {{\n    print(\"started\")\n    try fs.write_text(\"{}\", \"yes\")\n    time.sleep(30.seconds)\n    print(\"never\")\n}}\n",
        ready.display()
    );
    let file = dir.join("main.kav");
    fs::write(&file, src).unwrap();
    let exe = dir.join("stop");
    let built = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["build", file.to_str().unwrap(), "-o", exe.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let kavach = || Command::new(env!("CARGO_BIN_EXE_kavach"));
    let ways: [(&str, Command, &str, i32); 3] = [
        (
            "interpreted",
            {
                let mut c = kavach();
                c.arg("run").arg(&file);
                c
            },
            "TERM",
            143,
        ),
        (
            "native",
            {
                let mut c = kavach();
                c.args(["run", "--native"]).arg(&file);
                c
            },
            "INT",
            130,
        ),
        ("built", Command::new(&exe), "TERM", 143),
    ];
    for (name, mut cmd, signal, code) in ways {
        let _ = fs::remove_file(&ready);
        let child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let start = std::time::Instant::now();
        while !ready.exists() && start.elapsed() < std::time::Duration::from_secs(60) {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let sent = Command::new("kill")
            .args(["-s", signal, &child.id().to_string()])
            .status()
            .unwrap();
        assert!(sent.success(), "{name}");
        let out = child.wait_with_output().unwrap();
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(code), "{name}: {said}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "started\n",
            "{name}: {said}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Running out of memory is a panic (spec 5.6): exit code 101 and a
/// message, not an abort. The memory is capped with `ulimit -v` (above
/// the main task's 512 MB stack), so the list's growth meets the limit.
#[cfg(target_os = "linux")]
#[test]
fn running_out_of_memory_is_a_panic() {
    let dir = std::env::temp_dir().join(format!("kavach-oom-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.kav");
    fs::write(
        &file,
        "fn main() {
    let xs: [Int] = []
    for i in 0..10_000_000_000 {
        xs.push(i)
    }
    print(xs.len())
}
",
    )
    .unwrap();
    let exe = dir.join("oom");
    let built = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["build", file.to_str().unwrap(), "-o", exe.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!("ulimit -v 1500000; exec {}", exe.display()))
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&dir);
    let said = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(101), "{said}");
    assert!(said.contains("panic: out of memory"), "{said}");
}

/// Serial ports (spec 14.18), on Linux, where a pseudo-terminal stands in
/// for a device: a Python helper holds the other end, answers `pong` to
/// `ping`, and the program prints what came back.
#[cfg(target_os = "linux")]
#[test]
fn a_serial_port_sends_and_receives() {
    let helper = r#"
import os, pty, select, time
master, slave = pty.openpty()
print(os.ttyname(slave), flush=True)
got = b''
end = time.time() + 30
while b'ping' not in got and time.time() < end:
    ready, _, _ = select.select([master], [], [], 0.1)
    if ready:
        got += os.read(master, 64)
os.write(master, b'pong')
time.sleep(2)
"#;
    let program = r#"use std.devices

fn main() ! Error uses Serial, Console {
    const port = try devices.open_serial("PATH", 9_600)
    try port.write("ping".bytes())
    let got = ""
    for _ in 0..5 {
        const part = try port.read(16)
        got = got + (part.text() ?? "?")
        if got.len() >= 4 {
            break
        }
    }
    port.close()
    print(got)
}
"#;
    for native in [false, true] {
        let mut side = Command::new("python3")
            .arg("-c")
            .arg(helper)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut path = String::new();
        let stdout = side.stdout.take().unwrap();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(stdout), &mut path).unwrap();
        let dir =
            std::env::temp_dir().join(format!("kavach-serial-{native}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("main.kav");
        fs::write(&file, program.replace("PATH", path.trim())).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd.arg(&file).output().unwrap();
        let _ = side.kill();
        let _ = side.wait();
        let _ = fs::remove_dir_all(&dir);
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "pong\n",
            "native {native}: {said}"
        );
    }
}

/// The interpreter's run of the test file at `path`. A program that starts
/// itself again (`spawn process`, spec 9.8) runs through `kavach run`:
/// run inside this test, it would have only the test to start again.
fn interpret(
    compiled: &kavach_driver::Compiled,
    path: &Path,
    src: &str,
    args: &[String],
) -> kavach_interp::Output {
    if !src.contains("spawn process") {
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
