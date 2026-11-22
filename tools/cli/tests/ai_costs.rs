// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What `ask`s take and cost (spec 8.3, 8.4 *Prices* and *Usage and
//! cost*, 8.6, decision 0140), end to end against a fake server on this
//! machine: no real API is called. Each test runs with the interpreter
//! and as native code, which must agree.

mod ai_fake;

use ai_fake::{MODES, fake_server, run_with};

/// Anthropic's answer `json`, having taken `input` and `output` tokens.
fn took(json: &str, input: u64, output: u64) -> (&'static str, &'static str, String) {
    let quoted = json.replace('"', "\\\"");
    let body = format!(
        "{{\"content\":[{{\"type\":\"text\",\"text\":\"{quoted}\"}}],\"stop_reason\":\"end_turn\",\"usage\":{{\"input_tokens\":{input},\"output_tokens\":{output}}}}}"
    );
    ("200 OK", "", body)
}

/// Runs `program` against `replies`; what it printed, its audit log, and
/// how many requests were sent.
fn costs(
    program: &str,
    replies: Vec<(&'static str, &'static str, String)>,
    native: bool,
) -> (String, String, usize) {
    let (url, server) = fake_server(replies);
    let keys = [("ANTHROPIC_API_KEY", "sk-fake")];
    let (out, log) = run_with(program, &url, &keys, native);
    (out, log, server.join().unwrap().len())
}

/// `ai.ask_full` with the model `MODEL`, printing what it took.
const FULL: &str = "use std.ai

type Yes {
    ok: Bool
}

/// The shop's own price for one model; Anthropic's for the others.
type ShopAi: Provider {
    base: Anthropic

    fn price(self, model: String) -> Price? {
        if model == \"claude-haiku-5-5\" {
            return Price(input: 1.00, output: 2.00, per: 1_000_000,
                         as_of: Date(year: 2026, month: 11, day: 1), from: PriceSource.Program)
        }
        self.base.price(model)
    }
    fn check(self, model: Model) -> [String] { self.base.check(model) }
    fn key_variable(self) -> String { self.base.key_variable() }
    fn auth(self, model: Model) -> Auth { self.base.auth(model) }
    fn request(self, call: AskCall) -> HttpCall { self.base.request(call) }
    fn read(self, call: AskCall, reply: HttpReply) -> Reply { self.base.read(call, reply) }
    fn count_request(self, call: AskCall) -> HttpCall? { self.base.count_request(call) }
    fn read_count(self, call: AskCall, reply: HttpReply) -> Int? { self.base.read_count(call, reply) }
    fn estimate(self, call: AskCall) -> Estimate? { self.base.estimate(call) }
}

fn main() {
    const a = ai.ask_full<Yes>(\"Fine?\", model: MODEL) catch e => {
        print(`failed: {e}`)
        return
    }
    print(a.value.ok)
    print(`{a.usage.input ?? -1} in, {a.usage.output ?? -1} out`)
    print(a.model)
    print(a.cost)
}
";

/// `program` without its `ShopAi` type.
fn cut_shop(program: &str) -> String {
    let start = program.find("/// The shop's own price").unwrap();
    let end = start + program[start..].find("\n}\n").unwrap() + 3;
    format!("{}{}", &program[..start], &program[end..])
}

