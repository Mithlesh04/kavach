// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A list of models, tried in turn (spec 8.4, *Fallbacks*).

use super::*;

const REFUSED: &str = "HTTP/1.1 451 Refused\r\ncontent-length: 0\r\n\r\n";
const BAD_KEY: &str = "HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\n\r\n";
const BAD_REQUEST: &str = "HTTP/1.1 400 Bad Request\r\ncontent-length: 0\r\n\r\n";

/// Asks with `n` models, each with `retries`.
fn ask_list(fake: &mut Fake, n: usize, retries: i64) -> Asked<Tree> {
    kavach_runtime_core::env_file::set(
        Vec::new(),
        vec![("TEST_FAKE_AI_KEY".into(), "sk-t".into())],
    );
    let schema = Tree::Object(vec![("type".into(), Tree::Text("object".into()))]);
    let models = vec![settings(retries); n];
    let Ok(asked) = ask(fake, &models, "Refund?", &schema, false) else {
        panic!("stopped");
    };
    asked
}

fn fake_with(url: &str) -> Fake {
    let mut f = fake(url);
    f.kinds.push((401, "BadKey"));
    f.kinds.push((400, "BadRequest"));
    f
}

/// Which model each request went to.
fn models_asked(sent: &[String]) -> Vec<String> {
    let model = |r: &String| {
        let line = r.lines().find_map(|l| l.strip_prefix("x-model: "));
        line.unwrap_or("?").trim().to_string()
    };
    sent.iter().map(model).collect()
}

#[test]
fn a_busy_model_hands_the_call_to_the_next_at_once() {
    let (url, server) = fake_server(vec![BUSY, OK]);
    let mut f = fake_with(&url);
    let asked = ask_list(&mut f, 2, 4);
    assert_eq!(models_asked(&server.join().unwrap()), ["0", "1"]);
    assert!(asked.result.is_ok());
    assert!(f.waited.is_empty(), "waited {:?}", f.waited);
    assert_eq!(asked.used, 1);
    assert_eq!(asked.moved, [(0, Failure::RateLimited("no".into()))]);
}

#[test]
fn the_waits_start_when_every_model_is_busy() {
    let (url, server) = fake_server(vec![BUSY, BUSY, OK]);
    let mut f = fake_with(&url);
    let asked = ask_list(&mut f, 2, 1);
    // Then the model whose wait is over first.
    let asked_models = models_asked(&server.join().unwrap());
    assert_eq!(asked_models[..2], ["0", "1"]);
    assert_eq!(asked_models.len(), 3);
    assert!(asked.result.is_ok());
    // It fails now and then under load: say what happened (to-do).
    assert_eq!(
        f.waited.len(),
        1,
        "waited {:?}, moved {:?}, asked {asked_models:?}",
        f.waited,
        asked.moved
    );
    // A line for each change of model: none when the second goes again.
    let changes = if asked_models[2] == "0" { 2 } else { 1 };
    assert_eq!(asked.moved.len(), changes);
}

#[test]
fn trouble_a_wait_cant_fix_moves_on_and_leaves_the_model_out() {
    let (url, server) = fake_server(vec![BROKE, BAD_KEY, BUSY]);
    let mut f = fake_with(&url);
    let asked = ask_list(&mut f, 3, 0);
    assert_eq!(models_asked(&server.join().unwrap()), ["0", "1", "2"]);
    // Every model failed: the last try's error.
    assert_eq!(asked.result, Err(Failure::RateLimited("no".into())));
    let moved: Vec<String> = asked.moved.iter().map(|(_, f)| f.variant()).collect();
    assert_eq!(moved, ["QuotaUsedUp", "BadKey"]);
}

#[test]
fn a_refusal_never_moves_on() {
    let (url, server) = fake_server(vec![REFUSED]);
    let mut f = fake_with(&url);
    let asked = ask_list(&mut f, 2, 4);
    assert_eq!(models_asked(&server.join().unwrap()), ["0"]);
    assert_eq!(asked.result, Err(Failure::Refused(Some("no".into()))));
    assert!(asked.moved.is_empty());
}

#[test]
fn a_bad_request_doesnt_move_on() {
    let (url, server) = fake_server(vec![BAD_REQUEST]);
    let mut f = fake_with(&url);
    let asked = ask_list(&mut f, 2, 4);
    assert_eq!(server.join().unwrap().len(), 1);
    assert_eq!(asked.result, Err(Failure::BadRequest("no".into())));
}

#[test]
fn one_model_is_asked_as_before() {
    let (url, server) = fake_server(vec![BUSY, OK]);
    let mut f = fake_with(&url);
    let asked = ask_list(&mut f, 1, 2);
    assert_eq!(models_asked(&server.join().unwrap()), ["0", "0"]);
    assert!(asked.result.is_ok());
    assert_eq!(f.waited.len(), 1);
    assert!(
        asked.moved.is_empty(),
        "the same model is not moved on from"
    );
}
