// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;

mod fallbacks;
mod signing;

/// A provider written in Rust, as `std.ai`'s `wire` answers: the request
/// goes to `url` with the key where `auth` says (`x-api-key`); a
/// 200 is an answer, its body the answer's text; other statuses are the
/// trouble the test names in `kinds`.
struct Fake {
    url: String,
    /// `Auth` as `json.stringify` writes it.
    auth: &'static str,
    kinds: Vec<(i64, &'static str)>,
    waited: Vec<Duration>,
}

impl Side for Fake {
    type Value = Tree;
    type Stop = ();

    fn translate(&mut self, model: usize, input: &str) -> Result<String, ()> {
        Ok(self.answer(model, input))
    }

    fn wait_until(&mut self, until: Instant) -> Result<(), ()> {
        let left = until.saturating_duration_since(Instant::now());
        if left > Duration::from_millis(50) {
            // A retry's wait: noted, not waited, so tests stay fast.
            self.waited.push(left);
            return Ok(());
        }
        std::thread::sleep(left);
        Ok(())
    }

    fn fits(&mut self, answer: &Tree) -> Option<Tree> {
        matches!(answer, Tree::Object(_)).then(|| answer.clone())
    }
}

impl Fake {
    fn answer(&mut self, model: usize, input: &str) -> String {
        let tree = json::parse(input, &Limits::default()).unwrap();
        let op = wire::field(&tree, "op").cloned();
        if op == Some(Tree::Text("request".into())) {
            let auth = self.auth;
            return format!(
                r#"{{"url":"{}/v1/messages","headers":{{"content-type":"application/json","x-model":"{model}"}},"body":"{{}}","auth":{auth},"key_variable":"TEST_FAKE_AI_KEY"}}"#,
                self.url
            );
        }
        let Some(Tree::Int(status)) = wire::field(&tree, "status").cloned() else {
            panic!("no status in {input}");
        };
        let Some(Tree::Text(body)) = wire::field(&tree, "body").cloned() else {
            panic!("no body in {input}");
        };
        if status == 451 {
            return r#"{"Refused":{"reason":"no","usage":{}}}"#.into();
        }
        if status == 200 {
            let text = json::write(&Tree::Text(body), 0);
            return format!(
                r#"{{"Answer":{{"text":{text},"usage":{{"input":5,"output":null}}}}}}"#
            );
        }
        let kind = (self.kinds.iter().find(|(s, _)| *s == status))
            .map_or(format!(r#"{{"Other":{{"status":{status}}}}}"#), |(_, k)| {
                format!("\"{k}\"")
            });
        format!(r#"{{"Problem":{{"kind":{kind},"message":"no","wait":{{"ms":1}}}}}}"#)
    }
}

/// A server that answers one request per reply, in order, and gives back
/// what it was sent.
fn fake_server(replies: Vec<&'static str>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut all = Vec::new();
        for reply in replies {
            let (mut s, _) = listener.accept().unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = s.read(&mut buf).unwrap();
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got);
                if let Some(end) = text.find("\r\n\r\n") {
                    let length = (text[..end].lines())
                        .find_map(|l| l.strip_prefix("content-length: "))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if got.len() >= end + 4 + length || n == 0 {
                        break;
                    }
                }
            }
            s.write_all(reply.as_bytes()).unwrap();
            all.push(String::from_utf8_lossy(&got).into_owned());
        }
        all
    });
    (format!("http://{address}"), handle)
}

fn settings(retries: i64) -> Tree {
    Tree::Object(vec![
        ("model".into(), Tree::Text("test-model".into())),
        ("max_tokens".into(), Tree::Int(64)),
        ("retries".into(), Tree::Int(retries)),
        ("api_key_env".into(), Tree::Text("TEST_FAKE_AI_KEY".into())),
        ("provider".into(), Tree::Text("Fake".into())),
    ])
}

/// The keys every test sees: fake AWS credentials as `--env` values,
/// which beat the real environment, so no test reads real ones; and a
/// plain key. Every test sets the same, as tests run at once.
fn set_keys() {
    let forced = [
        ("AWS_ACCESS_KEY_ID", "FAKEKEY"),
        ("AWS_SECRET_ACCESS_KEY", "fake-secret"),
        ("AWS_SESSION_TOKEN", "fake-token"),
    ];
    let forced = forced.iter().map(|(k, v)| (k.to_string(), v.to_string()));
    kavach_runtime_core::env_file::set(
        forced.collect(),
        vec![("TEST_FAKE_AI_KEY".into(), "sk-t".into())],
    );
}