#[test]
fn ask_full_gives_the_usage_and_the_cost_with_its_note() {
    let cases = [
        // Anthropic's own table.
        (
            "Model(name: \"claude-opus-5-5\")",
            "about $0.003568 (estimate: prices as of 2026-11-22; the provider's bill is exact)",
        ),
        // The program's own price wins.
        (
            "Model(name: \"claude-haiku-5-5\", provider: ShopAi(base: Anthropic()))",
            "about $0.000604 (estimate: the program's prices as of 2026-11-01; the provider's bill is exact)",
        ),
        // A model with no known price costs `none`, never 0.
        ("Model(name: \"claude-opus-4-1\")", "none"),
    ];
    for native in MODES {
        for (model, cost) in cases {
            // The shop's provider only where it is used: an unused type warns.
            let program = match model.contains("ShopAi") {
                true => FULL.replace("MODEL", model),
                false => cut_shop(&FULL.replace("MODEL", model)),
            };
            let (out, _, sent) = costs(&program, vec![took(r#"{"ok":true}"#, 412, 96)], native);
            let name = model.split('"').nth(1).unwrap();
            let want = format!("true\n412 in, 96 out\n{name}\n{cost}\n");
            assert_eq!(out, want, "native: {native}");
            assert_eq!(sent, 1, "native: {native}");
        }
    }
}

/// An agent with `cost <= LIMIT per day` asking `MODEL` twice, then the
/// run's totals and the agent's.
const LIMITED: &str = "use std.ai

type Yes {
    ok: Bool
}

agent Reviewer {
    model Model(name: MODEL)
    settings AgentSettings(audit: Audit(file: \"audit.log\"))
    can Ai
    limit Ai {
        cost <= LIMIT per day
    }
    on check(text: String) -> Bool ! Error {
        const y = try ask<Yes>(text)
        y.ok
    }
}

fn main() uses Ai {
    const bot = Reviewer.start()
    for _ in 0..2 {
        const ok = bot.check(\"Fine?\") catch e => {
            print(`failed: {e}`)
            false
        }
        if ok {
            print(ok)
        }
    }
    const run = ai.spent()
    print(`{run.asks} asks, {run.usage.input ?? 0} in, {run.unpriced} unpriced: {run.cost}`)
    const bot_spent = ai.spent(by: \"Reviewer\")
    print(`Reviewer: {bot_spent.asks} asks`)
}
";

#[test]
fn a_cost_limit_lets_the_crossing_ask_run_then_refuses_and_totals_add_up() {
    let program = LIMITED
        .replace("MODEL", "\"claude-opus-5-5\"")
        .replace("LIMIT", "0.001");
    for native in MODES {
        let (out, log, sent) = costs(&program, vec![took(r#"{"ok":true}"#, 412, 96)], native);
        let want = "true\n\
            failed: Denied(reason: \"limit broken: cost <= 0.001 per day (about $0.003568 so far: the ask that crosses a limit still runs, and costs are estimates)\")\n\
            1 asks, 412 in, 0 unpriced: about $0.003568 (estimate: prices as of 2026-11-22; the provider's bill is exact)\n\
            Reviewer: 1 asks\n";
        assert_eq!(out, want, "native: {native}");
        assert_eq!(sent, 1, "native: {native}");
        assert!(
            log.contains("  model: claude-opus-5-5, tokens: 412 in, 96 out\n  cost: about $0.003568 (estimate: prices as of 2026-11-22; the provider's bill is exact)\n"),
            "native: {native}: {log}"
        );
    }
}

#[test]
fn a_cost_limit_refuses_a_model_with_no_price() {
    let program = LIMITED
        .replace("MODEL", "\"claude-opus-4-1\"")
        .replace("LIMIT", "5.00");
    for native in MODES {
        let (out, _, sent) = costs(&program, Vec::new(), native);
        let refused =
            "failed: Denied(reason: \"no price for claude-opus-4-1: give one, decision 0140\")\n";
        let want = format!("{refused}{refused}0 asks, 0 in, 0 unpriced: none\nReviewer: 0 asks\n");
        assert_eq!(out, want, "native: {native}");
        assert_eq!(sent, 0, "native: {native}");
    }
}

#[test]
fn an_ask_with_no_price_is_counted_apart_in_the_totals() {
    let program = LIMITED
        .replace("MODEL", "\"claude-opus-4-1\"")
        .replace("cost <= LIMIT per day", "count <= 10 per day");
    let replies = vec![
        took(r#"{"ok":true}"#, 412, 96),
        took(r#"{"ok":true}"#, 10, 5),
    ];
    for native in MODES {
        let (out, log, _) = costs(&program, replies.clone(), native);
        let want = "true\ntrue\n2 asks, 422 in, 2 unpriced: none\nReviewer: 2 asks\n";
        assert_eq!(out, want, "native: {native}");
        assert!(
            log.contains("  cost: not known (no price for claude-opus-4-1)\n"),
            "native: {native}: {log}"
        );
    }
}

/// `ai.ask_full` with a list: an answer that isn't a `Yes`, then the
/// first model busy, then the second answering.
const LISTED: &str = "use std.ai

type Yes {
    ok: Bool
}

fn main() {
    const a = ai.ask_full<Yes>(\"Fine?\", model: [Model(name: \"claude-opus-5-5\"), Model(name: \"claude-haiku-5-5\")]) catch e => {
        print(`failed: {e}`)
        return
    }
    print(`{a.usage.input ?? -1} in, {a.usage.output ?? -1} out, from {a.model}`)
    print(a.cost)
}
";

#[test]
fn every_try_counts_at_its_own_models_price() {
    let busy = ("429 Too Many Requests", "retry-after: 5\r\n", String::new());
    for native in MODES {
        let replies = vec![
            took("not a Yes", 412, 96),
            busy.clone(),
            took(r#"{"ok":true}"#, 412, 96),
        ];
        let (out, _, sent) = costs(LISTED, replies, native);
        // 0.003568 at Opus's prices, and 0.0000892 at Haiku's.
        let want = "824 in, 192 out, from claude-haiku-5-5\n\
            about $0.0036572 (estimate: prices as of 2026-11-22; the provider's bill is exact)\n";
        assert_eq!(out, want, "native: {native}");
        assert_eq!(sent, 3, "native: {native}");
    }
}

#[test]
fn an_ask_that_fails_still_counts_what_its_tries_took() {
    let program = LISTED.replace(
        "model: [Model(name: \"claude-opus-5-5\"), Model(name: \"claude-haiku-5-5\")]",
        "model: Model(name: \"claude-opus-5-5\")",
    );
    let program = program.replace(
        "        print(`failed: {e}`)\n        return",
        "        print(`failed: {e}`)\n        print(ai.spent().cost)\n        return",
    );
    for native in MODES {
        // Three answers that aren't a `Yes`: the first try, and two more.
        let replies = vec![
            took("nope", 10, 5),
            took("nope", 10, 5),
            took("nope", 10, 5),
        ];
        let (out, _, sent) = costs(&program, replies, native);
        let want = "failed: InvalidOutput(answer: \"null\")\n\
            about $0.00042 (estimate: prices as of 2026-11-22; the provider's bill is exact)\n";
        assert_eq!(out, want, "native: {native}");
        assert_eq!(sent, 3, "native: {native}");
    }
}
