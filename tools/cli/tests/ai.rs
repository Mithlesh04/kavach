//! `ask<T>` (spec 8.4) end to end, against a fake model server on this
//! machine (`KAVACH_AI_URL`): no real API is called, so nothing costs.
//! Each test runs with the interpreter and as native code, which must
//! send the same requests and print the same.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;

/// A server that answers one request per reply, in order, with `input`
/// as the JSON of its answer, and gives back the bodies it was sent.
fn fake_model(inputs: Vec<&'static str>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut bodies = Vec::new();
        // A run that never connects (it failed first) must not hang the
        // test: stop waiting after a while.
        listener.set_nonblocking(true).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        for input in inputs {
            let mut s = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(_) if std::time::Instant::now() < deadline => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(e) => panic!("no request came: {e}"),
                }
            };
            s.set_nonblocking(false).unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = s.read(&mut buf).unwrap();
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got);
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
            // Like the API: the answer's JSON is the text of a text block;
            // `busy` is the API turning the call away (status 429).
            let quoted = input.replace('\\', "\\\\").replace('"', "\\\"");
            let (status, body) = match input {
                "busy" => (
                    "429 Too Many Requests",
                    r#"{"type":"error","error":{"type":"rate_limit_error","message":"busy"}}"#
                        .to_string(),
                ),
                _ => (
                    "200 OK",
                    format!(
                        "{{\"content\":[{{\"type\":\"text\",\"text\":\"{quoted}\"}}],\"stop_reason\":\"end_turn\",\"usage\":{{\"input_tokens\":60,\"output_tokens\":10}}}}"
                    ),
                ),
            };
            let reply = format!(
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            s.write_all(reply.as_bytes()).unwrap();
        }
        bodies
    });
    (format!("http://{address}"), handle)
}

const PROGRAM: &str = "type RefundDecision {
    /// true if the refund should be issued
    approve: Bool
    reason: String
}

