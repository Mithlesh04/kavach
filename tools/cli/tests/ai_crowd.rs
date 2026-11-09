//! Many `ask`s at once against a provider that takes only a few calls at
//! the same time (spec 8.4): every one gets its answer, through the gate
//! all of a program's `ask`s share, in the interpreter and native code.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// At most this many calls at once; more are turned away (status 429).
const TAKES: usize = 4;

/// A server that answers each call after 30 ms with `{"ok":true}`, but
/// turns a call away at once while `TAKES` others are under way. Gives
/// how many it answered and how many it turned away.
fn crowded_model(answers: usize) -> (String, std::thread::JoinHandle<(usize, usize)>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (under_way, answered, refused) = (
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        );
        listener.set_nonblocking(true).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
        while answered.load(Ordering::SeqCst) < answers && std::time::Instant::now() < deadline {
            let Ok((mut s, _)) = listener.accept() else {
                std::thread::sleep(std::time::Duration::from_millis(2));
                continue;
            };
            let (under_way, answered, refused) =
                (under_way.clone(), answered.clone(), refused.clone());
            std::thread::spawn(move || {
                s.set_nonblocking(false).unwrap();
                let mut got = Vec::new();
                let mut buf = [0u8; 8192];
                loop {
                    let n = s.read(&mut buf).unwrap_or(0);
                    got.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&got);
                    let whole = text.find("\r\n\r\n").is_some_and(|end| {
                        let length = text[..end]
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length: "))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        got.len() >= end + 4 + length
                    });
                    if whole || n == 0 {
                        break;
                    }
                }
                let (status, body) = if under_way.fetch_add(1, Ordering::SeqCst) >= TAKES {
                    refused.fetch_add(1, Ordering::SeqCst);
                    ("429 Too Many Requests", r#"{"type":"error","error":{"type":"rate_limit_error","message":"too many at once"}}"#.to_string())
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(30));
                    answered.fetch_add(1, Ordering::SeqCst);
                    ("200 OK", r#"{"content":[{"type":"text","text":"{\"ok\":true}"}],"stop_reason":"end_turn","usage":{"input_tokens":20,"output_tokens":5}}"#.to_string())
                };
                under_way.fetch_sub(1, Ordering::SeqCst);
                let reply = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = s.write_all(reply.as_bytes());
            });
        }
        (
            answered.load(Ordering::SeqCst),
            refused.load(Ordering::SeqCst),
        )
    });
    (format!("http://{address}"), handle)
}

const MANY: &str = "type Yes {
    ok: Bool
}

fn one() -> Bool ! Error uses Ai {
    (try ask<Yes>(\"Is this fine?\")).ok
}

fn main() ! Error uses Ai, Console {
    let asked: [Task<Bool>] = []
    for _ in 0..40 {
        asked.push(spawn one())
    }
    let yes = 0
    for a in asked {
        if try a.wait() {
            yes += 1
        }
    }
    print(yes)
}
";

#[test]
fn forty_asks_at_once_all_get_through_a_provider_that_takes_four() {
    for native in [false, true] {
        let (url, server) = crowded_model(40);
        let dir =
            std::env::temp_dir().join(format!("kavach-crowd-{}-{native}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("many.kav");
        std::fs::write(&file, MANY).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd
            .arg(&file)
            .env("KAVACH_AI_URL", &url)
            .env("ANTHROPIC_API_KEY", "sk-fake")
            .output()
            .unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let text = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        let (answered, refused) = server.join().unwrap();
        assert_eq!(
            text, "40\n",
            "native: {native}, answered {answered}, turned away {refused}"
        );
        assert_eq!(answered, 40, "native: {native}");
    }
}
