//! Agents in native code (spec 8.3–8.7) decide, log and keep totals
//! exactly like the interpreter: both run the same project, then each
//! runs again on the state file the other wrote.

use std::path::{Path, PathBuf};
use std::process::Command;

const BOT: &str = "type Receipt { id: String }
type PaymentError = Declined | Denied(reason: String)
type EmailError = NotSent | Denied(reason: String)

capability Payments {
    fn refund(order_id: Int, amount: Decimal, secret card: String) -> Receipt ! PaymentError
}

capability Email {
    fn send(to: String) ! EmailError
}

type FakePayments: Payments {
    fn refund(self, order_id: Int, amount: Decimal, secret card: String) -> Receipt ! PaymentError {
        if order_id == 3 {
            fail PaymentError.Declined
        }
        Receipt(id: \"r{order_id}\")
    }
}

type QuietEmail: Email {
    fn send(self, to: String) ! EmailError {
    }
}

agent Bot {
    settings AgentSettings(audit: Audit(file: \"audit/bot.log\"))
    can Payments.refund, Email.send

    limit Payments.refund {
        sum(amount) <= 150 per day
        count <= 3 per day
    }

    // No one answers here, so the request runs out of time (decision 0072).
    approve Payments.refund when amount > 60 by \"finance\" within 1.seconds

    on refund(order_id: Int, amount: Decimal) -> Receipt ! Error {
        const r = try Payments.refund(order_id, amount, \"4242\")
        // A task started in a handler runs inside the agent too.
        const sent = spawn Email.send(\"asha@example.com\")
        try sent.wait()
        r
    }
}

fn show(bot: Bot, order_id: Int, amount: Decimal) {
    const r = bot.refund(order_id, amount) catch e => {
        print(\"{order_id}: {e}\")
        return
    }
    print(\"{order_id}: {r.id}\")
}

fn main() {
    const bot = Bot.start(Payments: FakePayments(), Email: QuietEmail())
    show(bot, 1, 40.25)
    show(bot, 2, 70)
    show(bot, 3, 10)
    show(bot, 4, 50.25)
    show(bot, 5, 1)
}
";

// The agent's state is kept by default, in `agents/Bot/` (decision 0060).
const TOML: &str = "[package]\nname = \"bot\"\n";

fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kavach-native-agents-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), TOML).unwrap();
    std::fs::write(dir.join("src/main.kav"), BOT).unwrap();
    dir
}

/// Runs the project with the interpreter or, with `native`, as native
/// code (checking that every object is freed); gives its output.
fn run(dir: &Path, native: bool) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    cmd.env("KAVACH_CHECK_LEAKS", "1").arg("run");
    if native {
        cmd.arg("--native");
    }
    let out = cmd.arg(dir.join("src/main.kav")).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

/// The audit log without its times, which differ from run to run.
fn audit(dir: &Path) -> String {
    let log = std::fs::read_to_string(dir.join("audit/bot.log")).unwrap_or_default();
    log.lines()
        .map(|l| match l.starts_with(' ') {
            true => l.to_string(),
            false => l.split_once(' ').map_or(l, |(_, rest)| rest).to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn native_agents_match_the_interpreter() {
    let (interp, native) = (project("interp"), project("native"));
    let first = "1: r1
2: Denied(reason: \"not approved by finance (amount > 60) within 1 second\")
3: Declined
4: r4
5: r5
approval needed by finance: Payments.refund(order_id: 2, amount: 70, card: <secret>) (request Bot-2-1): kavach approve Bot-2-1 --as <name>, or kavach deny
";
    assert_eq!(run(&interp, false), first);
    assert_eq!(run(&native, true), first);
    let log = audit(&interp);
    assert_eq!(audit(&native), log);
    assert!(
        log.starts_with(
            "Bot request#1 Payments.refund(order_id: 1, amount: 40.25, card: <secret>)"
        ),
        "{log}"
    );
    assert!(
        log.contains(
            "Bot request#1 Email.send(to: \"asha@example.com\")\n  verdict: allowed\n  result: ok"
        ),
        "{log}"
    );
    assert!(log.contains("  result: error Declined"), "{log}");

    // Each runs again on the totals the other kept: 91.50 in 3 calls.
    let second = "1: Denied(reason: \"limit broken: count <= 3 per day\")
2: Denied(reason: \"limit broken: sum(amount) <= 150 per day\")
3: Denied(reason: \"limit broken: count <= 3 per day\")
4: Denied(reason: \"limit broken: count <= 3 per day\")
5: Denied(reason: \"limit broken: count <= 3 per day\")
";
    let state = "agents/Bot/state.json";
    let (from_interp, from_native) = (interp.join(state), native.join(state));
    let swapped =
        std::env::temp_dir().join(format!("kavach-native-agents-{}.json", std::process::id()));
    std::fs::rename(&from_interp, &swapped).unwrap();
    std::fs::rename(&from_native, &from_interp).unwrap();
    std::fs::rename(&swapped, &from_native).unwrap();
    assert_eq!(run(&interp, false), second);
    assert_eq!(run(&native, true), second);
    std::fs::remove_dir_all(&interp).unwrap();
    std::fs::remove_dir_all(&native).unwrap();
}
