//! Durable agents (spec 8.5, decision 0025): a handler stopped half way
//! runs again when the agent starts, and a call its journal already has
//! does not run twice.

use std::process::Command;

const PROGRAM: &str = "use std.cli

type Receipt { id: String }
type PaymentError = Declined | Denied(reason: String)
type EmailError = NotSent | Denied(reason: String)

capability Payments {
    fn refund(order_id: Int) -> Receipt ! PaymentError
}

capability Email {
    fn send(to: String) ! EmailError
}

type Shop: Payments {
    fn refund(self, order_id: Int) -> Receipt ! PaymentError uses Console {
        print(\"[pay] refund {order_id}\")
        Receipt(id: \"r{order_id}\")
    }
}

type Mail: Email {
    crash: Bool

    fn send(self, to: String) ! EmailError uses Console, Env {
        if self.crash {
            cli.exit(3)            // the program stops half way
        }
        print(\"[email] {to}\")
    }
}

agent Bot {
    can Payments.refund, Email.send

    on request(order_id: Int) -> String ! Error {
        const r = try Payments.refund(order_id)
        try Email.send(\"order {order_id}: {r.id}\")
        r.id
    }
}

fn main() ! Error {
    const crash = cli.args().contains(\"crash\")
    const bot = Bot.start(Payments: Shop(), Email: Mail(crash: crash))
    if crash {
        const _id = try bot.request(7)
    }
    print(\"started\")
}
";

/// How the program runs: by the interpreter, as native code, or as an
/// executable from `kavach build`.
#[derive(Clone, Copy)]
enum Way {
    Interpreted,
    Native,
    Built,
}

fn kavach() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kavach"))
}

/// The story of one durable agent: the program stops half way through an
/// event (run `first`), then finishes it when it starts again (`then`).
fn story(name: &str, first: Way, then: Way) {
    let dir = std::env::temp_dir().join(format!("kavach-durable-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    // In a project, an agent keeps a journal by default (decision 0060).
    let toml = "[package]\nname = \"shop\"\n";
    std::fs::write(dir.join("kavach.toml"), toml).unwrap();
    let file = dir.join("src/main.kav");
    std::fs::write(&file, PROGRAM).unwrap();
    let exe = dir.join(format!("shop{}", std::env::consts::EXE_SUFFIX));
    if matches!(first, Way::Built) || matches!(then, Way::Built) {
        let built = kavach()
            .arg("build")
            .arg(&file)
            .arg("-o")
            .arg(&exe)
            .output();
        let built = built.unwrap();
        let said = String::from_utf8_lossy(&built.stderr);
        assert!(built.status.success(), "{said}");
    }
    let run = |way: Way, args: &[&str]| {
        let mut cmd = match way {
            Way::Interpreted => kavach(),
            Way::Native => kavach(),
            Way::Built => Command::new(&exe),
        };
        match way {
            Way::Interpreted => cmd.arg("run").arg(&file),
            Way::Native => cmd.args(["run", "--native"]).arg(&file),
            Way::Built => &mut cmd,
        };
        let out = cmd
            .env("KAVACH_CHECK_LEAKS", "1")
            .args(args)
            .output()
            .unwrap();
        let printed = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        (printed, out.status.code())
    };
    // The refund goes out, then the program stops before the email.
    let crashed = run(first, &["crash"]);
    assert_eq!(
        crashed,
        (
            "[pay] refund 7
"
            .to_string(),
            Some(3)
        ),
        "{name}"
    );
    // Starting again finishes the event: the email, but no second refund.
    let finished = (
        "[email] order 7: r7
started
"
        .to_string(),
        Some(0),
    );
    assert_eq!(run(then, &[]), finished, "{name}");
    let journal = std::fs::read_to_string(dir.join("agents/Bot/journal")).unwrap();
    // Now the journal has nothing left to finish.
    assert_eq!(
        run(then, &[]),
        (
            "started
"
            .to_string(),
            Some(0)
        ),
        "{name}"
    );
    // Starting again kept only what is unfinished: nothing, and the next
    // event's number.
    let kept = std::fs::read_to_string(dir.join("agents/Bot/journal")).unwrap();
    assert_eq!(
        kept,
        "{\"next\":2}
",
        "{name}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
    let lines: Vec<&str> = journal.lines().collect();
    assert_eq!(lines.len(), 5, "{name}: {journal}");
    assert_eq!(
        lines[0],
        "{\"event\":1,\"handler\":\"request\",\"args\":[7]}"
    );
    assert_eq!(
        lines[1],
        "{\"of\":1,\"call\":\"Payments.refund\",\"ok\":{\"id\":\"r7\"}}"
    );
    // The second start of event 1 (decision 0073 counts them).
    assert_eq!(lines[2], "{\"again\":1}");
    assert_eq!(lines[3], "{\"of\":1,\"call\":\"Email.send\",\"ok\":null}");
    assert_eq!(lines[4], "{\"done\":1}");
}

#[test]
fn an_event_cut_short_finishes_without_paying_twice() {
    story("interpreted", Way::Interpreted, Way::Interpreted);
}

#[test]
fn native_code_finishes_an_event_without_paying_twice() {
    story("native", Way::Native, Way::Native);
    story("built", Way::Built, Way::Built);
}

#[test]
fn native_code_finishes_what_the_interpreter_began() {
    story("mixed", Way::Interpreted, Way::Native);
}

/// An `ask` a limit refuses is journaled the same way by the interpreter
/// and native code, so a run after a crash gives the same refusal.
const REFUSED: &str = "agent Asker {
    can Ai

    limit Ai {
        count <= 0 per day
    }

    on question() -> Bool ! Error {
        ask<Bool>(\"Is water wet?\") catch _ => false
    }
}

fn main() ! Error {
    const bot = Asker.start()
    print(try bot.question())
}
";

#[test]
fn a_refused_ask_is_journaled_by_both_runs() {
    for native in [false, true] {
        let dir = std::env::temp_dir().join(format!(
            "kavach-durable-refused-{native}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("kavach.toml"),
            "[package]
name = \"asker\"
",
        )
        .unwrap();
        let file = dir.join("src/main.kav");
        std::fs::write(&file, REFUSED).unwrap();
        let mut cmd = kavach();
        cmd.arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd.arg(&file).output().unwrap();
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "false
",
            "{said}"
        );
        let journal = std::fs::read_to_string(dir.join("agents/Asker/journal")).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let lines: Vec<&str> = journal.lines().collect();
        assert_eq!(lines.len(), 3, "native {native}: {journal}");
        assert!(
            lines[1].starts_with("{\"of\":1,\"call\":\"Ai.ask\",\"error\":"),
            "native {native}: {journal}"
        );
    }
}

/// `agents.call_key()` (spec 14.16): an implementation gets its call's
/// key, the same in the run after a crash; outside a call, `none`.
const KEYED: &str = "use std.agents
use std.cli

type PaymentError = Declined | Denied(reason: String)
type EmailError = NotSent | Denied(reason: String)

capability Payments {
    fn refund(order_id: Int) ! PaymentError
}

capability Email {
    fn send(to: String) ! EmailError
}

type Shop: Payments {
    fn refund(self, order_id: Int) ! PaymentError uses Console {
        print(\"[pay] {agents.call_key()}\")
    }
}

type Mail: Email {
    crash: Bool

    fn send(self, to: String) ! EmailError uses Console, Env {
        print(\"[email] {agents.call_key()}\")
        if self.crash {
            cli.exit(3)
        }
    }
}

agent Bot {
    can Payments.refund, Email.send

    on request(order_id: Int) ! Error {
        try Payments.refund(order_id)
        try Email.send(\"order {order_id}\")
    }
}

fn main() ! Error {
    const crash = cli.args().contains(\"crash\")
    const bot = Bot.start(Payments: Shop(), Email: Mail(crash: crash))
    print(\"outside: {agents.call_key()}\")
    if crash {
        try bot.request(7)
    }
}
";

#[test]
fn a_call_keeps_its_key_after_a_crash() {
    for (first, then) in [
        (Way::Interpreted, Way::Interpreted),
        (Way::Native, Way::Native),
        (Way::Interpreted, Way::Native),
    ] {
        let dir = std::env::temp_dir().join(format!("kavach-durable-key-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"shop\"\n").unwrap();
        let file = dir.join("src/main.kav");
        std::fs::write(&file, KEYED).unwrap();
        let run = |way: Way, args: &[&str]| {
            let mut cmd = kavach();
            cmd.arg("run");
            if matches!(way, Way::Native) {
                cmd.arg("--native");
            }
            let out = cmd.arg(&file).args(args).output().unwrap();
            String::from_utf8_lossy(&out.stdout).into_owned()
                + &String::from_utf8_lossy(&out.stderr)
        };
        assert_eq!(
            run(first, &["crash"]),
            "outside: none\n[pay] Bot-1-1\n[email] Bot-1-2\n"
        );
        assert_eq!(run(then, &[]), "[email] Bot-1-2\noutside: none\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
