// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The model call behind `ask<T>` (spec 8.4), shared by the interpreter and
//! native code. The provider, written in Kavach (`std.ai`'s `Provider`,
//! decision 0140), only translates: the program's side calls it through
//! `Side::translate`. Everything else is here: the key, where the call
//! goes, sending, waits and retries, and the gates. Each try is sent on a
//! helper thread; the program's side waits, so other tasks run meanwhile.

pub(crate) mod http;

use kavach_runtime_core::json::{self, Limits, Tree};
use std::time::{Duration, Instant};

pub use wire::Failure;

/// How long one try may take, from connecting to the last byte, when the
/// `Model` doesn't say.
pub const DEADLINE: Duration = Duration::from_secs(120);

/// How many times an answer that doesn't fit `T` is asked for again.
pub const RETRIES: usize = 2;

/// The settings of a `Model` the runtime itself needs, as the compiler
/// works them out (`kavach_driver::model_settings`).
pub struct Request {
    /// The provider's name for the model.
    pub model: String,
    pub max_tokens: i64,
    /// How long one try may take.
    pub timeout: Duration,
    /// How many times a busy, dropped or late call is sent again.
    pub retries: usize,
    /// Where the calls go; `None`: the provider's own, or `KAVACH_AI_URL`.
    pub api_url: Option<String>,
    /// The variable holding the key; `None`: the provider's own.
    pub api_key_env: Option<String>,
    /// The provider's type, for its gate.
    pub provider: String,
}

impl Request {
    pub fn new(settings: &Tree) -> Request {
        let text = |name: &str| match wire::field(settings, name) {
            Some(Tree::Text(s)) => Some(s.clone()),
            _ => None,
        };
        let int = |name: &str| match wire::field(settings, name) {
            Some(Tree::Int(n)) => Some(*n),
            _ => None,
        };
        Request {
            model: text("model").unwrap_or_default(),
            max_tokens: int("max_tokens").unwrap_or(4096),
            timeout: int("timeout_ms")
                .and_then(|n| u64::try_from(n).ok())
                .map_or(DEADLINE, Duration::from_millis),
            retries: int("retries")
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(4),
            api_url: text("api_url"),
            api_key_env: text("api_key_env"),
            provider: text("provider").unwrap_or_else(|| "Anthropic".into()),
        }
    }
}

/// The tokens a call took, as the provider counts them: what went in
/// (the prompt and the schema) and what came out.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
}

/// What the program's side does for an `ask`: the interpreter's, or
/// native code's.
pub trait Side {
    /// A decoded `T`.
    type Value;
    /// Why the program stopped waiting (a panic, or a task cancelled).
    type Stop;
    /// One call of the provider's `wire` function (`std.ai`): JSON text
    /// in, JSON text out.
    fn translate(&mut self, input: &str) -> Result<String, Self::Stop>;
    /// Waits until `until`, letting other tasks run.
    fn wait_until(&mut self, until: Instant) -> Result<(), Self::Stop>;
    /// `answer` as a `T`, when it fits.
    fn fits(&mut self, answer: &Tree) -> Option<Self::Value>;
}

/// What an `ask` gave: its value or error, the tokens every try took,
/// and whether any reply was read.
pub struct Asked<V> {
    pub result: Result<V, Failure>,
    pub tokens: Tokens,
    pub read: bool,
}

/// Asks for a `T` with `prompt`, its JSON Schema `schema` (`wrapped`: the
/// answer is `{"value": …}`), and a model's `settings`.
pub fn ask<S: Side>(
    side: &mut S,
    settings: &Tree,
    prompt: &str,
    schema: &Tree,
    wrapped: bool,
) -> Result<Asked<S::Value>, S::Stop> {
    let req = Request::new(settings);
    let mut asked = Asked {
        result: Err(Failure::Failed(0, String::new())),
        tokens: Tokens::default(),
        read: false,
    };
    let mut last = String::new();
    for attempt in 0..=RETRIES {
        // The type in the prompt first; then held to it (0057).
        let strict = attempt > 0;
        let text = match strict {
            true => prompt.to_string(),
            false => format!("{prompt}\n\n{}", compact::instruction(schema)),
        };
        let reply = match one_try(side, &req, &text, schema, strict)? {
            Ok(r) => r,
            Err(f) => {
                asked.result = Err(f);
                return Ok(asked);
            }
        };
        if let Some(usage) = reply.usage() {
            asked.tokens.input += usage.input;
            asked.tokens.output += usage.output;
            asked.read = true;
        }
        let answer = match reply {
            wire::Reply::Answer(text, _) => text,
            other => {
                asked.result = Err(other.failure(&req));
                return Ok(asked);
            }
        };
        // Text around the JSON, or fences, are left out (decision 0057).
        let tree = compact::json_in(&answer)
            .and_then(|t| json::parse(t, &Limits::default()).ok())
            .unwrap_or(Tree::Null);
        let tree = match wrapped {
            true => wire::field(&tree, "value").cloned().unwrap_or(Tree::Null),
            false => tree,
        };
        if let Some(v) = side.fits(&tree) {
            asked.result = Ok(v);
            return Ok(asked);
        }
        last = json::write(&tree, 0);
    }
    asked.result = Err(Failure::InvalidOutput(last));
    Ok(asked)
}

/// The provider's request for one attempt, sent until it is answered or
/// the trouble is one a wait can't fix; with the reply as the provider
/// reads it.
fn one_try<S: Side>(
    side: &mut S,
    req: &Request,
    prompt: &str,
    schema: &Tree,
    strict: bool,
) -> Result<Result<wire::Reply, Failure>, S::Stop> {
    let input = |op: &str| -> Vec<(String, Tree)> {
        vec![
            ("op".into(), Tree::Text(op.into())),
            ("prompt".into(), Tree::Text(prompt.into())),
            ("schema".into(), schema.clone()),
            ("strict".into(), Tree::Bool(strict)),
        ]
    };
    let asked = json::write(&Tree::Object(input("request")), 0);
    let call = match wire::Call::read(&side.translate(&asked)?) {
        Ok(c) => c,
        Err(f) => return Ok(Err(f)),
    };
    let sending = match send::Sending::new(call, req) {
        Ok(s) => s,
        Err(f) => return Ok(Err(f)),
    };
    let mut k = 0;
    loop {
        let pending = sending.start();
        let mut n = 0;
        let sent = loop {
            if let Some(s) = pending.poll() {
                break s;
            }
            side.wait_until(Instant::now() + crate::net::backoff(n))?;
            n += 1;
        };
        let reply = match sent {
            send::Sent::Reply(status, headers, body) => {
                let mut fields = input("read");
                fields.push(("status".into(), Tree::Int(status)));
                let headers = headers.into_iter().map(|(k, v)| (k, Tree::Text(v)));
                fields.push(("headers".into(), Tree::Object(headers.collect())));
                fields.push(("body".into(), Tree::Text(body)));
                let read = side.translate(&json::write(&Tree::Object(fields), 0))?;
                match wire::Reply::read(&read) {
                    Ok(r) => r,
                    Err(f) => return Ok(Err(f)),
                }
            }
            send::Sent::Failed(kind, message) => wire::Reply::Problem(kind, message, None),
        };
        match reply.wait(k, req) {
            Some(wait) => side.wait_until(Instant::now() + wait)?,
            None => return Ok(Ok(reply)),
        }
        k += 1;
    }
}

mod compact;
mod gate;
mod send;
mod wire;

#[cfg(test)]
mod tests;