fn main() {
    const d = ask<RefundDecision>(\"Refund order 7?\") catch e => {
        print(\"failed: {e}\")
        return
    }
    print(d.approve)
    print(d.reason)
}
";

/// The two ways a program runs: the interpreter, and native code.
const MODES: [bool; 2] = [false, true];

/// Runs `PROGRAM` with the interpreter or, with `native`, as native code;
/// `url` and `key` set the variables.
fn run(url: Option<&str>, key: Option<&str>, native: bool) -> String {
    run_program(PROGRAM, url, key, native)
}

fn run_program(program: &str, url: Option<&str>, key: Option<&str>, native: bool) -> String {
    let dir =
        std::env::temp_dir().join(format!("kavach-ai-{}-{}", std::process::id(), rand_part()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("ask.kav");
    std::fs::write(&file, program).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    cmd.arg("run");
    if native {
        cmd.arg("--native").env("KAVACH_CHECK_LEAKS", "1");
    }
    cmd.arg(&file)
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("KAVACH_AI_URL");
    if let Some(u) = url {
        cmd.env("KAVACH_AI_URL", u);
    }
    if let Some(k) = key {
        cmd.env("ANTHROPIC_API_KEY", k);
    }
    let out = cmd.output().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

fn rand_part() -> u128 {
    // The clock alone collides: Windows gives the same nanosecond to
    // tests that start together, and one test then removes the other's
    // folder. A counter keeps every name distinct.
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .wrapping_mul(1000)
        .wrapping_add(n as u128)
}

#[test]
fn the_answer_is_a_checked_value() {
    let mut sent = Vec::new();
    for native in MODES {
        let (url, server) = fake_model(vec![r#"{"approve":true,"reason":"It arrived broken."}"#]);
        let out = run(Some(&url), Some("sk-fake"), native);
        sent.push(server.join().unwrap());
        assert_eq!(out, "true\nIt arrived broken.\n", "native: {native}");
    }
    // Native code sends exactly what the interpreter sends.
    assert_eq!(sent[0], sent[1]);
    let bodies = &sent[0];
    // The first try writes the type into the prompt (decision 0057),
    // with the doc comment as a note, and no structured output.
    assert!(
        bodies[0].contains(r#"Answer with only this JSON: {\"approve\": bool (true if the refund should be issued), \"reason\": string}"#),
        "{}",
        bodies[0]
    );
    assert!(!bodies[0].contains("output_config"), "{}", bodies[0]);
    assert!(!bodies[0].contains("tool_choice"), "{}", bodies[0]);
}

#[test]
fn an_answer_that_doesnt_fit_is_asked_again() {
    for native in MODES {
        let (url, server) = fake_model(vec![
            r#"{"approve":"yes"}"#,
            r#"{"approve":false,"reason":"No."}"#,
        ]);
        let out = run(Some(&url), Some("sk-fake"), native);
        let bodies = server.join().unwrap();
        assert_eq!(bodies.len(), 2);
        assert_eq!(out, "false\nNo.\n", "native: {native}");
        // The second try is held to the schema, made from the type with
        // its doc comment: structured output, not a forced tool call.
        let b = &bodies[1];
        assert!(
            b.contains("\"output_config\":{\"format\":{\"type\":\"json_schema\",\"schema\":"),
            "{b}"
        );
        assert!(b.contains("\"approve\":{\"type\":\"boolean\",\"description\":\"true if the refund should be issued\"}"), "{b}");
        assert!(
            b.contains("\"required\":[\"approve\",\"reason\"],\"additionalProperties\":false"),
            "{b}"
        );
        assert!(
            !b.contains("tool_choice") && !b.contains("Answer with only"),
            "{b}"
        );
    }
}

#[test]
fn three_answers_that_dont_fit_fail() {
    for native in MODES {
        let (url, server) = fake_model(vec![r#"{}"#, r#"{}"#, r#"{"approve":1}"#]);
        let out = run(Some(&url), Some("sk-fake"), native);
        assert_eq!(server.join().unwrap().len(), 3);
        assert_eq!(
            out, "failed: InvalidOutput(answer: \"{\\\"approve\\\":1}\")\n",
            "native: {native}"
        );
    }
}

#[test]
fn without_a_key_nothing_is_sent() {
    for native in MODES {
        let out = run(Some("http://127.0.0.1:9"), None, native);
        assert_eq!(
            out, "failed: NoKey(variable: \"ANTHROPIC_API_KEY\")\n",
            "native: {native}"
        );
    }
}

const LIMITED: &str = "type Check {
    prime: Bool
}

agent Bot {
    can Ai

    limit Ai {
        count <= 1 per day
    }

    on check(n: Int) -> Check ! Error {
        try ask<Check>(\"Is {n} prime?\")
    }
}

fn main() {
    const bot = Bot.start()
    for n in [7, 8] {
        const c = bot.check(n) catch e => {
            print(\"{n}: {e}\")
            continue
        }
        print(\"{n}: {c.prime}\")
    }
}
";

#[test]
fn a_limit_on_ai_refuses_the_call_before_it_is_sent() {
    let dir = std::env::temp_dir().join(format!("kavach-ai-limit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("limited.kav");
    std::fs::write(&file, LIMITED).unwrap();
    for native in MODES {
        let (url, server) = fake_model(vec![r#"{"prime":true}"#]);
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.arg("run");
        if native {
            cmd.arg("--native").env("KAVACH_CHECK_LEAKS", "1");
        }
        let out = cmd
            .arg(&file)
            .env("KAVACH_AI_URL", &url)
            .env("ANTHROPIC_API_KEY", "sk-fake")
            .output()
            .unwrap();
        // Only one request reached the server.
        assert_eq!(server.join().unwrap().len(), 1);
        let printed = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            printed,
            "7: true
8: Denied(reason: \"limit broken: count <= 1 per day\")
",
            "native: {native}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// An agent's `model`, a call's own `model:` and the default (spec 8.4,
/// decision 0056): each request names its model and settings.
const MODELS: &str = "type Yes {
    ok: Bool
}

fn careful() -> Model {
    Model(
        name: \"claude-sonnet-5-5\",
        effort: Effort.High,
        max_tokens: 300,
        thinking: false,
        instructions: \"Be brief.\",
        stop: [\"END\"],
        timeout: 30.seconds,
        retries: 2,
        extra: \"\\{\\\"metadata\\\": \\{\\\"user_id\\\": \\\"shop-7\\\"}}\",
    )
}

agent Reviewer {
    model careful()
    can Ai
    on check() -> Bool ! Error {
        (try ask<Yes>(\"a\")).ok
    }
    on quick() -> Bool ! Error {
        (try ask<Yes>(\"b\", model: Model(name: \"claude-haiku-5-5\", max_tokens: 50, thinking: false))).ok
    }
}

fn main() ! Error uses Ai, Console {
    const r = Reviewer.start()
    print(try r.check())
    print(try r.quick())
    print((try ask<Yes>(\"c\")).ok)
}
";

#[test]
fn each_ask_names_its_model_and_settings() {
    let mut sent = Vec::new();
    for native in MODES {
        let (url, server) = fake_model(vec![r#"{"ok":true}"#; 3]);
        let out = run_program(MODELS, Some(&url), Some("sk-fake"), native);
        sent.push(server.join().unwrap());
        assert_eq!(out, "true\ntrue\ntrue\n", "native: {native}");
    }
    assert_eq!(
        sent[0], sent[1],
        "native code sends what the interpreter sends"
    );
    let b = &sent[0];
    assert!(
        b[0].contains("\"model\":\"claude-sonnet-5-5\",\"max_tokens\":300"),
        "{}",
        b[0]
    );
    // Each setting of the agent's model (decision 0059).
    for part in [
        r#""output_config":{"effort":"high"}"#,
        r#""system":"Be brief.""#,
        r#""stop_sequences":["END"]"#,
        r#""metadata":{"user_id":"shop-7"}"#,
    ] {
        assert!(b[0].contains(part), "{part} in {}", b[0]);
    }
    // The call's own model wins over the agent's.
    assert!(
        b[1].contains("\"model\":\"claude-haiku-5-5\",\"max_tokens\":50"),
        "{}",
        b[1]
    );
    assert!(
        !b[1].contains("effort") && !b[1].contains("system"),
        "{}",
        b[1]
    );
    // Thinking off, each family its own way (spec 8.4).
    assert!(
        b[0].contains(r#""thinking":{"type":"between_tools"}"#),
        "{}",
        b[0]
    );
    assert!(
        b[1].contains(r#""thinking":{"type":"disabled"}"#),
        "{}",
        b[1]
    );
    assert!(!b[2].contains("thinking"), "{}", b[2]);
    // Outside an agent, with no model: the default.
    assert!(
        b[2].contains("\"model\":\"claude-haiku-5-5\",\"max_tokens\":4096"),
        "{}",
        b[2]
    );
}

/// A call the provider turns away as busy is sent again after a wait
/// (spec 8.4), so many `ask`s at once get through a limit on them.
#[test]
fn a_busy_provider_is_asked_again() {
    for native in MODES {
        let (url, server) = fake_model(vec!["busy", r#"{"approve":true,"reason":"Broken."}"#]);
        let out = run(Some(&url), Some("sk-fake"), native);
        assert_eq!(server.join().unwrap().len(), 2);
        assert_eq!(out, "true\nBroken.\n", "native: {native}");
    }
}

/// `limit Ai { tokens <= … }` (spec 8.3): once the window's asks took
/// more tokens than the limit, the next one is refused and never sent.
const BUDGET: &str = "type Yes {
    ok: Bool
}

agent Bot {
    can Ai
    limit Ai {
        tokens <= 100 per day
    }
    on check() -> Bool ! Error {
        (try ask<Yes>(\"a\")).ok
    }
}

fn main() ! Error uses Ai, Console {
    const b = Bot.start()
    for _ in 0..3 {
        const ok = b.check() catch e => {
            print(\"refused: {e}\")
            continue
        }
        print(ok)
    }
}
";

#[test]
fn a_token_budget_refuses_asks_past_it() {
    for native in MODES {
        // Each answer takes 60 + 10 tokens: 0, then 70, are under 100.
        let (url, server) = fake_model(vec![r#"{"ok":true}"#, r#"{"ok":true}"#]);
        let out = run_program(BUDGET, Some(&url), Some("sk-fake"), native);
        assert_eq!(server.join().unwrap().len(), 2, "the third is never sent");
        assert_eq!(
            out, "true\ntrue\nrefused: Denied(reason: \"limit broken: tokens <= 100 per day\")\n",
            "native: {native}"
        );
    }
}
