//! Tests that need the internet, or change the machine's clipboard, so
//! they don't run by default: run them with
//! `cargo test -p kavach-cli --test online -- --ignored`.

use std::process::Command;

/// `https://` through TLS (decision 0021): a real site works, and one with
/// an expired certificate is refused, in the interpreter and native code.
#[test]
#[ignore = "needs the internet"]
fn https_checks_certificates() {
    let dir = std::env::temp_dir().join(format!("kavach-online-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("https.kav");
    std::fs::write(
        &file,
        "use std.http\n\nfn main() ! Error {\n    \
         const page = try http.get(\"https://example.com/\")\n    \
         print(page.status)\n    \
         const bad = http.get(\"https://expired.badssl.com/\") catch e => {\n        \
         print(\"refused\")\n        return\n    }\n    print(\"accepted\")\n}\n",
    )
    .unwrap();
    for how in [&["run"][..], &["run", "--native"][..]] {
        let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
            .args(how)
            .arg(&file)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            stdout,
            "200\nrefused\n",
            "{how:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// HTTP/3 over QUIC (spec 14.10, 14.11, decision 0022): a real server
/// answers, in the interpreter and native code. Its certificate is checked
/// as for TLS over TCP.
#[test]
#[ignore = "needs the internet"]
fn http3_gets_a_page_over_quic() {
    let dir = std::env::temp_dir().join(format!("kavach-online-h3-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("h3.kav");
    std::fs::write(
        &file,
        "use std.http

fn main() ! Error {
             const page = try http.get3(\"https://cloudflare-quic.com/\")
             print(page.status)
             print(page.headers.get(\"alt-svc\") != none)
}
",
    )
    .unwrap();
    for how in [&["run"][..], &["run", "--native"][..]] {
        let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
            .args(how)
            .arg(&file)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            stdout,
            "200
true
",
            "{how:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// ALPN (spec 14.10): a server that speaks HTTP/2 chooses `h2` when it
/// is offered, and nothing is chosen when nothing was offered.
#[test]
#[ignore = "needs the internet"]
fn tls_offers_protocols() {
    let dir = std::env::temp_dir().join(format!("kavach-alpn-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("alpn.kav");
    std::fs::write(
        &file,
        "use std.net

fn main() ! Error {
             const a = try net.connect_tls(\"example.com:443\", protocols: [\"h2\", \"http/1.1\"])
             print(a.protocol() ?? \"none\")
             const b = try net.connect_tls(\"example.com:443\")
             print(b.protocol() ?? \"none\")
}
",
    )
    .unwrap();
    for how in [&["run"][..], &["run", "--native"][..]] {
        let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
            .args(how)
            .arg(&file)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            stdout,
            "h2
none
",
            "{how:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `ask<T>` against the real model (spec 8.4): one small call, so it costs
/// a fraction of a cent. Needs `ANTHROPIC_API_KEY`.
#[test]
#[ignore = "needs the internet and an API key; costs money"]
fn ask_gets_a_typed_answer_from_the_model() {
    let dir = std::env::temp_dir().join(format!("kavach-ask-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("ask.kav");
    std::fs::write(
        &file,
        "type Check {
    /// true if the number is prime
    prime: Bool
}

         fn main() ! Error {
    const c = try ask<Check>(\"Is 7 a prime number?\")
    print(c.prime)
}
",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("run")
        .arg(&file)
        .env_remove("KAVACH_AI_URL")
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "true
",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `std.clipboard` on the machine's real clipboard (spec 14.17): what a
/// program writes is read back, in the interpreter and native code. What
/// was on the clipboard is put back afterwards.
#[test]
#[ignore = "changes the machine's clipboard for a moment"]
fn the_clipboard_takes_text_and_gives_it_back() {
    let src = "use std.clipboard

fn main() ! Error uses Clipboard, Console {
    const before = clipboard.read_text() catch _ => \"\"
    try clipboard.write_text(\"kavach clipboard test\")
    const got = try clipboard.read_text()
    try clipboard.write_text(before)
    print(got)
}
";
    let dir = std::env::temp_dir().join(format!("kavach-clipboard-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.kav");
    std::fs::write(&file, src).unwrap();
    for native in [false, true] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
        cmd.arg("run");
        if native {
            cmd.arg("--native");
        }
        let out = cmd.arg(&file).output().unwrap();
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "kavach clipboard test\n",
            "native {native}: {said}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
