// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Requests signed with AWS Signature Version 4 (`Auth.AwsSignature`,
//! spec 8.4, *Signing AWS requests*), end to end against a fake server on
//! this machine (`KAVACH_AI_URL`): no real service is called, and the
//! credentials are fake ones set for the child process only. Each test
//! runs with the interpreter and as native code.

mod ai_fake;

use ai_fake::{MODES, fake_server, run_with};

/// A provider for an AWS service, written in the program: the runtime
/// signs its requests. Its address is never reached: `KAVACH_AI_URL`
/// sends every call to the fake server.
const SIGNED: &str = "type Yes {
    ok: Bool
}

type Signed: Provider {
    fn check(self, model: Model) -> [String] { [] }
    fn key_variable(self) -> String { \"UNUSED_AI_KEY\" }
    fn auth(self, model: Model) -> Auth {
        Auth.AwsSignature(service: \"bedrock\", region: \"us-east-1\")
    }
    fn request(self, call: AskCall) -> HttpCall {
        HttpCall(url: \"https://bedrock.example/model/m1:0/converse?b=2&a=1\",
            headers: {\"content-type\": \"application/json\"}, body: call.prompt)
    }
    fn read(self, call: AskCall, reply: HttpReply) -> Reply {
        if reply.status == 200 {
            Reply.Answer(text: reply.body, usage: Usage(input: 3))
        } else {
            Reply.Problem(kind: Trouble.Other(status: reply.status), message: reply.body, wait: none)
        }
    }
    fn price(self, model: String) -> Price? { none }
    fn count_request(self, call: AskCall) -> HttpCall? {
        HttpCall(url: \"https://bedrock.example/model/m1:0/count-tokens\", headers: {:}, body: call.prompt)
    }
    fn read_count(self, call: AskCall, reply: HttpReply) -> Int? {
        if reply.status == 200 { 7 } else { none }
    }
    fn estimate(self, call: AskCall) -> Estimate? { none }
}

fn signed() -> Model {
    Model(name: \"m1\", provider: Signed(), retries: 0)
}

fn main() {
    const y = ask<Yes>(\"Fine?\", model: signed()) catch e => {
        print(`failed: {e}`)
        return
    }
    print(y.ok)
}
";

const ID: (&str, &str) = ("AWS_ACCESS_KEY_ID", "FAKEKEY");
const SECRET: (&str, &str) = ("AWS_SECRET_ACCESS_KEY", "fake-secret");
const TOKEN: (&str, &str) = ("AWS_SESSION_TOKEN", "fake-session-token");

/// The value of header `name` in `request`.
fn header<'r>(request: &'r str, name: &str) -> Option<&'r str> {
    let head = &request[..request.find("\r\n\r\n")?];
    head.split("\r\n")
        .find_map(|l| l.strip_prefix(name)?.strip_prefix(": "))
}

/// Checks that `request` is signed with SigV4 for bedrock in us-east-1,
/// with the signed headers `signed`; gives its signature.
fn check_signed(request: &str, signed: &str) -> String {
    let date = header(request, "x-amz-date").unwrap_or_else(|| panic!("{request}"));
    assert!(
        date.len() == 16 && date.as_bytes()[8] == b'T' && date.ends_with('Z'),
        "{date}"
    );
    let authorization = header(request, "authorization").unwrap_or_else(|| panic!("{request}"));
    let want = format!(
        "AWS4-HMAC-SHA256 Credential=FAKEKEY/{}/us-east-1/bedrock/aws4_request, SignedHeaders={signed}, Signature=",
        &date[..8]
    );
    let signature = authorization
        .strip_prefix(&want)
        .unwrap_or_else(|| panic!("{authorization}"));
    assert!(
        signature.len() == 64 && signature.bytes().all(|b| b.is_ascii_hexdigit()),
        "{signature}"
    );
    // The secret itself is never sent.
    assert!(!request.contains("fake-secret"), "{request}");
    signature.to_string()
}

#[test]
fn an_ask_is_signed_with_the_credentials_of_the_environment() {
    for native in MODES {
        let (url, server) = fake_server(vec![("200 OK", "", r#"{"ok":true}"#.into())]);
        let (out, _) = run_with(SIGNED, &url, &[ID, SECRET], native);
        let sent = server.join().unwrap();
        assert_eq!(out, "true\n", "native: {native}");
        let request = &sent[0];
        // Sent where `KAVACH_AI_URL` says, and signed for that address.
        assert!(
            request.starts_with("POST /model/m1:0/converse?b=2&a=1 HTTP/1.1"),
            "{request}"
        );
        let host = url.strip_prefix("http://").unwrap();
        assert_eq!(header(request, "host"), Some(host), "{request}");
        check_signed(request, "content-type;host;x-amz-date");
        assert_eq!(header(request, "x-amz-security-token"), None);
    }
}

#[test]
fn a_session_token_is_sent_and_signed() {
    for native in MODES {
        let (url, server) = fake_server(vec![("200 OK", "", r#"{"ok":true}"#.into())]);
        let (out, _) = run_with(SIGNED, &url, &[ID, SECRET, TOKEN], native);
        let sent = server.join().unwrap();
        assert_eq!(out, "true\n", "native: {native}");
        check_signed(
            &sent[0],
            "content-type;host;x-amz-date;x-amz-security-token",
        );
        assert_eq!(
            header(&sent[0], "x-amz-security-token"),
            Some("fake-session-token")
        );
    }
}

#[test]
fn missing_credentials_are_no_key_and_send_nothing() {
    let cases: [(&[(&str, &str)], &str); 2] = [
        (&[], "AWS_ACCESS_KEY_ID"),
        // Set to empty text: not set.
        (
            &[ID, ("AWS_SECRET_ACCESS_KEY", "")],
            "AWS_SECRET_ACCESS_KEY",
        ),
    ];
    for native in MODES {
        for (keys, variable) in cases {
            let (url, server) = fake_server(vec![]);
            let (out, _) = run_with(SIGNED, &url, keys, native);
            assert!(server.join().unwrap().is_empty(), "native: {native}");
            assert_eq!(
                out,
                format!("failed: NoKey(variable: \"{variable}\")\n"),
                "native: {native}"
            );
        }
    }
}

#[test]
fn a_counting_call_is_signed_too() {
    let program = SIGNED.replace(
        "const y = ask<Yes>(\"Fine?\", model: signed()) catch e => {",
        "const y = ai.count(\"Fine?\", model: signed()) catch e => {",
    );
    let program = program.replace("print(y.ok)", "print(y.tokens)");
    // `Yes` goes, as an unused type warns.
    let program = format!(
        "use std.ai\n\n{}",
        program.replace("type Yes {\n    ok: Bool\n}\n\n", "")
    );
    for native in MODES {
        let (url, server) = fake_server(vec![("200 OK", "", String::new())]);
        let (out, _) = run_with(&program, &url, &[ID, SECRET], native);
        let sent = server.join().unwrap();
        assert_eq!(out, "7\n", "native: {native}");
        assert!(
            sent[0].starts_with("POST /model/m1:0/count-tokens HTTP/1.1"),
            "{}",
            sent[0]
        );
        check_signed(&sent[0], "host;x-amz-date");
    }
}
