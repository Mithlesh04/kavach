// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Providers and the errors a program can tell apart (spec 8.4, decision
//! 0140), end to end against a fake server on this machine
//! (`KAVACH_AI_URL`): no real API is called. Each test runs with the
//! interpreter and as native code.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::time::{Duration, Instant};

/// A server that answers one request per reply, in order, each reply the
/// status line's rest, its headers and its body; it gives back each
/// request it was sent, head and body.
fn fake_server(
    replies: Vec<(&'static str, &'static str, String)>,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut all = Vec::new();
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);
        for (status, headers, body) in replies {
            let mut s = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    // A run that failed before it sent must not hang here.
                    Err(_) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => return all,
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
                    let length = (text[..end].lines())
                        .find_map(|l| l.strip_prefix("content-length: "))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if got.len() >= end + 4 + length {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            all.push(String::from_utf8_lossy(&got).into_owned());
            let reply = format!(
                "HTTP/1.1 {status}\r\n{headers}content-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            s.write_all(reply.as_bytes()).unwrap();
        }
        all
    });
    (format!("http://{address}"), handle)
}

/// Anthropic's answer with `json` as its text, or with another stop.
fn answer(json: &str, stop: &str) -> (&'static str, &'static str, String) {
    let quoted = json.replace('\\', "\\\\").replace('"', "\\\"");
    let body = format!(
        "{{\"content\":[{{\"type\":\"text\",\"text\":\"{quoted}\"}}],\"stop_reason\":\"{stop}\",\"usage\":{{\"input_tokens\":60,\"output_tokens\":10}}}}"
    );
    ("200 OK", "", body)
}

/// Anthropic's error body.
fn error(status: &'static str, kind: &str, message: &str) -> (&'static str, &'static str, String) {
    let body = format!(
        "{{\"type\":\"error\",\"error\":{{\"type\":\"{kind}\",\"message\":\"{message}\"}}}}"
    );
    (status, "", body)
}

const PROGRAM: &str = "type Yes {
    ok: Bool
}

fn main() {
    const y = ask<Yes>(\"Fine?\", model: Model(name: \"claude-haiku-5-5\", retries: 1)) catch e => {
        print(`failed: {e}`)
        return
    }
    print(y.ok)
}
";