fn ask_fake(fake: &mut Fake, retries: i64) -> Asked<Tree> {
    set_keys();
    let schema = Tree::Object(vec![("type".into(), Tree::Text("object".into()))]);
    let Ok(asked) = ask(fake, &[settings(retries)], "Refund?", &schema, false) else {
        panic!("stopped");
    };
    asked
}

fn fake(url: &str) -> Fake {
    Fake {
        url: url.to_string(),
        auth: r#"{"InHeader":{"name":"x-api-key"}}"#,
        kinds: vec![(429, "RateLimited"), (402, "QuotaUsedUp")],
        waited: Vec::new(),
    }
}

const OK: &str =
    "HTTP/1.1 200 OK\r\nconnection: close\r\ncontent-length: 14\r\n\r\n{\"approve\":1}\n";
// A wait long enough that a slow, busy machine never sees it run out
// before the next try (the fake notes waits, never sleeps them).
const BUSY: &str = "HTTP/1.1 429 Too Many Requests\r\nretry-after: 60\r\ncontent-length: 0\r\n\r\n";
const BROKE: &str = "HTTP/1.1 402 Payment Required\r\ncontent-length: 0\r\n\r\n";

#[test]
fn the_key_goes_where_the_provider_says_and_the_answer_is_read() {
    let (url, server) = fake_server(vec![OK]);
    let mut f = fake(&url);
    let asked = ask_fake(&mut f, 0);
    let sent = server.join().unwrap();
    assert!(
        sent[0].starts_with("POST /v1/messages HTTP/1.1"),
        "{}",
        sent[0]
    );
    assert!(sent[0].contains("x-api-key: sk-t"), "{}", sent[0]);
    assert!(
        sent[0].contains("content-type: application/json"),
        "{}",
        sent[0]
    );
    assert!(matches!(asked.result, Ok(Tree::Object(_))));
    // An output count the provider didn't give is 0.
    assert_eq!(
        asked.tokens,
        Tokens {
            input: 5,
            output: 0
        }
    );
}

#[test]
fn a_rate_limited_call_waits_as_the_provider_asks_then_goes_again() {
    let (url, server) = fake_server(vec![BUSY, OK]);
    let mut f = fake(&url);
    let asked = ask_fake(&mut f, 2);
    assert_eq!(server.join().unwrap().len(), 2);
    assert!(asked.result.is_ok());
    assert_eq!(f.waited.len(), 1);
}

#[test]
fn a_quota_used_up_is_never_sent_again() {
    let (url, server) = fake_server(vec![BROKE]);
    let mut f = fake(&url);
    let asked = ask_fake(&mut f, 4);
    assert_eq!(server.join().unwrap().len(), 1);
    assert_eq!(asked.result, Err(Failure::QuotaUsedUp("no".into())));
}

#[test]
fn trouble_the_provider_cant_place_is_failed_with_its_status() {
    let (url, server) = fake_server(vec!["HTTP/1.1 418 Teapot\r\ncontent-length: 0\r\n\r\n"]);
    let mut f = fake(&url);
    let asked = ask_fake(&mut f, 4);
    server.join().unwrap();
    assert_eq!(asked.result, Err(Failure::Failed(418, "no".into())));
}

#[test]
fn no_connection_is_a_network_failure_after_the_retries() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let mut f = fake(&url);
    let asked = ask_fake(&mut f, 1);
    assert!(
        matches!(asked.result, Err(Failure::Network(_))),
        "{:?}",
        asked.result.err()
    );
    assert_eq!(f.waited.len(), 1);
}

#[test]
fn a_provider_without_a_key_reads_no_variable() {
    let (url, server) = fake_server(vec![OK]);
    let mut f = fake(&url);
    f.auth = r#""NoKey""#;
    let schema = Tree::Object(Vec::new());
    let mut s = settings(0);
    if let Tree::Object(fs) = &mut s {
        fs.retain(|(k, _)| k != "api_key_env");
    }
    let Ok(asked) = ask(&mut f, &[s], "?", &schema, false) else {
        panic!("stopped");
    };
    let sent = server.join().unwrap();
    assert!(asked.result.is_ok());
    assert!(!sent[0].contains("x-api-key"), "{}", sent[0]);
}

#[test]
fn a_missing_key_sends_nothing() {
    let mut f = fake("http://127.0.0.1:9");
    let mut s = settings(0);
    if let Tree::Object(fs) = &mut s {
        fs.retain(|(k, _)| k != "api_key_env");
        fs.push((
            "api_key_env".into(),
            Tree::Text("TEST_NOT_SET_AI_KEY".into()),
        ));
    }
    let Ok(asked) = ask(&mut f, &[s], "?", &Tree::Object(Vec::new()), false) else {
        panic!("stopped");
    };
    assert_eq!(
        asked.result,
        Err(Failure::NoKey("TEST_NOT_SET_AI_KEY".into()))
    );
}

