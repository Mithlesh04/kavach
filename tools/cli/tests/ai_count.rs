// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Counting a prompt's tokens before it is sent (`ai.count`, spec 8.3,
//! 8.4 *Counting tokens before sending*, 8.6, decision 0140 choice 10),
//! end to end against a fake server on this machine: no real API is
//! called. Each test runs with the interpreter and as native code, which
//! must agree.

mod ai_fake;

use ai_fake::{MODES, error, fake_server, run_with};

/// Anthropic's count: `{"input_tokens": n}`.
fn counted(n: u64) -> (&'static str, &'static str, String) {
    ("200 OK", "", format!("{{\"input_tokens\":{n}}}"))
}

/// Runs `program` against `replies` with `keys`; what it printed, its
/// audit log, and each request it sent.
fn count(
    program: &str,
    replies: Vec<(&'static str, &'static str, String)>,
    keys: &[(&str, &str)],
    native: bool,
) -> (String, String, Vec<String>) {
    let (url, server) = fake_server(replies);
    let (out, log) = run_with(program, &url, keys, native);
    (out, log, server.join().unwrap())
}

const KEY: [(&str, &str); 1] = [("ANTHROPIC_API_KEY", "sk-fake")];

/// Counts `Refund order 7?` with model `MODEL`, printing the count or
/// the error.
const COUNT: &str = "use std.ai

/// Anthropic, without its counting call.
type Offline: Provider {
    base: Anthropic

    fn count_request(self, call: AskCall) -> HttpCall? { none }
    fn estimate(self, call: AskCall) -> Estimate? { ESTIMATE }
    fn check(self, model: Model) -> [String] { self.base.check(model) }
    fn key_variable(self) -> String { self.base.key_variable() }
    fn auth(self, model: Model) -> Auth { self.base.auth(model) }
    fn request(self, call: AskCall) -> HttpCall { self.base.request(call) }
    fn read(self, call: AskCall, reply: HttpReply) -> Reply { self.base.read(call, reply) }
    fn price(self, model: String) -> Price? { self.base.price(model) }
    fn read_count(self, call: AskCall, reply: HttpReply) -> Int? { self.base.read_count(call, reply) }
}

fn main() {
    const n = ai.count(\"Refund order 7?\", model: MODEL) catch e => {
        print(`failed: {e}`)
        return
    }
    print(n)
    print(`{n.tokens} {n.exact}`)
}
";

/// `COUNT` with `model`, and `Offline`'s `estimate` giving `estimate`.
fn program(model: &str, estimate: &str) -> String {
    let p = COUNT.replace("MODEL", model).replace("ESTIMATE", estimate);
    match model.contains("Offline") {
        true => p,
        // An unused type warns: it goes when not used.
        false => {
            let start = p.find("/// Anthropic, without").unwrap();
            let end = start + p[start..].find("\n}\n").unwrap() + 3;
            format!("{}{}", &p[..start], &p[end..])
        }
    }
}

#[test]
fn a_count_is_the_providers_own_from_its_counting_call() {
    let model = "Model(name: \"claude-opus-5-5\", instructions: \"Be brief.\", max_tokens: 300, stop: [\"END\"])";
    for native in MODES {
        let (out, _, sent) = count(&program(model, ""), vec![counted(14)], &KEY, native);
        assert_eq!(
            out, "14 tokens (counted by the provider)\n14 true\n",
            "native: {native}"
        );
        assert_eq!(sent.len(), 1, "native: {native}");
        let request = &sent[0];
        assert!(
            request.starts_with("POST /v1/messages/count_tokens "),
            "{request}"
        );
        assert!(request.contains("x-api-key: sk-fake"), "{request}");
        let body = &request[request.find("\r\n\r\n").unwrap() + 4..];
        assert_eq!(
            body,
            r#"{"model":"claude-opus-5-5","messages":[{"role":"user","content":"Refund order 7?"}],"system":"Be brief."}"#,
            "native: {native}"
        );
    }
}

#[test]
fn a_provider_without_a_counting_call_gives_its_estimate() {
    let model = "Model(name: \"claude-haiku-5-5\", provider: Offline(base: Anthropic()))";
    let cases = [
        (
            "self.base.estimate(call)",
            "about 4 tokens (estimate: about 4 characters a token)\n4 false\n",
        ),
        (
            "none",
            "failed: Failed(status: 0, message: \"Offline can't count tokens: it has no counting call and no estimate (spec 8.4)\")\n",
        ),
    ];
    for native in MODES {
        for (estimate, want) in cases {
            let (out, _, sent) = count(&program(model, estimate), vec![], &KEY, native);
            assert_eq!(out, want, "native: {native}");
            // Offline: nothing is sent.
            assert!(sent.is_empty(), "native: {native}");
        }
    }
}

