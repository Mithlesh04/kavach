//! Durable agents across bad restarts (spec 8.5, decision 0073): an event
//! that keeps stopping the program is set aside, and a second program
//! can't start an agent another program runs.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

const PROGRAM: &str = "use std.cli
use std.time

type PayError = Declined | Denied(reason: String)

capability Payments {
    fn pay(order_id: Int) -> Int ! PayError
}

type Till: Payments {
    crash: Bool
    decline: Bool

    fn pay(self, order_id: Int) -> Int ! PayError uses Console, Env {
        if self.crash {
            cli.exit(3)            // the program stops in the middle of the event
        }
        if self.decline {
            fail PayError.Declined
        }
        order_id
    }
}

agent Bot {
    settings AgentSettings(audit: Audit())

    can Payments.pay

    on request(order_id: Int) -> Int ! Error {
        try Payments.pay(order_id)
    }
}

fn main() ! Error uses Clock {
    const args = cli.args()
    const till = Till(crash: args.contains(\"crash\"), decline: args.contains(\"decline\"))
    const bot = Bot.start(Payments: till)
    if args.contains(\"crash\") {
        const _paid = try bot.request(7)
    }
    print(\"started\")
    if args.contains(\"hold\") {
        time.sleep(10.seconds)     // keeps the agent's folder locked
    }
}
";

const PANICS: &str = "use std.cli

agent Picker {
    settings AgentSettings(audit: Audit())

    can Console

    on pick(items: [Int], at: Int) -> Int {
        items[at]
    }
}

fn main() ! Error {
    const bot = Picker.start()
    if cli.args().contains(\"send\") {
        const got = bot.pick([1, 2], 5) catch e => {
            print(\"failed: {e}\")
            return
        }
        print(got)
    }
    print(\"started\")
}
";

#[derive(Clone, Copy, Debug)]
enum Way {
    Interpreted,
    Native,
}

/// A project holding `PROGRAM` in a folder of its own.
fn project(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kavach-restarts-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"shop\"\n").unwrap();
    std::fs::write(dir.join("src/main.kav"), PROGRAM).unwrap();
    dir
}

fn command(dir: &std::path::Path, way: Way, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    match way {
        Way::Interpreted => cmd.arg("run"),
        Way::Native => cmd.args(["run", "--native"]),
    };
    cmd.arg(dir.join("src/main.kav")).args(args);
    cmd
}

/// Standard output and error together, and the exit code.
fn run(dir: &std::path::Path, way: Way, args: &[&str]) -> (String, Option<i32>) {
    let out = command(dir, way, args).output().unwrap();
    let said =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (said, out.status.code())
}

/// A handler that panics again when it runs after a restart: the audit
/// log gets the line each time, and nothing says nobody waited (spec 8.6).
fn panics_again(way: Way) {
    let dir = project(&format!("panics-{way:?}"));
    std::fs::write(dir.join("src/main.kav"), PANICS).unwrap();
    let (said, code) = run(&dir, way, &["send"]);
    assert_eq!(code, Some(0), "{way:?}: {said}");
    assert!(
        said.contains("failed: panicked: index 5"),
        "{way:?}: {said}"
    );
    let (said, code) = run(&dir, way, &[]);
    assert_eq!(code, Some(0), "{way:?}: {said}");
    assert!(said.contains("started"), "{way:?}: {said}");
    assert!(said.contains("panic: index 5"), "{way:?}: {said}");
    assert!(!said.contains("nobody waited"), "{way:?}: {said}");
    let audit = std::fs::read_to_string(dir.join("agents/Picker/audit.log")).unwrap();
    assert_eq!(
        audit.matches("Picker pick panicked: index 5").count(),
        2,
        "{way:?}: {audit}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_replayed_panic_goes_to_the_audit_log() {
    panics_again(Way::Interpreted);
}

#[test]
fn native_code_logs_a_replayed_panic_too() {
    panics_again(Way::Native);
}

/// An event that runs again has no sender: however it ends, the audit log
/// gets its outcome, and an error nothing read says it came from a restart
/// (spec 8.6).
fn outcomes_after_a_restart(way: Way) {
    let dir = project(&format!("outcomes-{way:?}"));
    assert_eq!(run(&dir, way, &["crash"]).1, Some(3), "{way:?}");
    let (said, code) = run(&dir, way, &["decline"]);
    assert_eq!(code, Some(0), "{way:?}: {said}");
    assert!(
        said.contains("error in an event run again after a restart: event #1 of request: Declined"),
        "{way:?}: {said}"
    );
    assert!(!said.contains("nobody waited"), "{way:?}: {said}");
    let audit = dir.join("agents/Bot/audit.log");
    let log = std::fs::read_to_string(&audit).unwrap();
    assert!(
        log.contains("finished after a restart: event #1 of request: error Declined"),
        "{way:?}: {log}"
    );
    assert_eq!(run(&dir, way, &["crash"]).1, Some(3), "{way:?}");
    assert_eq!(
        run(&dir, way, &[]),
        ("started\n".to_string(), Some(0)),
        "{way:?}"
    );
    let log = std::fs::read_to_string(&audit).unwrap();
    assert!(
        log.contains("finished after a restart: event #2 of request: ok 7"),
        "{way:?}: {log}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_event_run_again_writes_its_outcome() {
    outcomes_after_a_restart(Way::Interpreted);
}

#[test]
fn native_code_writes_their_outcomes_too() {
    outcomes_after_a_restart(Way::Native);
}

fn set_aside(way: Way) {
    let dir = project(&format!("aside-{way:?}"));
    // Three starts of the event, each stopped in the middle of it.
    for start in 1..=3 {
        assert_eq!(
            run(&dir, way, &["crash"]).1,
            Some(3),
            "{way:?}: start {start}"
        );
    }
    // The fourth start sets it aside and the agent starts.
    assert_eq!(
        run(&dir, way, &[]),
        ("started\n".to_string(), Some(0)),
        "{way:?}"
    );
    let folder = dir.join("agents/Bot");
    let aside = std::fs::read_to_string(folder.join("set-aside")).unwrap();
    assert!(
        aside.starts_with("{\"event\":1,\"handler\":\"request\",\"args\":[7]}"),
        "{aside}"
    );
    assert_eq!(aside.matches("{\"again\":1}").count(), 2, "{aside}");
    let audit = std::fs::read_to_string(folder.join("audit.log")).unwrap();
    assert!(
        audit.contains("Bot set aside: event #1 of request (started 3 times without finishing)"),
        "{audit}"
    );
    // Nothing is left to finish: the next start runs nothing again.
    assert_eq!(
        run(&dir, way, &[]),
        ("started\n".to_string(), Some(0)),
        "{way:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_event_that_keeps_stopping_the_program_is_set_aside() {
    set_aside(Way::Interpreted);
}

#[test]
fn native_code_sets_such_an_event_aside_too() {
    set_aside(Way::Native);
}

fn locked(way: Way) {
    let dir = project(&format!("lock-{way:?}"));
    let mut first = command(&dir, way, &["hold"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // Once the first program says it started, it holds the folder.
    let mut line = String::new();
    BufReader::new(first.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert_eq!(line, "started\n", "{way:?}");
    let (said, code) = run(&dir, way, &[]);
    let _ = first.kill();
    let _ = first.wait();
    assert_eq!(code, Some(101), "{way:?}: {said}");
    assert!(
        said.contains("panic: the agent Bot already runs in another program: its folder ")
            && said.contains("is locked"),
        "{way:?}: {said}"
    );
    // With the first program gone, the agent starts again.
    assert_eq!(
        run(&dir, way, &[]),
        ("started\n".to_string(), Some(0)),
        "{way:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_second_program_cant_start_a_running_agent() {
    locked(Way::Interpreted);
}

#[test]
fn native_code_locks_the_agents_folder_too() {
    locked(Way::Native);
}
