//! A `tokens` limit with many `ask`s at once (spec 8.3, "`ask`s under
//! way"): 10 `ask`s started together under `tokens <= 200 per day`, each
//! answer 70 tokens, each `ask` at most 3 × (1 + 30) = 93. How many run
//! is what each `asks_under_way` setting promises, in the interpreter and
//! native code:
//!
//! - `Average`: as when they run one at a time (0, 70 and 140 pass; 210
//!   doesn't), so 3;
//! - `Largest`: 93 and 186 pass, 279 doesn't, so 2;
//! - `NotCounted`: none is counted before it answers, so all 10.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// A model that answers every call after 100 ms with `{"ok":true}`, 50
/// tokens in and 20 out, until `done`. Gives how many it answered.
fn model(done: Arc<AtomicBool>) -> (String, std::thread::JoinHandle<usize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let answered = Arc::new(AtomicUsize::new(0));
        listener.set_nonblocking(true).unwrap();
        while !done.load(Ordering::SeqCst) {
            let Ok((mut s, _)) = listener.accept() else {
                std::thread::sleep(std::time::Duration::from_millis(2));
                continue;
            };
            let answered = answered.clone();
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
                std::thread::sleep(std::time::Duration::from_millis(100));
                answered.fetch_add(1, Ordering::SeqCst);
                let body = r#"{"content":[{"type":"text","text":"{\"ok\":true}"}],"stop_reason":"end_turn","usage":{"input_tokens":50,"output_tokens":20}}"#;
                let reply = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = s.write_all(reply.as_bytes());
            });
        }
        answered.load(Ordering::SeqCst)
    });
    (format!("http://{address}"), handle)
}

const PROGRAM: &str = "type Yes {
    ok: Bool
}

agent Writer {
    model Model(name: \"claude-haiku-5-5\", max_tokens: 30)
    settings AgentSettings(asks_under_way: UnderWay.WAY)
    can Ai
    limit Ai {
        tokens <= 200 per day
    }
    on write() -> Bool ! Error {
        const _y = ask<Yes>(\"a\") catch _ => {
            return false
        }
        true
    }
}

fn main() ! Error uses Ai, Console {
    const w = Writer.start()
    let asked: [Task<Bool>] = []
    for _ in 0..10 {
        asked.push(spawn w.write())
    }
    let answered = 0
    for a in asked {
        if try a.wait() {
            answered += 1
        }
    }
    print(\"answered {answered}\")
}
";

#[test]
fn each_way_of_counting_asks_under_way_keeps_its_promise() {
    for (way, runs) in [("Average", 3), ("Largest", 2), ("NotCounted", 10)] {
        for native in [false, true] {
            let done = Arc::new(AtomicBool::new(false));
            let (url, server) = model(done.clone());
            let dir = std::env::temp_dir().join(format!(
                "kavach-budget-{}-{way}-{native}",
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            let file = dir.join("writer.kav");
            std::fs::write(&file, PROGRAM.replace("WAY", way)).unwrap();
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
            cmd.arg("run");
            if native {
                cmd.arg("--native");
            }
            let out = cmd
                .arg(&file)
                .current_dir(&dir)
                .env("KAVACH_AI_URL", &url)
                .env("ANTHROPIC_API_KEY", "sk-fake")
                .output()
                .unwrap();
            done.store(true, Ordering::SeqCst);
            let asked = server.join().unwrap();
            std::fs::remove_dir_all(&dir).unwrap();
            let text = String::from_utf8_lossy(&out.stdout).into_owned()
                + &String::from_utf8_lossy(&out.stderr);
            let want = format!("answered {runs}\n");
            assert_eq!(text, want, "{way}, native: {native}");
            assert_eq!(asked, runs, "{way}, native: {native}: calls the model got");
        }
    }
}
