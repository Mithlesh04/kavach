//! The model call behind `ask<T>` (spec 8.4), shared by the interpreter and
//! native code: one request to Anthropic's Messages API with structured
//! outputs (`output_config.format`), so the reply's text is JSON in the
//! shape of a schema built from `T`. Forced tool calls are not used: newer
//! models refuse them. It runs on a helper thread; the caller polls, so
//! other tasks run meanwhile. `KAVACH_AI_URL` sends it elsewhere (tests).

pub(crate) mod http;

use kavach_runtime_core::json::{self, Limits, Tree};
use std::sync::mpsc;
use std::time::Duration;

/// The variable the key is read from.
pub const KEY_VARIABLE: &str = "ANTHROPIC_API_KEY";

/// How long one call may take, from connecting to the last byte.
pub const DEADLINE: Duration = Duration::from_secs(120);

/// One question for the model.
pub struct Request {
    pub model: String,
    pub max_tokens: u32,
    /// `low` to `max`; `None`: the model's own.
    pub effort: Option<String>,
    /// `Some(false)`: answer without thinking first (spec 8.4).
    pub thinking: Option<bool>,
    /// The system prompt.
    pub instructions: Option<String>,
    pub stop: Vec<String>,
    /// How long one try may take.
    pub timeout: Duration,
    /// How many times a busy, dropped or late call is sent again.
    pub retries: usize,
    /// Where the call goes; `None`: `KAVACH_AI_URL`, or Anthropic's.
    pub api_url: Option<String>,
    /// The variable holding the key; `None`: `ANTHROPIC_API_KEY`.
    pub api_key_env: Option<String>,
    /// More request fields, added as they are.
    pub extra: Vec<(String, Tree)>,
    /// Held to the schema by the provider's structured output; otherwise
    /// the schema's short form is written into the prompt (decision 0057).
    pub strict: bool,
    pub prompt: String,
    /// A JSON Schema for an object: the shape the answer must have.
    pub schema: Tree,
}

