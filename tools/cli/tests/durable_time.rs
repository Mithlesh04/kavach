//! Durable agents and time (spec 8.5, decision 0062): a run after a
//! crash sees the time the first run saw, and its waits (`sleep`, `every`,
//! `.timeout`) end when they would have, not a whole wait later.

use std::process::Command;

fn kavach() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kavach"))
}

/// The command that runs `file`: `kavach run`, or the native program,
/// built first. Timing then counts only the program, not compiling it,
/// which a busy machine makes slow.
fn program(file: &std::path::Path, native: bool) -> Command {
    if !native {
        let mut cmd = kavach();
        cmd.arg("run").arg(file);
        return cmd;
    }
    // Built once: Windows may still hold a program that just ended.
    let exe = file.with_extension(std::env::consts::EXE_EXTENSION);
    if exe.exists() {
        return Command::new(exe);
    }
    let built = kavach()
        .arg("build")
        .arg(file)
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    Command::new(exe)
}

/// Runs `file` with `args`: what it printed, its exit code, and how long
/// the program ran.
fn run(
    file: &std::path::Path,
    native: bool,
    args: &[&str],
) -> (String, Option<i32>, std::time::Duration) {
    let mut cmd = program(file, native);
    let started = std::time::Instant::now();
    let out = cmd.args(args).output().unwrap();
    let printed =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (printed, out.status.code(), started.elapsed())
}

/// A durable handler's clock read and wait (spec 8.5, decision 0062): a
/// run after a crash sees the time the first run saw, and doesn't wait
/// again for what was waited already.
const TIMED: &str = "use std.cli
use std.time

type EmailError = NotSent | Denied(reason: String)

capability Email {
    fn send(to: String) ! EmailError
}

type Mail: Email {
    crash: Bool

    fn send(self, to: String) ! EmailError uses Console, Env {
        if self.crash {
            cli.exit(3)
        }
        print(\"[email] {to}\")
    }
}

agent Bot {
    can Email.send, Clock

    on remind() ! Error uses Clock {
        const t = time.now_ms()
        time.sleep(2.seconds)
        try Email.send(\"at {t}\")
    }
}

fn main() ! Error {
    const crash = cli.args().contains(\"crash\")
    const bot = Bot.start(Email: Mail(crash: crash))
    if crash {
        try bot.remind()
    }
}
";

#[test]
fn a_run_after_a_crash_sees_the_same_time_and_waits_no_more() {
    for (first, then) in [(false, false), (true, true), (false, true)] {
        let name = format!("timed-{first}-{then}");
        let dir =
            std::env::temp_dir().join(format!("kavach-durable-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"bot\"\n").unwrap();
        let file = dir.join("src/main.kav");
        std::fs::write(&file, TIMED).unwrap();
        let (said, code, _) = run(&file, first, &["crash"]);
        assert_eq!(code, Some(3), "{name}: {said}");
        let journal = std::fs::read_to_string(dir.join("agents/Bot/journal")).unwrap();
        let t = journal
            .lines()
            .find_map(|l| l.strip_prefix("{\"of\":1,\"call\":\"time.now_ms\",\"ok\":"))
            .and_then(|rest| rest.strip_suffix('}'))
            .unwrap_or_else(|| panic!("{name}: no clock read in {journal}"))
            .to_string();
        assert!(
            journal.contains("\"call\":\"time.sleep\""),
            "{name}: {journal}"
        );
        let (said, code, took) = run(&file, then, &[]);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            (said.as_str(), code),
            (format!("[email] at {t}\n").as_str(), Some(0)),
            "{name}"
        );
        assert!(
            took < std::time::Duration::from_millis(1_800),
            "{name}: waited again ({took:?})"
        );
    }
}

/// `every` in a durable handler (decision 0062): after a crash at round 2
/// the rounds already waited for pass at once, and only round 3 waits.
const TICKS: &str = "use std.cli
use std.time

type EmailError = NotSent | Denied(reason: String)

capability Email {
    fn send(to: String) ! EmailError
}

type Mail: Email {
    crash: Bool

    fn send(self, to: String) ! EmailError uses Console, Env {
        if self.crash and to == \"tick 2\" {
            cli.exit(3)
        }
        print(\"[email] {to}\")
    }
}

agent Bot {
    can Email.send, Clock

    on ticks() ! Error uses Clock {
        for tick in time.every(600.ms) {
            if tick == 3 {
                break
            }
            try Email.send(\"tick {tick}\")
        }
    }
}

fn main() ! Error {
    const crash = cli.args().contains(\"crash\")
    const bot = Bot.start(Email: Mail(crash: crash))
    if crash {
        try bot.ticks()
    }
}
";

#[test]
fn every_round_already_waited_for_passes_at_once() {
    for (first, then) in [(false, false), (true, true)] {
        let name = format!("ticks-{first}-{then}");
        let dir =
            std::env::temp_dir().join(format!("kavach-durable-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"bot\"\n").unwrap();
        let file = dir.join("src/main.kav");
        std::fs::write(&file, TICKS).unwrap();
        let (said, code, _) = run(&file, first, &["crash"]);
        assert_eq!(
            (said.as_str(), code),
            ("[email] tick 0\n[email] tick 1\n", Some(3)),
            "{name}"
        );
        let (said, code, took) = run(&file, then, &[]);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            (said.as_str(), code),
            ("[email] tick 2\n", Some(0)),
            "{name}"
        );
        assert!(
            took < std::time::Duration::from_millis(2_000),
            "{name}: waited again ({took:?})"
        );
    }
}

/// A `.timeout(d)` in a durable handler (decision 0062): killed half way
/// through the wait, the run after it times out when the first would
/// have, not a whole `d` later.
const TIMEOUT: &str = "use std.cli
use std.time

type EmailError = NotSent | Denied(reason: String)

capability Email {
    fn send(to: String) ! EmailError
}

type Mail: Email {
    fn send(self, to: String) ! EmailError uses Console {
        print(\"[email] {to}\")
    }
}

agent Bot {
    can Email.send, Clock

    on wait() ! Error uses Clock {
        const (_send, receive) = Channel<Int>(capacity: 1)
        const got = receive.take().timeout(3.seconds) catch _ => none
        try Email.send(\"got {got}\")
    }
}

fn main() ! Error {
    const bot = Bot.start(Email: Mail())
    if cli.args().contains(\"first\") {
        try bot.wait()
    }
}
";

#[test]
fn a_timeout_ends_when_it_would_have_after_a_restart() {
    for native in [false, true] {
        let name = format!("timeout-{native}");
        let dir =
            std::env::temp_dir().join(format!("kavach-durable-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"bot\"\n").unwrap();
        let file = dir.join("src/main.kav");
        std::fs::write(&file, TIMEOUT).unwrap();
        let journal = dir.join("agents/Bot/journal");
        // The first run is killed once its wait has begun.
        let mut first = program(&file, native)
            .arg("first")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let start = std::time::Instant::now();
        let begun = || std::fs::read_to_string(&journal).is_ok_and(|j| j.contains("time.timeout"));
        while !begun() && start.elapsed() < std::time::Duration::from_secs(60) {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        std::thread::sleep(std::time::Duration::from_millis(1_500));
        first.kill().unwrap();
        let _ = first.wait();
        let mut again = program(&file, native);
        let started = std::time::Instant::now();
        let out = again.output().unwrap();
        let took = started.elapsed();
        std::fs::remove_dir_all(&dir).unwrap();
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "[email] got none\n",
            "{name}: {said}"
        );
        assert!(
            took < std::time::Duration::from_millis(2_700),
            "{name}: waited again ({took:?})"
        );
    }
}
