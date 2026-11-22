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

use kavach_runtime_core::agents::costs::{self, Price, Spend, Usage};
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
    /// The model's price, as the build worked it out (spec 8.4, *Prices*).
    pub price: Option<Price>,
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
            price: wire::field(settings, "price").and_then(Price::read),
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
    /// One call of the `wire` function (`std.ai`) of the provider of
    /// model `model`, counted in the order the models are tried: JSON
    /// text in, JSON text out.
    fn translate(&mut self, model: usize, input: &str) -> Result<String, Self::Stop>;
    /// Waits until `until`, letting other tasks run.
    fn wait_until(&mut self, until: Instant) -> Result<(), Self::Stop>;
    /// `answer` as a `T`, when it fits.
    fn fits(&mut self, answer: &Tree) -> Option<Self::Value>;
}

/// What an `ask` gave: its value or error, the tokens every try took,
/// whether any reply was read, the model of its last try, and each model
/// it moved on from, with why (spec 8.4, *Fallbacks*). Models are counted
/// in the order they are tried. Also every try's usage, and what they
/// cost at each one's model's price, or why that isn't known (8.4).
pub struct Asked<V> {
    pub result: Result<V, Failure>,
    pub tokens: Tokens,
    pub read: bool,
    pub used: usize,
    pub moved: Vec<(usize, Failure)>,
    pub usage: Usage,
    pub cost: Result<Spend, String>,
    /// The answer that fitted `T`, as JSON.
    pub answer: Tree,
}

/// `ask_once`, then what it took counted for the run (`ai.spent`).
pub fn ask<S: Side>(
    side: &mut S,
    models: &[Tree],
    prompt: &str,
    schema: &Tree,
    wrapped: bool,
) -> Result<Asked<S::Value>, S::Stop> {
    let asked = ask_once(side, models, prompt, schema, wrapped)?;
    if asked.read {
        costs::record(None, &asked.usage, asked.cost.as_ref().ok());
    }
    Ok(asked)
}

/// An answer as the `value` of an `Asked<T>` (`ai.ask_full`, spec 8.4),
/// with nothing taken yet: to see whether it fits.
pub fn as_full(answer: &Tree) -> Tree {
    Tree::Object(vec![
        ("value".into(), answer.clone()),
        ("usage".into(), Usage::default().tree()),
        ("cost".into(), Tree::Null),
        ("model".into(), Tree::Text(String::new())),
    ])
}

/// The `Asked<T>` of an `ask` that got an answer, from model `model`.
pub fn full_answer<V>(asked: &Asked<V>, model: &str) -> Tree {
    let cost = asked.cost.as_ref().map_or(Tree::Null, Spend::tree);
    Tree::Object(vec![
        ("value".into(), asked.answer.clone()),
        ("usage".into(), asked.usage.tree()),
        ("cost".into(), cost),
        ("model".into(), Tree::Text(model.to_string())),
    ])
}

/// One try's `usage`, with model `req`'s price, added to `cost`.
fn count_cost(cost: &mut Result<Spend, String>, req: &Request, usage: &Usage) {
    let Ok(spend) = cost else {
        return;
    };
    let Some(price) = &req.price else {
        *cost = Err(format!("no price for {}", req.model));
        return;
    };
    match price.cost(usage) {
        Some(d) => spend.add(d, price),
        None => *cost = Err("the reply gave no token counts".into()),
    }
}

/// Asks for a `T` with `prompt`, its JSON Schema `schema` (`wrapped`: the
/// answer is `{"value": …}`), and the settings of each model, in the
/// order they are tried (spec 8.4, *Fallbacks*).
fn ask_once<S: Side>(
    side: &mut S,
    models: &[Tree],
    prompt: &str,
    schema: &Tree,
    wrapped: bool,
) -> Result<Asked<S::Value>, S::Stop> {
    let reqs: Vec<Request> = models.iter().map(Request::new).collect();
    let mut asked = Asked {
        result: Err(Failure::Failed(
            0,
            "an `ask` with no model (spec 8.4)".into(),
        )),
        tokens: Tokens::default(),
        read: false,
        used: 0,
        moved: Vec::new(),
        usage: Usage::default(),
        cost: Ok(Spend::default()),
        answer: Tree::Null,
    };
    if reqs.is_empty() {
        return Ok(asked);
    }
    let mut turns = Turns {
        ready: vec![Instant::now(); reqs.len()],
        out: vec![false; reqs.len()],
    };
    let mut last = String::new();
    for attempt in 0..=RETRIES {
        // The type in the prompt first; then held to it (0057).
        let strict = attempt > 0;
        let text = match strict {
            true => prompt.to_string(),
            false => format!("{prompt}\n\n{}", compact::instruction(schema)),
        };
        let tried = Try {
            prompt: &text,
            schema,
            strict,
        };
        let reply = match tried.run(side, &reqs, &mut turns, &mut asked)? {
            Ok(r) => r,
            Err(f) => {
                asked.result = Err(f);
                return Ok(asked);
            }
        };
        if let Some(usage) = reply.usage() {
            asked.tokens.input += usage.input.unwrap_or(0);
            asked.tokens.output += usage.output.unwrap_or(0);
            asked.usage.add(&usage);
            asked.read = true;
            count_cost(&mut asked.cost, &reqs[asked.used], &usage);
        }
        let answer = match reply {
            wire::Reply::Answer(text, _) => text,
            other => {
                asked.result = Err(other.failure(&reqs[asked.used]));
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
            asked.answer = tree;
            return Ok(asked);
        }
        last = json::write(&tree, 0);
    }
    asked.result = Err(Failure::InvalidOutput(last));
    Ok(asked)
}

/// Where each model of an `ask` stands, from one attempt to the next:
/// when it may be sent again, and whether it is left out for the rest of
/// the call (spec 8.4, *Fallbacks*).
struct Turns {
    ready: Vec<Instant>,
    out: Vec<bool>,
}

/// One attempt of an `ask`: its prompt, and whether the answer is held to
/// the schema.
struct Try<'t> {
    prompt: &'t str,
    schema: &'t Tree,
    strict: bool,
}

