//! `std.log` with a `[log]` section in `kavach.toml` (spec 14.12): the
//! level, a file instead of the console, and files set aside when full,
//! the same in the interpreter and native code.

use std::path::{Path, PathBuf};
use std::process::Command;

const PROGRAM: &str = "use std.log

type Step: LogEvent {
    n: Int
}

fn main() {
    for n in 1..=4 {
        log.debug(Step(n: n))
    }
    print(\"done\")
}
";

/// Each file line is the time (20 bytes), a space and `DEBUG Step(n: 1)`
/// (16 bytes), and its line end: 38 bytes, so two lines fill a file.
const TOML: &str = "[package]\nname = \"steps\"\n\n[log]\nlevel = \"debug\"\nconsole = false\nfile = \"logs/app.log\"\nmax_bytes = 80\nkeep = 1\n";

fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kavach-log-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), TOML).unwrap();
    std::fs::write(dir.join("src/main.kav"), PROGRAM).unwrap();
    dir
}

/// A log file's lines without their times.
fn lines(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .map(|l| l.split_once(' ').map_or(l, |(_, rest)| rest).to_string())
        .collect()
}

#[test]
fn log_settings_choose_level_file_and_rotation() {
    for native in [false, true] {
        let dir = project(if native { "native" } else { "interp" });
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.env("KAVACH_CHECK_LEAKS", "1").arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd.arg(dir.join("src/main.kav")).output().unwrap();
        let printed = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        // Nothing on the console but the program's own `print`.
        assert_eq!(printed, "done\n", "native: {native}");
        // Four lines, two a file: the older two set aside, nothing older kept.
        let logs = dir.join("logs");
        assert_eq!(
            lines(&logs.join("app.log")),
            ["DEBUG Step(n: 3)", "DEBUG Step(n: 4)"]
        );
        assert_eq!(
            lines(&logs.join("app.log.1")),
            ["DEBUG Step(n: 1)", "DEBUG Step(n: 2)"]
        );
        assert!(!logs.join("app.log.2").exists(), "native: {native}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// `[log.levels]`: an event type's level wins over its module's, which
/// wins over `level`.
#[test]
fn levels_by_module_and_event_type() {
    let billing = "use std.log

pub type Charged: LogEvent {
    cents: Int
}

pub type Retrying: LogEvent {
    attempt: Int
}

pub fn charge() {
    log.debug(Charged(cents: 500))
    log.warn(Retrying(attempt: 2))
    log.error(Retrying(attempt: 3))
}
";
    let main = "use std.log
use billing

type Started: LogEvent {}

fn main() {
    log.info(Started())
    log.error(Started())
    billing.charge()
}
";
    let toml = "[package]\nname = \"shop\"\n\n[log]\nlevel = \"warn\"\n\n[log.levels]\nbilling = \"debug\"\nRetrying = \"error\"\n";
    let dir = std::env::temp_dir().join(format!("kavach-log-levels-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), toml).unwrap();
    std::fs::write(dir.join("src/main.kav"), main).unwrap();
    std::fs::write(dir.join("src/billing.kav"), billing).unwrap();
    for native in [false, true] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.env("KAVACH_CHECK_LEAKS", "1").arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd.arg(dir.join("src/main.kav")).output().unwrap();
        let printed = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            printed, "ERROR Started\nDEBUG Charged(cents: 500)\nERROR Retrying(attempt: 3)\n",
            "native: {native}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A collector on this machine: answers each HTTP request with 200 and
/// gives back the bodies it got, once nothing more comes for a while.
fn fake_collector() -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let handle = std::thread::spawn(move || {
        let mut bodies = Vec::new();
        let mut quiet_until = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while std::time::Instant::now() < quiet_until {
            let Ok((mut s, _)) = listener.accept() else {
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            };
            s.set_nonblocking(false).unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = s.read(&mut buf).unwrap_or(0);
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got).to_string();
                if let Some(end) = text.find("\r\n\r\n") {
                    let length = text[..end]
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length: "))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if got.len() >= end + 4 + length {
                        bodies.push(text[end + 4..].to_string());
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let _ =
                s.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}");
            quiet_until = std::time::Instant::now() + std::time::Duration::from_secs(3);
        }
        bodies
    });
    (format!("http://{address}"), handle)
}

/// `[log.otlp]`: events also go to an OpenTelemetry collector, as OTLP
/// JSON, the last ones when the program ends.
#[test]
fn events_go_to_an_opentelemetry_collector() {
    let program = "use std.log

type Charged: LogEvent {
    cents: Int
}

fn main() {
    log.info(Charged(cents: 500))
    log.debug(Charged(cents: 1))
    log.error(Charged(cents: 7))
}
";
    for native in [false, true] {
        let (url, collector) = fake_collector();
        let toml = format!("[package]\nname = \"shop\"\n\n[log.otlp]\nendpoint = \"{url}\"\n");
        let dir = std::env::temp_dir().join(format!("kavach-otlp-{native}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("kavach.toml"), toml).unwrap();
        std::fs::write(dir.join("src/main.kav"), program).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.env("KAVACH_CHECK_LEAKS", "1").arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd.arg(dir.join("src/main.kav")).output().unwrap();
        let printed = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            printed, "INFO Charged(cents: 500)\nERROR Charged(cents: 7)\n",
            "native: {native}"
        );
        let sent = collector.join().unwrap().join("");
        for part in [
            "\"service.name\"",
            "\"stringValue\":\"shop\"",
            "\"severityText\":\"INFO\",\"body\":{\"stringValue\":\"Charged(cents: 500)\"}",
            "\"severityText\":\"ERROR\",\"body\":{\"stringValue\":\"Charged(cents: 7)\"}",
            "\"key\":\"event.name\",\"value\":{\"stringValue\":\"Charged\"}",
        ] {
            assert!(
                sent.contains(part),
                "native: {native}: {part} missing in {sent}"
            );
        }
        assert!(!sent.contains("cents: 1)"), "native: {native}: {sent}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// `Audit(otlp: true)` (spec 8.6): each audit entry also goes to the
/// collector, as an `AgentAudit` record with its fields as attributes.
#[test]
fn audit_entries_go_to_the_collector_too() {
    let program = "type Receipt { id: String }
type PaymentError = Declined | Denied(reason: String)

capability Payments {
    fn refund(order_id: Int, amount: Int) -> Receipt ! PaymentError
}

type Books: Payments {
    fn refund(self, order_id: Int, amount: Int) -> Receipt ! PaymentError {
        Receipt(id: \"r{order_id}\")
    }
}

agent Bot {
    settings AgentSettings(audit: Audit(file: \"audit.log\", otlp: true))
    can Payments.refund
    limit Payments.refund {
        amount <= 100
    }
    on refund(order_id: Int, amount: Int) -> Receipt ! PaymentError {
        try Payments.refund(order_id, amount)
    }
}

fn main() {
    const bot = Bot.start(Payments: Books())
    for (id, amount) in [(1, 50), (2, 150)] {
        const _r = bot.refund(id, amount) catch _ => Receipt(id: \"none\")
    }
}
";
    for native in [false, true] {
        let (url, collector) = fake_collector();
        let toml = format!("[package]\nname = \"shop\"\n\n[log.otlp]\nendpoint = \"{url}\"\n");
        let dir =
            std::env::temp_dir().join(format!("kavach-audit-otlp-{native}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("kavach.toml"), toml).unwrap();
        std::fs::write(dir.join("src/main.kav"), program).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd.arg(dir.join("src/main.kav")).output().unwrap();
        let sent = collector.join().unwrap().join("");
        let stderr = String::from_utf8_lossy(&out.stderr);
        for part in [
            "\"key\":\"event.name\",\"value\":{\"stringValue\":\"AgentAudit\"}",
            "\"key\":\"agent\",\"value\":{\"stringValue\":\"Bot\"}",
            "\"key\":\"call\",\"value\":{\"stringValue\":\"Payments.refund\"}",
            "\"severityText\":\"INFO\"",
            "\"severityText\":\"WARN\"",
            "limit broken: amount <= 100",
        ] {
            assert!(
                sent.contains(part),
                "native: {native}: {part} missing in {sent} {stderr}"
            );
        }
        assert!(dir.join("audit.log").exists(), "the file stays the record");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Spans and metrics (spec 14.12, decision 0107) go to the collector too:
/// spans to `/v1/traces`, a failed block's marked as an error, events in
/// a span carrying its ids, and metrics to `/v1/metrics` at the end.
#[test]
fn spans_and_metrics_go_to_the_collector() {
    let program = "use std.log

type Paid: LogEvent {
    cents: Int
}

type Declined = Card

fn pay(ok: Bool) -> Int ! Declined {
    log.span(\"pay\") {
        if not ok {
            fail Declined.Card
        }
        log.info(Paid(cents: 500))
        500
    }
}

fn main() {
    log.span(\"checkout\") {
        log.name_span(\"checkout /cart/\\{id}\")
        print(pay(true) catch _ => 0)
        print(pay(false) catch _ => 0)
    }
    log.count(\"orders\", 2, attributes: [\"route\": \"/cart\"])
    log.histogram(\"checkout_ms\", 12.5)
}
";
    for native in [false, true] {
        let (url, collector) = fake_collector();
        let toml = format!("[package]\nname = \"shop\"\n\n[log.otlp]\nendpoint = \"{url}\"\n");
        let dir =
            std::env::temp_dir().join(format!("kavach-spans-{native}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("kavach.toml"), toml).unwrap();
        std::fs::write(dir.join("src/main.kav"), program).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd.arg(dir.join("src/main.kav")).output().unwrap();
        let printed = String::from_utf8_lossy(&out.stdout).into_owned();
        assert_eq!(
            printed, "INFO Paid(cents: 500)\n500\n0\n",
            "native: {native}"
        );
        let sent = collector.join().unwrap().join("\n");
        for part in [
            "\"resourceSpans\"",
            "\"name\":\"checkout /cart/{id}\"",
            "\"name\":\"pay\"",
            "\"parentSpanId\"",
            "\"code\":2",
            "\"traceId\"",
            "\"name\":\"orders\",\"sum\"",
            "\"key\":\"route\",\"value\":{\"stringValue\":\"/cart\"}",
            "\"asDouble\":2",
            "\"name\":\"checkout_ms\",\"histogram\"",
        ] {
            assert!(
                sent.contains(part),
                "native: {native}: {part} missing in {sent}"
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// K39: `http.serve`'s span is named by the method only, so a path's ids
/// and tokens never become span names sent to the collector.
#[test]
fn server_spans_are_named_by_method_not_path() {
    let program = "use std.http
use std.net

fn hello(r: Request) -> Response {
    http.text(200, \"hi\")
}

fn main() ! Error {
    const listener = try net.listen(\"127.0.0.1:0\")
    const _server = spawn http.serve(listener, hello)
    const got = try http.get(\"http://{listener.address()}/users/7/reset/secret-token\")
    print(got.status)
}
";
    for native in [false, true] {
        let (url, collector) = fake_collector();
        let toml = format!(
            "[package]
name = \"shop\"

[log.otlp]
endpoint = \"{url}\"
"
        );
        let dir =
            std::env::temp_dir().join(format!("kavach-route-{native}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("kavach.toml"), toml).unwrap();
        std::fs::write(dir.join("src/main.kav"), program).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd.arg(dir.join("src/main.kav")).output().unwrap();
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "200
",
            "native: {native}"
        );
        let sent = collector.join().unwrap().join(
            "
",
        );
        assert!(
            sent.contains("\"name\":\"GET\""),
            "native: {native}: {sent}"
        );
        assert!(!sent.contains("secret-token"), "native: {native}: {sent}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