/// Runs `program`, the interpreter's way or as native code, with the
/// fake server's address in `KAVACH_AI_URL` and a key in `key_variable`.
fn run(program: &str, url: &str, key_variable: &str, native: bool) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("kavach-ai-providers-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("ask.kav");
    std::fs::write(&file, program).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    cmd.arg("run");
    if native {
        cmd.arg("--native").env("KAVACH_CHECK_LEAKS", "1");
    }
    let out = cmd
        .arg(&file)
        .env_remove("ANTHROPIC_API_KEY")
        .env("KAVACH_AI_URL", url)
        .env(key_variable, "sk-fake")
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

const MODES: [bool; 2] = [false, true];

#[test]
fn a_rate_limited_call_waits_as_long_as_retry_after_says() {
    for native in MODES {
        let busy = ("429 Too Many Requests", "retry-after: 2\r\n", String::new());
        let (url, server) = fake_server(vec![busy, answer(r#"{"ok":true}"#, "end_turn")]);
        let started = Instant::now();
        let out = run(PROGRAM, &url, "ANTHROPIC_API_KEY", native);
        assert_eq!(server.join().unwrap().len(), 2);
        assert_eq!(out, "true\n", "native: {native}");
        // The first step alone waits at most a second.
        assert!(
            started.elapsed() >= Duration::from_secs(2),
            "native: {native}"
        );
    }
}

#[test]
fn each_kind_of_trouble_is_its_own_error() {
    let cases = [
        (
            error("402 Payment Required", "billing_error", "no credit"),
            1,
            "failed: QuotaUsedUp(message: \"no credit\")\n",
        ),
        (
            error("401 Unauthorized", "authentication_error", "bad key"),
            1,
            "failed: BadKey(message: \"bad key\")\n",
        ),
        (
            error("404 Not Found", "not_found_error", "no model"),
            1,
            "failed: NotFound(model: \"claude-haiku-5-5\")\n",
        ),
        (
            error("400 Bad Request", "invalid_request_error", "bad"),
            1,
            "failed: BadRequest(message: \"bad\")\n",
        ),
        (answer("", "refusal"), 1, "failed: Refused(reason: none)\n"),
        (
            answer(r#"{"o"#, "max_tokens"),
            1,
            "failed: CutOff(max_tokens: 4096)\n",
        ),
        (
            error("418 I'm a teapot", "teapot", "short and stout"),
            1,
            "failed: Failed(status: 418, message: \"short and stout\")\n",
        ),
    ];
    for native in MODES {
        for (reply, sent, printed) in &cases {
            // One reply: a call sent again would find no server, and
            // fail with `Network` instead.
            let (url, server) = fake_server(vec![reply.clone()]);
            let out = run(PROGRAM, &url, "ANTHROPIC_API_KEY", native);
            assert_eq!(server.join().unwrap().len(), *sent, "{printed}");
            assert_eq!(&out, printed, "native: {native}");
        }
    }
}

#[test]
fn an_overloaded_provider_is_asked_again_then_fails_as_overloaded() {
    for native in MODES {
        let busy = error("529 Overloaded", "overloaded_error", "busy");
        let (url, server) = fake_server(vec![busy.clone(), busy]);
        let out = run(PROGRAM, &url, "ANTHROPIC_API_KEY", native);
        assert_eq!(
            server.join().unwrap().len(),
            2,
            "one try, then `retries: 1` more"
        );
        assert_eq!(
            out, "failed: Overloaded(message: \"busy\")\n",
            "native: {native}"
        );
    }
}

#[test]
fn no_connection_is_a_network_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    for native in MODES {
        let out = run(PROGRAM, &url, "ANTHROPIC_API_KEY", native);
        assert!(
            out.starts_with("failed: Network(message: "),
            "native: {native}: {out}"
        );
    }
}

/// A provider the program writes itself: another path, its key as a
/// bearer token from its own variable, and a reply whose body is the
/// answer.
const OWN: &str = "type Yes {
    ok: Bool
}

type Plain: Provider {
    fn check(self, model: Model) -> [String] { [] }
    fn key_variable(self) -> String { \"PLAIN_AI_KEY\" }
    fn auth(self, model: Model) -> Auth { Auth.Bearer }
    fn request(self, call: AskCall) -> HttpCall {
        HttpCall(url: \"https://plain.example/ask\", headers: {\"x-model\": call.model.name}, body: call.prompt)
    }
    fn read(self, call: AskCall, reply: HttpReply) -> Reply {
        if reply.status == 200 {
            Reply.Answer(text: reply.body, usage: Usage(input: 3))
        } else {
            Reply.Problem(kind: Trouble.Other(status: reply.status), message: reply.body, wait: none)
        }
    }
}

fn plain() -> Model {
    Model(name: \"tiny\", provider: Plain())
}

fn main() {
    const y = ask<Yes>(\"Fine?\", model: plain()) catch e => {
        print(`failed: {e}`)
        return
    }
    print(y.ok)
}
";

#[test]
fn a_programs_own_provider_translates_and_the_runtime_sends() {
    let mut sent = Vec::new();
    for native in MODES {
        let (url, server) = fake_server(vec![("200 OK", "", r#"{"ok":true}"#.into())]);
        let out = run(OWN, &url, "PLAIN_AI_KEY", native);
        let got = server.join().unwrap();
        assert_eq!(out, "true\n", "native: {native}");
        let request = &got[0];
        // `KAVACH_AI_URL` keeps the provider's path.
        assert!(request.starts_with("POST /ask HTTP/1.1"), "{request}");
        assert!(
            request.contains("authorization: Bearer sk-fake"),
            "{request}"
        );
        assert!(request.contains("x-model: tiny"), "{request}");
        // The type is in the prompt, as for Anthropic (decision 0057).
        assert!(
            request.contains("Fine?\n\nAnswer with only this JSON"),
            "{request}"
        );
        sent.push(got);
    }
    assert_eq!(
        sent[0], sent[1],
        "native code sends what the interpreter sends"
    );
}
