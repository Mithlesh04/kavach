// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Google access tokens from a service account's key (`Auth.GoogleToken`,
//! spec 8.4, *Google access tokens*), end to end against a fake server on
//! this machine that is both the token server and the provider
//! (`KAVACH_AI_URL`): no real service is called. The key file is a FAKE
//! one the test writes, given to the child process only, with `--env`.
//! Each test runs with the interpreter and as native code.

mod ai_fake;
#[path = "../../../runtime/net/src/ai/tests/fake_google_key.rs"]
mod fake_google_key;

use ai_fake::{MODES, fake_server, run_forced};
use kavach_runtime_core::base64;

/// A provider for a Google service, written in the program: the runtime
/// gets its token. Its address is never reached: `KAVACH_AI_URL` sends
/// every call to the fake server.
const GOOGLE: &str = "type Yes {
    ok: Bool
}

type Google: Provider {
    fn check(self, model: Model) -> [String] { [] }
    fn key_variable(self) -> String { \"UNUSED_AI_KEY\" }
    fn auth(self, model: Model) -> Auth {
        Auth.GoogleToken(scope: \"https://www.googleapis.com/auth/cloud-platform\")
    }
    fn request(self, call: AskCall) -> HttpCall {
        HttpCall(url: \"https://europe-west4-aiplatform.googleapis.com/v1/projects/shop/locations/europe-west4/publishers/google/models/m1:generateContent\",
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
        HttpCall(url: \"https://europe-west4-aiplatform.googleapis.com/v1/m1:countTokens\", headers: {:}, body: call.prompt)
    }
    fn read_count(self, call: AskCall, reply: HttpReply) -> Int? {
        if reply.status == 200 { 7 } else { none }
    }
    fn estimate(self, call: AskCall) -> Estimate? { none }
}

fn google() -> Model {
    Model(name: \"m1\", provider: Google(), retries: 0)
}

fn main() {
    for _ in 0..2 {
        const y = ask<Yes>(\"Fine?\", model: google()) catch e => {
            print(`failed: {e}`)
            return
        }
        print(y.ok)
    }
}
";

const VARIABLE: &str = "GOOGLE_APPLICATION_CREDENTIALS";
const OK: (&str, &str, &str) = ("200 OK", "", r#"{"ok":true}"#);

/// The token server's reply: `token`, lasting `seconds`.
fn token(token: &str, seconds: i64) -> (&'static str, &'static str, String) {
    let body =
        format!(r#"{{"access_token":"{token}","expires_in":{seconds},"token_type":"Bearer"}}"#);
    ("200 OK", "", body)
}

fn ok() -> (&'static str, &'static str, String) {
    (OK.0, OK.1, OK.2.to_string())
}

/// A fake key file whose token server is the fake server's `/token`, in
/// a folder of its own; its path.
fn key_file(url: &str, name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("kavach-google-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("key.json");
    std::fs::write(&file, fake_google_key::key_file(&format!("{url}/token"))).unwrap();
    file.to_string_lossy().into_owned()
}

/// The value of header `name` in `request`.
fn header<'r>(request: &'r str, name: &str) -> Option<&'r str> {
    let head = &request[..request.find("\r\n\r\n")?];
    head.split("\r\n")
        .find_map(|l| l.strip_prefix(name)?.strip_prefix(": "))
}

/// Checks that `request` asks the fake token server for a token with a
/// JWT signed by the fake key, for the cloud-platform scope.
fn check_token_request(request: &str, url: &str) {
    assert!(request.starts_with("POST /token HTTP/1.1\r\n"), "{request}");
    assert_eq!(
        header(request, "content-type"),
        Some("application/x-www-form-urlencoded")
    );
    let body = &request[request.find("\r\n\r\n").unwrap() + 4..];
    let jwt = body
        .strip_prefix("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=")
        .unwrap_or_else(|| panic!("{body}"));
    let parts: Vec<&str> = jwt.split('.').collect();
    let text = |p: &str| String::from_utf8(base64::decode(p, true).unwrap()).unwrap();
    assert_eq!(
        text(parts[0]),
        r#"{"alg":"RS256","typ":"JWT","kid":"fake-key-id"}"#
    );
    let claims = text(parts[1]);
    let want = format!(
        r#"{{"iss":"{}","scope":"https://www.googleapis.com/auth/cloud-platform","aud":"{url}/token","iat":"#,
        fake_google_key::EMAIL
    );
    assert!(claims.starts_with(&want), "{claims}");
    // `exp` is an hour after `iat`.
    let number = |name: &str| -> i64 {
        let at = claims.find(&format!("\"{name}\":")).unwrap() + name.len() + 3;
        let digits: String = claims[at..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.parse().unwrap()
    };
    assert_eq!(number("exp") - number("iat"), 3600, "{claims}");
    // Signed RS256 by the fake key: checked with its public key.
    let signature = base64::decode(parts[2], true).unwrap();
    let signed = format!("{}.{}", parts[0], parts[1]);
    let n = fake_google_key::modulus();
    assert!(kavach_runtime_net::crypto::rsa_verify(
        &n,
        &[1, 0, 1],
        signed.as_bytes(),
        &signature,
        false
    ));
}

#[test]
fn a_token_is_fetched_with_a_signed_jwt_and_kept() {
    for native in MODES {
        let (url, server) = fake_server(vec![token("ya29.fake-token", 3599), ok(), ok()]);
        let key = key_file(&url, &format!("kept-{native}"));
        let (out, _) = run_forced(GOOGLE, &url, &[(VARIABLE, &key)], native);
        let sent = server.join().unwrap();
        assert_eq!(out, "true\ntrue\n", "native: {native}");
        // One token for both asks.
        assert_eq!(sent.len(), 3, "native: {native}");
        check_token_request(&sent[0], &url);
        for ask in &sent[1..] {
            assert!(
                ask.starts_with("POST /v1/projects/shop/locations/europe-west4/publishers/google/models/m1:generateContent HTTP/1.1"),
                "{ask}"
            );
            assert_eq!(header(ask, "authorization"), Some("Bearer ya29.fake-token"));
        }
    }
}

#[test]
fn a_token_about_to_end_is_fetched_again_from_the_models_variable() {
    let program = GOOGLE.replace(
        "retries: 0)",
        "retries: 0, api_key_env: \"SHOP_GOOGLE_KEY\")",
    );
    for native in MODES {
        let replies = vec![token("ya29.one", 60), ok(), token("ya29.two", 60), ok()];
        let (url, server) = fake_server(replies);
        let key = key_file(&url, &format!("short-{native}"));
        let forced = [
            ("SHOP_GOOGLE_KEY", key.as_str()),
            (VARIABLE, "no-such-file.json"),
        ];
        let (out, _) = run_forced(&program, &url, &forced, native);
        let sent = server.join().unwrap();
        assert_eq!(out, "true\ntrue\n", "native: {native}");
        assert_eq!(sent.len(), 4, "native: {native}");
        check_token_request(&sent[2], &url);
        assert_eq!(header(&sent[1], "authorization"), Some("Bearer ya29.one"));
        assert_eq!(header(&sent[3], "authorization"), Some("Bearer ya29.two"));
    }
}

#[test]
fn no_key_file_is_no_key_or_a_bad_key_and_sends_nothing() {
    for native in MODES {
        let (url, server) = fake_server(vec![]);
        let (out, _) = run_forced(GOOGLE, &url, &[], native);
        assert!(server.join().unwrap().is_empty(), "native: {native}");
        assert_eq!(
            out, "failed: NoKey(variable: \"GOOGLE_APPLICATION_CREDENTIALS\")\n",
            "native: {native}"
        );
        let (url, server) = fake_server(vec![]);
        let missing = std::env::temp_dir().join("kavach-no-such-google-key.json");
        let missing = missing.to_string_lossy().into_owned();
        let (out, _) = run_forced(GOOGLE, &url, &[(VARIABLE, &missing)], native);
        assert!(server.join().unwrap().is_empty(), "native: {native}");
        assert_eq!(
            out,
            "failed: BadKey(message: \"the service account key file that GOOGLE_APPLICATION_CREDENTIALS names is not there (spec 8.4)\")\n",
            "native: {native}"
        );
    }
}

#[test]
fn a_refused_jwt_is_a_bad_key_that_quotes_no_secret() {
    let refused = (
        "400 Bad Request",
        "",
        r#"{"error":"invalid_grant","error_description":"Invalid JWT Signature."}"#.to_string(),
    );
    for native in MODES {
        let (url, server) = fake_server(vec![refused.clone()]);
        let key = key_file(&url, &format!("refused-{native}"));
        let (out, _) = run_forced(GOOGLE, &url, &[(VARIABLE, &key)], native);
        let sent = server.join().unwrap();
        assert_eq!(sent.len(), 1, "native: {native}");
        assert_eq!(
            out,
            "failed: BadKey(message: \"getting a Google access token failed: the token server said 400: invalid_grant: Invalid JWT Signature.\")\n",
            "native: {native}"
        );
    }
}

#[test]
fn a_counting_call_gets_a_token_too() {
    let program = GOOGLE.replace(
        "const y = ask<Yes>(\"Fine?\", model: google()) catch e => {",
        "const y = ai.count(\"Fine?\", model: google()) catch e => {",
    );
    let program = program.replace("print(y.ok)", "print(y.tokens)");
    let program = format!(
        "use std.ai\n\n{}",
        program.replace("type Yes {\n    ok: Bool\n}\n\n", "")
    );
    for native in MODES {
        let replies = vec![token("ya29.count", 3599), ok(), ok()];
        let (url, server) = fake_server(replies);
        let key = key_file(&url, &format!("count-{native}"));
        let (out, _) = run_forced(&program, &url, &[(VARIABLE, &key)], native);
        let sent = server.join().unwrap();
        assert_eq!(out, "7\n7\n", "native: {native}");
        assert_eq!(sent.len(), 3, "native: {native}");
        assert!(
            sent[1].starts_with("POST /v1/m1:countTokens HTTP/1.1"),
            "{}",
            sent[1]
        );
        assert_eq!(header(&sent[2], "authorization"), Some("Bearer ya29.count"));
    }
}