#[test]
fn an_error_is_the_ai_errors_json() {
    let t = json::write(&Failure::TimedOut(30_000).tree(), 0);
    assert_eq!(t, r#"{"TimedOut":{"after":{"ms":30000}}}"#);
    let t = json::write(&Failure::Refused(None).tree(), 0);
    assert_eq!(t, r#"{"Refused":{"reason":null}}"#);
}

#[test]
fn kavach_ai_url_keeps_the_providers_path() {
    let to = "http://127.0.0.1:5/proxy/";
    assert_eq!(
        send::moved("https://api.anthropic.com/v1/messages", to),
        "http://127.0.0.1:5/proxy/v1/messages"
    );
}

#[test]
fn a_reply_the_runtime_cant_read_fails_the_ask() {
    assert!(matches!(
        wire::Reply::read("null"),
        Err(Failure::Failed(0, _))
    ));
    assert!(
        matches!(wire::Call::read(r#"{"url":"x","auth":{"GoogleToken":{"scope":"s"}}}"#), Err(Failure::Failed(0, m)) if m.contains("not built yet"))
    );
}

#[test]
fn chunked_bodies_are_joined() {
    assert_eq!(
        http::dechunk(b"4\r\nWiki\r\n5;x=1\r\npedia\r\n0\r\n\r\n").unwrap(),
        b"Wikipedia"
    );
    assert!(http::dechunk(b"9\r\nshort\r\n").is_err());
    let (status, headers, body) = http::parse(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nRetry-After: 2\r\n\r\n2\r\nhi\r\n0\r\n\r\n",
    )
    .unwrap();
    assert_eq!((status, body.as_slice()), (200, &b"hi"[..]));
    assert!(headers.contains(&("retry-after".to_string(), "2".to_string())));
}

#[test]
fn each_provider_and_key_has_its_own_gate() {
    let a = gate::of("Fake A");
    a.enter();
    a.leave(true, false);
    assert_eq!(a.limit(), 1.0);
    assert_eq!(gate::of("Fake A").limit(), 1.0);
    assert_eq!(gate::of("Fake B").limit(), gate::MOST);
}
#[test]
fn the_gate_lets_through_what_the_provider_takes() {
    let g = gate::Gate::new();
    for _ in 0..10 {
        g.enter();
    }
    // Turned away with 9 others under way: at most 9 at once from now.
    g.leave(true, false);
    assert_eq!(g.limit(), 9.0);
    // Others turned away as the burst drains lower it to what is left.
    for _ in 0..5 {
        g.leave(true, false);
    }
    assert_eq!(g.limit(), 4.0);
    // Answers raise it a little at a time, one over as many answers.
    for _ in 0..4 {
        g.leave(false, true);
    }
    assert!(g.limit() > 4.0 && g.limit() < 5.0, "{}", g.limit());
    // It never goes below one call, nor above the most.
    g.enter();
    g.leave(true, false);
    assert_eq!(g.limit(), 1.0);
    for _ in 0..100_000 {
        g.enter();
        g.leave(false, true);
    }
    assert_eq!(g.limit(), gate::MOST);
}

#[test]
fn a_call_waits_at_a_full_gate_until_another_ends() {
    let g = std::sync::Arc::new(gate::Gate::new());
    g.enter();
    g.leave(true, false); // at most 1 at once
    g.enter();
    let (g2, (tx, rx)) = (g.clone(), std::sync::mpsc::channel());
    let waiting = std::thread::spawn(move || {
        g2.enter();
        tx.send(()).unwrap();
        g2.leave(false, true);
    });
    assert!(
        rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "went through a full gate"
    );
    g.leave(false, true);
    assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
    waiting.join().unwrap();
}

#[test]
fn waits_are_spread_over_half_to_all_of_each_step() {
    for (k, step) in [(0, 1000), (1, 2000), (2, 4000), (3, 8000), (7, 8000)] {
        let waits: Vec<u128> = (0..50).map(|_| gate::wait_before(k).as_millis()).collect();
        assert!(
            waits.iter().all(|w| (step / 2..=step).contains(w)),
            "{k}: {waits:?}"
        );
        assert!(waits.iter().any(|w| *w != waits[0]), "{k}: all the same");
    }
}
