// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Counting a prompt's input tokens before it is sent (`ai.count`, spec
//! 8.4, decision 0140 choice 10): with the provider's counting call
//! where it has one, sent as an `ask`'s request is (the key, the address,
//! the waits and retries), else with its estimate, marked as one.

use super::wire::{Call, Failure, Reply, field};
use super::{Request, Side, send};
use kavach_runtime_core::json::{self, Limits, Tree};
use std::time::Instant;

/// A prompt's tokens, as `std.ai`'s `Count` holds them.
#[derive(Clone, Debug, PartialEq)]
pub struct Counted {
    pub tokens: i64,
    /// The provider counted them; else an estimate made offline.
    pub exact: bool,
    /// How far to trust it.
    pub note: String,
}

impl Counted {
    /// The count of a dry run or a generated test: nothing was counted.
    pub fn dry() -> Counted {
        Counted {
            tokens: 0,
            exact: false,
            note: "dry run: nothing was counted".into(),
        }
    }

    /// The count as `Count` prints it: `412 tokens (counted by the
    /// provider)`, or `about 103 tokens (estimate: …)`.
    pub fn shown(&self) -> String {
        let about = if self.exact { "" } else { "about " };
        format!("{about}{} tokens ({})", self.tokens, self.note)
    }

    /// The `Count` value as JSON.
    pub fn tree(&self) -> Tree {
        Tree::Object(vec![
            ("tokens".into(), Tree::Int(self.tokens)),
            ("exact".into(), Tree::Bool(self.exact)),
            ("note".into(), Tree::Text(self.note.clone())),
        ])
    }
}

/// The input of one call of the provider's `wire`.
fn input(op: &str, prompt: &str) -> Vec<(String, Tree)> {
    vec![
        ("op".into(), Tree::Text(op.into())),
        ("prompt".into(), Tree::Text(prompt.into())),
        ("schema".into(), Tree::Null),
        ("strict".into(), Tree::Bool(false)),
    ]
}

fn translate<S: Side>(side: &mut S, fields: Vec<(String, Tree)>) -> Result<String, S::Stop> {
    side.translate(0, &json::write(&Tree::Object(fields), 0))
}

fn parse(text: &str) -> Tree {
    json::parse(text, &Limits::default()).unwrap_or(Tree::Null)
}

/// Counts `prompt`'s tokens for the model whose settings are `model`,
/// through its provider (`Side::translate` with model 0).
pub fn count<S: Side>(
    side: &mut S,
    model: &Tree,
    prompt: &str,
) -> Result<Result<Counted, Failure>, S::Stop> {
    let req = Request::new(model);
    let out = translate(side, input("count_request", prompt))?;
    if matches!(parse(&out), Tree::Null) {
        return estimate(side, &req, prompt);
    }
    let call = match Call::read(&out) {
        Ok(c) => c,
        Err(f) => return Ok(Err(f)),
    };
    let sending = match send::Sending::new(call, &req) {
        Ok(s) => s.counting(),
        Err(f) => return Ok(Err(f)),
    };
    let mut tries = 0;
    loop {
        let reply = match super::sent(side, &sending)? {
            send::Sent::Reply(status, headers, body) => {
                let mut fields = input("read_count", prompt);
                fields.push(("status".into(), Tree::Int(status)));
                let headers = headers.into_iter().map(|(k, v)| (k, Tree::Text(v)));
                fields.push(("headers".into(), Tree::Object(headers.collect())));
                fields.push(("body".into(), Tree::Text(body)));
                if let Tree::Int(n) = parse(&translate(side, fields.clone())?) {
                    return Ok(Ok(Counted {
                        tokens: n,
                        exact: true,
                        note: "counted by the provider".into(),
                    }));
                }
                // No count: the provider's `read` says what went wrong.
                fields[0].1 = Tree::Text("read".into());
                match Reply::read(&translate(side, fields)?) {
                    Ok(r @ Reply::Problem(..)) => r,
                    Ok(_) => {
                        let why = "the counting call's reply had no count (spec 8.4)";
                        return Ok(Err(Failure::Failed(status, why.into())));
                    }
                    Err(f) => return Ok(Err(f)),
                }
            }
            send::Sent::Failed(kind, message) => Reply::Problem(kind, message, None),
        };
        match reply.wait(tries, &req) {
            Some(wait) => {
                side.wait_until(Instant::now() + wait)?;
                tries += 1;
            }
            None => return Ok(Err(reply.failure(&req))),
        }
    }
}

/// The provider has no counting call: its estimate, or why there is none.
fn estimate<S: Side>(
    side: &mut S,
    req: &Request,
    prompt: &str,
) -> Result<Result<Counted, Failure>, S::Stop> {
    let made = parse(&translate(side, input("estimate", prompt))?);
    Ok(match (field(&made, "tokens"), field(&made, "how")) {
        (Some(Tree::Int(n)), how) => {
            let how = match how {
                Some(Tree::Text(h)) => h.as_str(),
                _ => "",
            };
            Ok(Counted {
                tokens: *n,
                exact: false,
                note: format!("estimate: {how}"),
            })
        }
        _ => Err(Failure::Failed(
            0,
            format!(
                "{} can't count tokens: it has no counting call and no estimate (spec 8.4)",
                req.provider
            ),
        )),
    })
}