impl Request {
    /// The request for `prompt` and `schema` with a model's `settings`, as
    /// the compiler works them out (`kavach_driver::model_settings`).
    pub fn new(settings: &Tree, prompt: String, schema: Tree, strict: bool) -> Request {
        let field = |name: &str| match settings {
            Tree::Object(fs) => fs.iter().find(|(k, _)| k == name).map(|(_, v)| v),
            _ => None,
        };
        let text = |name: &str| match field(name) {
            Some(Tree::Text(s)) => Some(s.clone()),
            _ => None,
        };
        let int = |name: &str| match field(name) {
            Some(Tree::Int(n)) => Some(*n),
            _ => None,
        };
        let stop = match field("stop") {
            Some(Tree::Array(items)) => (items.iter())
                .filter_map(|t| match t {
                    Tree::Text(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        // `extra` is checked to look like an object when the program is
        // built; anything that doesn't parse adds nothing.
        let extra = match text("extra").map(|x| json::parse(&x, &Limits::default())) {
            Some(Ok(Tree::Object(fs))) => fs,
            _ => Vec::new(),
        };
        Request {
            model: text("model").unwrap_or_default(),
            max_tokens: int("max_tokens")
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or(4096),
            effort: text("effort"),
            thinking: match field("thinking") {
                Some(Tree::Bool(b)) => Some(*b),
                _ => None,
            },
            instructions: text("instructions"),
            stop,
            timeout: int("timeout_ms")
                .and_then(|n| u64::try_from(n).ok())
                .map_or(DEADLINE, Duration::from_millis),
            retries: int("retries")
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(4),
            api_url: text("api_url"),
            api_key_env: text("api_key_env"),
            extra,
            strict,
            prompt,
            schema,
        }
    }
}

/// What a call gave.
#[derive(Debug)]
pub enum Outcome {
    /// The reply's JSON, in the requested shape if the model kept to it;
    /// still to check; and the tokens it took.
    Answer(Tree, Tokens),
    /// The provider refused, or the network failed (status 0).
    Failed { status: i64, message: String },
    /// The key's variable, named here, is not set.
    NoKey(String),
}

/// The tokens a call took, as the provider counts them: what went in
/// (the prompt and the schema) and what came out.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
}

/// A call in progress on a helper thread.
pub struct Pending {
    done: mpsc::Receiver<Outcome>,
}

impl Pending {
    /// One look at whether the call has finished.
    pub fn poll(&self) -> Option<Outcome> {
        match self.done.try_recv() {
            Ok(o) => Some(o),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Outcome::Failed {
                status: 0,
                message: "the call stopped".into(),
            }),
        }
    }
}

/// Starts a call.
pub fn start(req: Request) -> Pending {
    let (send, done) = mpsc::channel();
    let variable = req.api_key_env.clone().unwrap_or(KEY_VARIABLE.to_string());
    let Ok(key) = std::env::var(&variable) else {
        let _ = send.send(Outcome::NoKey(variable));
        return Pending { done };
    };
    let base = match &req.api_url {
        Some(url) => url.clone(),
        None => {
            std::env::var("KAVACH_AI_URL").unwrap_or_else(|_| "https://api.anthropic.com".into())
        }
    };
    std::thread::spawn(move || {
        let mut outcome = gated(&base, key.trim(), &req);
        for k in 0..req.retries {
            if !busy(&outcome) {
                break;
            }
            std::thread::sleep(gate::wait_before(k));
            outcome = gated(&base, key.trim(), &req);
        }
        let _ = send.send(outcome);
    });
    Pending { done }
}

/// One try, through the program's gate (spec 8.4).
fn gated(base: &str, key: &str, req: &Request) -> Outcome {
    gate::GATE.enter();
    let outcome = call(base, key, req);
    let turned_away = matches!(outcome, Outcome::Failed { status: 429, .. });
    gate::GATE.leave(turned_away, matches!(outcome, Outcome::Answer(..)));
    outcome
}

/// Whether the provider turned the call away because it was busy, or the
/// network dropped it: then it is worth sending again.
fn busy(o: &Outcome) -> bool {
    matches!(o, Outcome::Failed { status, .. } if *status == 0 || *status == 429 || (500..600).contains(status))
}

/// The request's body: the prompt, and the shape the answer must have.
fn body(req: &Request) -> String {
    let text = |s: &str| Tree::Text(s.to_string());
    let prompt = match req.strict {
        true => req.prompt.clone(),
        false => format!("{}\n\n{}", req.prompt, compact::instruction(&req.schema)),
    };
    let message = Tree::Object(vec![
        ("role".into(), text("user")),
        ("content".into(), text(&prompt)),
    ]);
    let mut fields = vec![
        ("model".into(), text(&req.model)),
        ("max_tokens".into(), Tree::Int(i64::from(req.max_tokens))),
        ("messages".into(), Tree::Array(vec![message])),
    ];
    if req.strict {
        let format = Tree::Object(vec![
            ("type".into(), text("json_schema")),
            ("schema".into(), req.schema.clone()),
        ]);
        fields.push((
            "output_config".into(),
            Tree::Object(vec![("format".into(), format)]),
        ));
    }
    if let Some(e) = &req.effort {
        // Beside the format, when the call is held to the schema.
        let effort = ("effort".to_string(), text(e));
        match fields.iter_mut().find(|(k, _)| k == "output_config") {
            Some((_, Tree::Object(config))) => config.push(effort),
            _ => fields.push(("output_config".into(), Tree::Object(vec![effort]))),
        }
    }
    if req.thinking == Some(false) {
        // Each model turns thinking off its own way: Sonnet 5.5 takes
        // `between_tools`, the others `disabled`, as their errors say
        // (Opus 5.5 can't: the compiler refuses it, spec 8.4).
        let off = if req.model.contains("sonnet-5-5") {
            "between_tools"
        } else {
            "disabled"
        };
        fields.push((
            "thinking".into(),
            Tree::Object(vec![("type".into(), text(off))]),
        ));
    }
    if let Some(s) = &req.instructions {
        fields.push(("system".into(), text(s)));
    }
    if !req.stop.is_empty() {
        let stop = req.stop.iter().map(|s| text(s)).collect();
        fields.push(("stop_sequences".into(), Tree::Array(stop)));
    }
    // A provider setting Kavach doesn't name: written as it is, in place
    // of one Kavach would send.
    for (k, v) in &req.extra {
        fields.retain(|(name, _)| name != k);
        fields.push((k.clone(), v.clone()));
    }
    json::write(&Tree::Object(fields), 0)
}

fn call(base: &str, key: &str, req: &Request) -> Outcome {
    let headers = [
        ("x-api-key", key),
        ("anthropic-version", "2023-06-01"),
        ("content-type", "application/json"),
    ];
    let url = format!("{}/v1/messages", base.trim_end_matches('/'));
    let (status, reply) = match http::post(&url, &headers, body(req).as_bytes(), req.timeout) {
        Ok(r) => r,
        Err(message) => return Outcome::Failed { status: 0, message },
    };
    let text = String::from_utf8_lossy(&reply);
    let tree = json::parse(&text, &Limits::default()).unwrap_or(Tree::Null);
    if status != 200 {
        return Outcome::Failed {
            status,
            message: error_message(&tree).unwrap_or_else(|| text.chars().take(200).collect()),
        };
    }
    // A refusal, or an answer cut off at `max_tokens`, won't change when
    // asked again, so neither is retried.
    match field(&tree, "stop_reason") {
        Some(Tree::Text(r)) if r == "refusal" => {
            return Outcome::Failed {
                status,
                message: "the model refused to answer".into(),
            };
        }
        Some(Tree::Text(r)) if r == "max_tokens" => {
            return Outcome::Failed {
                status,
                message: format!(
                    "the answer was cut off at max_tokens ({}): give its `Model` a larger `max_tokens` (spec 8.4)",
                    req.max_tokens
                ),
            };
        }
        _ => {}
    }
    // Text around the JSON, or fences, are left out (decision 0057).
    let answer = answer_text(&tree)
        .and_then(|t| json::parse(compact::json_in(&t)?, &Limits::default()).ok());
    let count = |name: &str| match field(field(&tree, "usage").unwrap_or(&Tree::Null), name) {
        Some(Tree::Int(n)) => u64::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let tokens = Tokens {
        input: count("input_tokens"),
        output: count("output_tokens"),
    };
    Outcome::Answer(answer.unwrap_or(Tree::Null), tokens)
}

fn field<'t>(tree: &'t Tree, name: &str) -> Option<&'t Tree> {
    match tree {
        Tree::Object(fields) => fields.iter().find(|(k, _)| k == name).map(|(_, v)| v),
        _ => None,
    }
}

/// `{"error": {"message": …}}`, as the API reports errors.
fn error_message(tree: &Tree) -> Option<String> {
    match field(field(tree, "error")?, "message")? {
        Tree::Text(s) => Some(s.clone()),
        _ => None,
    }
}

/// The first `text` block of the reply's `content` (reasoning blocks may
/// come before it).
fn answer_text(tree: &Tree) -> Option<String> {
    let Tree::Array(blocks) = field(tree, "content")? else {
        return None;
    };
    let block = blocks
        .iter()
        .find(|b| matches!(field(b, "type"), Some(Tree::Text(t)) if t == "text"))?;
    match field(block, "text")? {
        Tree::Text(t) => Some(t.clone()),
        _ => None,
    }
}

mod compact;
mod gate;

#[cfg(test)]
mod tests;
