//! Agents in a project (spec 8.5, 8.6): their `settings`, modes, the audit
//! log, and the state kept by default. Each test writes a small project to
//! a folder of its own and runs it.

use std::path::{Path, PathBuf};
use std::process::Command;

const BOT: &str = "type Receipt { id: String }
type PaymentError = Declined | Denied(reason: String)

capability Payments {
    fn refund(order_id: Int, amount: Int) -> Receipt ! PaymentError
}

type FakePayments: Payments {
    fn refund(self, order_id: Int, amount: Int) -> Receipt ! PaymentError {
        Receipt(id: \"r{order_id}\")
    }
}

agent Bot {
    can Payments.refund

    limit Payments.refund {
        amount <= 100
    }

    on refund(order_id: Int, amount: Int) -> Receipt ! PaymentError {
        try Payments.refund(order_id, amount)
    }
}

fn main() {
    const bot = Bot.start(Payments: FakePayments())
    for (id, amount) in [(1, 50), (2, 150)] {
        const r = bot.refund(id, amount) catch e => {
            print(\"{id}: {e}\")
            continue
        }
        print(\"{id}: {r.id}\")
    }
}
";

const TOML: &str = "[package]\nname = \"bot\"\n";

/// `src`, with `settings <settings>` in agent `Bot` (decision 0060).
fn with_settings(src: &str, settings: &str) -> String {
    match settings {
        "" => src.to_string(),
        s => src.replace(
            "agent Bot {\n",
            &format!("agent Bot {{\n    settings {s}\n"),
        ),
    }
}

/// Writes the project (`kavach.toml` and `src/main.kav`, its agent with
/// these `settings`) and runs it.
fn run_project(name: &str, settings: &str) -> (String, PathBuf) {
    let dir = std::env::temp_dir().join(format!("kavach-agents-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), TOML).unwrap();
    std::fs::write(dir.join("src/main.kav"), with_settings(BOT, settings)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("run")
        .arg(dir.join("src/main.kav"))
        .output()
        .unwrap();
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (text, dir)
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn enforce_refuses_and_the_audit_log_says_why() {
    let settings = "AgentSettings(audit: Audit(file: \"audit/bot.log\"))";
    let (out, dir) = run_project("enforce", settings);
    assert_eq!(
        out,
        "1: r1\n2: Denied(reason: \"limit broken: amount <= 100\")\n"
    );
    let log = read(&dir.join("audit/bot.log"));
    std::fs::remove_dir_all(&dir).unwrap();
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 6, "{log}");
    assert!(
        lines[0].ends_with("Bot request#1 Payments.refund(order_id: 1, amount: 50)"),
        "{log}"
    );
    assert_eq!(lines[1], "  verdict: allowed");
    assert_eq!(lines[2], "  result: ok Receipt(id: \"r1\")");
    assert!(
        lines[3].ends_with("Bot request#2 Payments.refund(order_id: 2, amount: 150)"),
        "{log}"
    );
    assert_eq!(lines[4], "  verdict: refused (limit broken: amount <= 100)");
    assert_eq!(lines[5], "  result: denied: limit broken: amount <= 100");
}

#[test]
fn observe_runs_every_call_and_reports_what_it_would_refuse() {
    let settings = "AgentSettings(mode: AgentMode.Observe, audit: Audit(file: \"audit.log\"))";
    let (out, dir) = run_project("observe", settings);
    assert_eq!(out, "1: r1\n2: r2\n");
    let log = read(&dir.join("audit.log"));
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(log.contains("  verdict: would be refused (limit broken: amount <= 100)\n  result: ok Receipt(id: \"r2\")"), "{log}");
}

/// With no settings, an agent in a project keeps what survives a restart,
/// its state and its journal, in `agents/<Name>/`, and no audit log.
#[test]
fn without_settings_only_state_and_journal_are_kept() {
    let (out, dir) = run_project("quiet", "");
    let names = |d: &Path| -> Vec<String> {
        let mut n: Vec<String> = (std::fs::read_dir(d).unwrap().flatten())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        n.sort();
        n
    };
    let (top, kept) = (names(&dir), names(&dir.join("agents/Bot")));
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        out,
        "1: r1\n2: Denied(reason: \"limit broken: amount <= 100\")\n"
    );
    assert_eq!(top, ["agents", "kavach.toml", "src"]);
    // `lock`: the folder is held while the program runs (decision 0073).
    assert_eq!(kept, ["journal", "lock", "state.json"]);
}

const DRY: &str = "type Receipt { id: String }
type PaymentError = Declined | Denied(reason: String)
type Decision { refund: Bool }

capability Payments {
    fn refund(order_id: Int, amount: Int) -> Receipt ! PaymentError
}

type LoudPayments: Payments {
    fn refund(self, order_id: Int, amount: Int) -> Receipt ! PaymentError uses Console {
        print(\"real refund {order_id}\")
        Receipt(id: \"r{order_id}\")
    }
}

agent Bot {
    can Ai, Payments.refund

    on refund(order_id: Int, amount: Int) ! Error {
        const d = try ask<Decision>(\"Refund {order_id}?\")
        if d.refund {
            const _r = try Payments.refund(order_id, amount)
        }
    }
}

fn main() ! Error {
    const bot = Bot.start(Payments: LoudPayments())
    try bot.refund(7, 50)
    print(\"done\")
}
";

/// `kavach run --dry-run` (spec 8.5): nothing runs, no model is called,
/// every call is printed, and two runs print the same.
#[test]
fn a_dry_run_answers_calls_without_running_them() {
    let dir = std::env::temp_dir().join(format!("kavach-agents-dry-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("dry.kav");
    std::fs::write(&file, DRY).unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_kavach"))
            .args(["run", "--dry-run"])
            .arg(&file)
            .env_remove("ANTHROPIC_API_KEY")
            .env_remove("KAVACH_AI_URL")
            .output()
            .unwrap()
    };
    let (first, second) = (run(), run());
    std::fs::remove_dir_all(&dir).unwrap();
    let stdout = String::from_utf8_lossy(&first.stdout);
    let stderr = String::from_utf8_lossy(&first.stderr);
    assert_eq!(
        stdout,
        "done
",
        "{stderr}"
    );
    assert!(
        stderr.starts_with("[dry run] ask(\"Refund 7?\") -> Decision(refund: "),
        "{stderr}"
    );
    assert!(!stdout.contains("real refund"));
    assert_eq!(
        first.stderr, second.stderr,
        "a dry run gives the same answers every time"
    );
}

const SECRET: &str = "type Receipt { id: String }
type PaymentError = Declined | Denied(reason: String)
type Order {
    id: Int
    secret card: String
}

capability Payments {
    fn charge(secret card: String, amount: Int) -> Receipt ! PaymentError
}

type FakePayments: Payments {
    fn charge(self, secret card: String, amount: Int) -> Receipt ! PaymentError {
        Receipt(id: \"c1\")
    }
}

agent Bot {
    can Payments.charge

    on pay(order: Order) -> Receipt ! PaymentError {
        try Payments.charge(order.card, 25)
    }
}

fn main() ! Error {
    const bot = Bot.start(Payments: FakePayments())
    const r = try bot.pay(Order(id: 1, card: \"4242424242424242\"))
    print(r.id)
}
";

/// The audit log writes `<secret>` for a secret argument (spec 8.7).
#[test]
fn the_audit_log_never_shows_a_secret() {
    let dir = std::env::temp_dir().join(format!("kavach-agents-secret-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), TOML).unwrap();
    let settings = "AgentSettings(audit: Audit(file: \"audit.log\"))";
    std::fs::write(dir.join("src/main.kav"), with_settings(SECRET, settings)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("run")
        .arg(dir.join("src/main.kav"))
        .output()
        .unwrap();
    let log = read(&dir.join("audit.log"));
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "c1
",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        log.contains("Payments.charge(card: <secret>, amount: 25)"),
        "{log}"
    );
    assert!(!log.contains("4242"), "{log}");
}

const ONCE: &str = "type Receipt { id: String }
type PaymentError = Declined | Denied(reason: String)

capability Payments {
    fn refund(order_id: Int, amount: Int) -> Receipt ! PaymentError
}

type FakePayments: Payments {
    fn refund(self, order_id: Int, amount: Int) -> Receipt ! PaymentError {
        Receipt(id: \"r{order_id}\")
    }
}

agent Bot {
    can Payments.refund

    limit Payments.refund {
        count <= 2 per day
    }

    on refund(order_id: Int, amount: Int) -> Receipt ! PaymentError {
        try Payments.refund(order_id, amount)
    }
}

fn main() {
    const bot = Bot.start(Payments: FakePayments())
    const r = bot.refund(1, 10) catch e => {
        print(e)
        return
    }
    print(r.id)
}
";

/// Limit totals hold across runs (spec 8.5): in a project, an agent keeps
/// its state by default.
#[test]
fn limit_totals_survive_a_restart() {
    let dir = std::env::temp_dir().join(format!("kavach-agents-state-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), TOML).unwrap();
    std::fs::write(dir.join("src/main.kav"), ONCE).unwrap();
    let run = || {
        let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
            .arg("run")
            .arg(dir.join("src/main.kav"))
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let outputs = [run(), run(), run()];
    let state = read(&dir.join("agents/Bot/state.json"));
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(outputs[0], "r1\n");
    assert_eq!(outputs[1], "r1\n");
    assert_eq!(
        outputs[2],
        "Denied(reason: \"limit broken: count <= 2 per day\")\n"
    );
    assert!(
        state.contains("Payments.refund: count <= 2 per day"),
        "{state}"
    );
}

/// Under `--dry-run`, an `ask`'s audit entry names the model a real run
/// would use (spec 8.6), so it can be checked before a run that costs.
#[test]
fn a_dry_run_names_each_asks_model() {
    let dir = std::env::temp_dir().join(format!("kavach-agents-model-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), TOML).unwrap();
    let program = "type Yes {\n    ok: Bool\n}\n\nfn careful() -> Model {\n    Model(name: \"claude-opus-5-5\")\n}\n\nagent Bot {\n    model careful()\n    settings AgentSettings(audit: Audit(file: \"audit.log\"))\n    can Ai\n    on check() -> Bool ! Error {\n        (try ask<Yes>(\"a\")).ok\n    }\n}\n\nfn main() ! Error uses Ai {\n    const b = Bot.start()\n    const _ok = try b.check()\n}\n";
    std::fs::write(dir.join("src/main.kav"), program).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["run", "--dry-run"])
        .arg(dir.join("src/main.kav"))
        .env_remove("ANTHROPIC_API_KEY")
        .output()
        .unwrap();
    let log = std::fs::read_to_string(dir.join("audit.log")).unwrap_or_default();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(
        log.contains("  model: claude-opus-5-5 (dry run: no tokens)\n"),
        "{log}{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `Audit(format: AuditFormat.JsonLines)` (spec 8.6): one JSON object per entry, with
/// the model of an `ask`.
#[test]
fn the_audit_log_can_be_json_lines() {
    let dir = std::env::temp_dir().join(format!("kavach-agents-json-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), TOML).unwrap();
    let program = "type Yes {\n    ok: Bool\n}\n\nagent Bot {\n    model Model(name: \"claude-opus-5-5\")\n    settings AgentSettings(audit: Audit(file: \"audit.log\", format: AuditFormat.JsonLines))\n    can Ai\n    on check() -> Bool ! Error {\n        (try ask<Yes>(\"a\")).ok\n    }\n}\n\nfn main() ! Error uses Ai {\n    const b = Bot.start()\n    const _ok = try b.check()\n}\n";
    std::fs::write(dir.join("src/main.kav"), program).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["run", "--dry-run"])
        .arg(dir.join("src/main.kav"))
        .env_remove("ANTHROPIC_API_KEY")
        .output()
        .unwrap();
    let log = std::fs::read_to_string(dir.join("audit.log")).unwrap_or_default();
    std::fs::remove_dir_all(&dir).unwrap();
    let line = log.lines().next().unwrap_or_default();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        line.starts_with("{\"time\":\"") && line.ends_with('}'),
        "{log}{stderr}"
    );
    for field in [
        "\"agent\":\"Bot\"",
        "\"call\":\"Ai.ask\"",
        "\"verdict\":\"dry run\"",
        "\"model\":\"claude-opus-5-5\"",
    ] {
        assert!(line.contains(field), "{field} in {line}");
    }
}

/// A call a limit refuses in a dry run fails as it would for real, and
/// its line says so (spec 8.5).
#[test]
fn a_dry_run_says_which_calls_are_refused() {
    let src = DRY
        .replace("    can Ai, Payments.refund\n", "    can Ai, Payments.refund\n\n    limit Payments.refund {\n        amount <= 100\n    }\n")
        .replace("        const d = try ask<Decision>(\"Refund {order_id}?\")\n        if d.refund {\n            const _r = try Payments.refund(order_id, amount)\n        }", "        const _r = try Payments.refund(order_id, amount)")
        .replace("try bot.refund(7, 50)", "bot.refund(7, 500) catch e => print(\"{e}\")");
    let dir =
        std::env::temp_dir().join(format!("kavach-agents-dry-refused-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("dry.kav");
    std::fs::write(&file, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["run", "--dry-run"])
        .arg(&file)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("[dry run] Payments.refund(order_id: 7, amount: 500) refused: limit broken: amount <= 100"),
        "{stderr}"
    );
    assert!(!String::from_utf8_lossy(&out.stdout).contains("real refund"));
}