#[test]
fn a_counting_call_that_fails_fails_the_count() {
    let once = "Model(name: \"claude-haiku-5-5\", retries: 0)";
    let again = "Model(name: \"claude-haiku-5-5\", retries: 1)";
    let busy = || error("429 Too Many Requests", "rate_limit_error", "slow down");
    for native in MODES {
        let cases = [
            (
                once,
                vec![busy()],
                "failed: RateLimited(message: \"slow down\")\n",
                1,
            ),
            (
                once,
                vec![error("401 Unauthorized", "authentication_error", "bad key")],
                "failed: BadKey(message: \"bad key\")\n",
                1,
            ),
            (
                once,
                vec![("200 OK", "", "{}".to_string())],
                "failed: Failed(status: 200, message: \"the counting call's reply had no count (spec 8.4)\")\n",
                1,
            ),
            // Busy, then counted: sent again after a wait, as an `ask`.
            (
                again,
                vec![busy(), counted(9)],
                "9 tokens (counted by the provider)\n9 true\n",
                2,
            ),
        ];
        for (model, replies, want, n) in cases {
            let (out, _, sent) = count(&program(model, ""), replies, &KEY, native);
            assert_eq!(out, want, "native: {native}");
            assert_eq!(sent.len(), n, "native: {native}");
        }
        // The key's variable isn't set: nothing is sent.
        let (out, _, sent) = count(&program(once, ""), vec![], &[], native);
        assert_eq!(
            out, "failed: NoKey(variable: \"ANTHROPIC_API_KEY\")\n",
            "native: {native}"
        );
        assert!(sent.is_empty(), "native: {native}");
    }
}

/// An agent whose limits a count would break if it counted as an `ask`.
const AGENT: &str = "use std.ai

type Yes {
    ok: Bool
}

agent Sizer {
    model Model(name: \"claude-opus-5-5\")
    settings AgentSettings(audit: Audit(file: \"audit.log\"))
    can Ai
    limit Ai {
        count <= 1 per day
        tokens <= 100 per day
    }
    on size(text: String) -> Int ! Error {
        const a = try ai.count(text)
        const b = try ai.count(text)
        const y = try ask<Yes>(text)
        if y.ok { a.tokens + b.tokens } else { 0 }
    }
}

fn main() uses Ai {
    const bot = Sizer.start()
    const n = bot.size(\"Fine?\") catch e => {
        print(`failed: {e}`)
        return
    }
    print(n)
    const s = ai.spent(by: \"Sizer\")
    print(`{s.asks} asks, {s.usage.input ?? 0} in`)
}
";

#[test]
fn a_count_is_in_the_audit_log_and_no_limit_counts_it() {
    let answer = (
        "200 OK",
        "",
        "{\"content\":[{\"type\":\"text\",\"text\":\"{\\\"ok\\\":true}\"}],\"stop_reason\":\"end_turn\",\"usage\":{\"input_tokens\":60,\"output_tokens\":10}}".to_string(),
    );
    for native in MODES {
        let replies = vec![counted(5000), counted(5000), answer.clone()];
        let (out, log, sent) = count(AGENT, replies, &KEY, native);
        // Two counts of 5,000 tokens, and the ask still runs: neither
        // `count` nor `tokens` counted them; `ai.spent` has the ask only.
        assert_eq!(out, "10000\n1 asks, 60 in\n", "native: {native}");
        assert_eq!(sent.len(), 3, "native: {native}");
        let entries: Vec<&str> = log.split("\n20").collect();
        assert_eq!(entries.len(), 3, "{log}");
        for entry in &entries[..2] {
            let lines: Vec<&str> = entry.lines().collect();
            assert!(
                lines[0].ends_with("Sizer request#1 ai.count(\"Fine?\")"),
                "{log}"
            );
            assert_eq!(
                &lines[1..],
                [
                    "  verdict: allowed",
                    "  model: claude-opus-5-5",
                    "  result: ok 5000 tokens (counted by the provider)",
                ],
                "native: {native}\n{log}"
            );
        }
        assert!(entries[2].contains("ask(\"Fine?\")"), "{log}");
    }
}