impl Try<'_> {
    /// The input of one call of a provider's `wire`.
    fn input(&self, op: &str) -> Vec<(String, Tree)> {
        vec![
            ("op".into(), Tree::Text(op.into())),
            ("prompt".into(), Tree::Text(self.prompt.into())),
            ("schema".into(), self.schema.clone()),
            ("strict".into(), Tree::Bool(self.strict)),
        ]
    }

    /// The attempt, sent to the models in turn until one gives a reply
    /// that ends it: an answer, or trouble that doesn't move on. A busy
    /// model hands the call to the next one at once; the waits start only
    /// when every model left is busy. When every model is left out, the
    /// last try's trouble.
    fn run<S: Side>(
        &self,
        side: &mut S,
        reqs: &[Request],
        turns: &mut Turns,
        asked: &mut Asked<S::Value>,
    ) -> Result<Result<wire::Reply, Failure>, S::Stop> {
        let request = json::write(&Tree::Object(self.input("request")), 0);
        let mut sending: Vec<Option<send::Sending>> = reqs.iter().map(|_| None).collect();
        // Each model's busy replies in this attempt, for its waits.
        let mut tries = vec![0usize; reqs.len()];
        // The last try that didn't end the attempt: its model, and why.
        let mut previous: Option<(usize, Failure)> = None;
        let mut now = Instant::now();
        loop {
            let left: Vec<usize> = (0..reqs.len()).filter(|&i| !turns.out[i]).collect();
            let Some(&i) = left.iter().find(|&&i| turns.ready[i] <= now) else {
                match left.iter().map(|&i| turns.ready[i]).min() {
                    Some(t) => {
                        side.wait_until(t)?;
                        now = t.max(Instant::now());
                        continue;
                    }
                    None => {
                        let why = previous.map(|(_, f)| f);
                        return Ok(Err(why.unwrap_or(Failure::Failed(0, String::new()))));
                    }
                }
            };
            if let Some((j, why)) = previous.take()
                && j != i
            {
                asked.moved.push((j, why));
            }
            asked.used = i;
            if sending[i].is_none() {
                let call = match wire::Call::read(&side.translate(i, &request)?) {
                    Ok(c) => c,
                    Err(f) => return Ok(Err(f)),
                };
                match send::Sending::new(call, &reqs[i]) {
                    Ok(s) => sending[i] = Some(s),
                    Err(f) => return Ok(Err(f)),
                }
            }
            let Some(s) = &sending[i] else {
                unreachable!("the request was just made");
            };
            let reply = match self.send(side, i, s)? {
                Ok(r) => r,
                Err(f) => return Ok(Err(f)),
            };
            now = Instant::now();
            match reply.wait(tries[i], &reqs[i]) {
                Some(wait) => {
                    turns.ready[i] = now + wait;
                    tries[i] += 1;
                }
                None if reply.moves_on() => turns.out[i] = true,
                None => return Ok(Ok(reply)),
            }
            previous = Some((i, reply.failure(&reqs[i])));
        }
    }

    /// Sends model `i`'s request once, and reads the reply with its
    /// provider.
    fn send<S: Side>(
        &self,
        side: &mut S,
        i: usize,
        sending: &send::Sending,
    ) -> Result<Result<wire::Reply, Failure>, S::Stop> {
        let pending = sending.start();
        let mut n = 0;
        let sent = loop {
            if let Some(s) = pending.poll() {
                break s;
            }
            side.wait_until(Instant::now() + crate::net::backoff(n))?;
            n += 1;
        };
        Ok(match sent {
            send::Sent::Reply(status, headers, body) => {
                let mut fields = self.input("read");
                fields.push(("status".into(), Tree::Int(status)));
                let headers = headers.into_iter().map(|(k, v)| (k, Tree::Text(v)));
                fields.push(("headers".into(), Tree::Object(headers.collect())));
                fields.push(("body".into(), Tree::Text(body)));
                let read = side.translate(i, &json::write(&Tree::Object(fields), 0))?;
                wire::Reply::read(&read)
            }
            send::Sent::Failed(kind, message) => Ok(wire::Reply::Problem(kind, message, None)),
        })
    }
}

mod compact;
mod gate;
mod send;
mod wire;

#[cfg(test)]
mod tests;
