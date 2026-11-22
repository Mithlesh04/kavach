// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A fake AI provider on this machine (`KAVACH_AI_URL`), and runs of a
//! program against it, for the tests of providers and of costs (spec
//! 8.4): no real API is called.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::time::{Duration, Instant};

/// A server that answers one request per reply, in order, each reply the
/// status line's rest, its headers and its body; it gives back each
/// request it was sent, head and body.
pub fn fake_server(
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
pub fn answer(json: &str, stop: &str) -> (&'static str, &'static str, String) {
    let quoted = json.replace('\\', "\\\\").replace('"', "\\\"");
    let body = format!(
        "{{\"content\":[{{\"type\":\"text\",\"text\":\"{quoted}\"}}],\"stop_reason\":\"{stop}\",\"usage\":{{\"input_tokens\":60,\"output_tokens\":10}}}}"
    );
    ("200 OK", "", body)
}

/// Anthropic's error body.
pub fn error(
    status: &'static str,
    kind: &str,
    message: &str,
) -> (&'static str, &'static str, String) {
    let body = format!(
        "{{\"type\":\"error\",\"error\":{{\"type\":\"{kind}\",\"message\":\"{message}\"}}}}"
    );
    (status, "", body)
}

/// Runs `program`, the interpreter's way or as native code, with the
/// fake server's address in `KAVACH_AI_URL` and a key in `key_variable`.
pub fn run(program: &str, url: &str, key_variable: &str, native: bool) -> String {
    run_with(program, url, &[(key_variable, "sk-fake")], native).0
}

/// `run` with each of `keys` set; with the audit log `audit.log` beside
/// the program, if it wrote one.
pub fn run_with(program: &str, url: &str, keys: &[(&str, &str)], native: bool) -> (String, String) {
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
    cmd.arg(&file)
        .env_remove("ANTHROPIC_API_KEY")
        .env("KAVACH_AI_URL", url);
    for (variable, key) in keys {
        cmd.env(variable, key);
    }
    let out = cmd.output().unwrap();
    let log = std::fs::read_to_string(dir.join("audit.log")).unwrap_or_default();
    std::fs::remove_dir_all(&dir).unwrap();
    let printed =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (printed, log)
}

pub const MODES: [bool; 2] = [false, true];
