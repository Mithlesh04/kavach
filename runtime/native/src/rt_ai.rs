//! `ask<T>(prompt)` in native code (spec 8.4): the model call of
//! `kavach_runtime_net::ai`, the interpreter's too, then the answer
//! decoded as a `T` like `json.decode<T>`, and asked again (twice at most)
//! when it doesn't fit. Waiting for the reply lets other tasks run.

use crate::heap::{first_of, release};
use crate::rt::type_of;
use crate::rt_errors::set_error;
use crate::rt_json::new_variant;
use crate::shapes::{find_union, new_text, with_str};
use kavach_kir::{Shape, Type};
use kavach_runtime_core::json::{self, Tree};
use kavach_runtime_net::ai::{self, Outcome, Request};
use std::time::Instant;

/// How many times an answer that doesn't fit `T` is asked for again.
const RETRIES: usize = 2;

thread_local! {
    /// The model of this thread's last `ask` and the tokens it took, for
    /// its line in the agent's audit log (spec 8.6).
    static LAST: std::cell::RefCell<Option<(kavach_runtime_core::agents::Asked, i64)>> = const { std::cell::RefCell::new(None) };
}

/// The model and tokens of this thread's last `ask`, as the audit log
/// shows them and as a count, once.
pub(crate) fn take_last() -> Option<(kavach_runtime_core::agents::Asked, i64)> {
    LAST.with(|l| l.borrow_mut().take())
}

/// `ask` with `prompt` (owned text), the request the compiler described
/// in `request` (owned text: the model, its token limit, the schema, and
/// whether the answer is wrapped in `{"value": …}`) and the decoding
/// templates of `T` (owned). Gives a `T` of type `code`, or 0 with an
/// `AiError` set.
pub fn ask(prompt: i64, request: i64, templates: i64, code: i64, line: i32) -> i64 {
    crate::rt_tasks::turn();
    crate::rt_tasks::refuse_if_held(line);
    let prompt_text = with_str(prompt, |s| s.to_string());
    let described = with_str(request, |s| json::parse(s, &json::Limits::default()));
    release(prompt);
    release(request);
    let field = |name: &str| match &described {
        Ok(Tree::Object(fs)) => fs.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone()),
        _ => None,
    };
    let model = match field("model") {
        Some(Tree::Text(m)) => m,
        _ => String::new(),
    };
    let wrapped = matches!(field("wrapped"), Some(Tree::Bool(true)));
    let mut used = ai::Tokens::default();
    let schema = field("schema").unwrap_or(Tree::Null);
    let ty = type_of(code);
    let mut last = String::new();
    for attempt in 0..=RETRIES {
        // The type in the prompt first; then held to it (0057).
        let settings = described.as_ref().unwrap_or(&Tree::Null);
        let request = Request::new(settings, prompt_text.clone(), schema.clone(), attempt > 0);
        let pending = ai::start(request);
        let mut n = 0;
        let outcome = loop {
            if let Some(o) = pending.poll() {
                break o;
            }
            let until = Instant::now() + kavach_runtime_net::net::backoff(n);
            crate::rt_tasks::sleep_until(until, line);
            if crate::rt_tasks::cancelling() {
                release(templates);
                return 0;
            }
            n += 1;
        };
        let answer = match outcome {
            Outcome::NoKey(name) => {
                let variable = new_text(&name, text_type());
                return fail(templates, "NoKey", &[variable as u64]);
            }
            Outcome::Failed { status, message } => {
                let message = new_text(&message, text_type());
                return fail(templates, "Failed", &[status as u64, message as u64]);
            }
            Outcome::Answer(tree, tokens) => {
                used.input += tokens.input;
                used.output += tokens.output;
                let shown = kavach_runtime_core::agents::Asked {
                    model: model.clone(),
                    tokens: Some((used.input, used.output)),
                };
                let took = i64::try_from(used.input + used.output).unwrap_or(i64::MAX);
                LAST.with(|l| *l.borrow_mut() = Some((shown, took)));
                tree
            }
        };
        let answer = match (wrapped, &answer) {
            (true, Tree::Object(fs)) => fs
                .iter()
                .find(|(k, _)| k == "value")
                .map_or(Tree::Null, |(_, v)| v.clone()),
            _ => answer,
        };
        if let Ok(slot) = crate::rt_json_decode::decode(&answer, ty, "", templates) {
            release(templates);
            return slot as i64;
        }
        last = json::write(&answer, 0);
    }
    let answer = new_text(&last, text_type());
    fail(templates, "InvalidOutput", &[answer as u64])
}

fn text_type() -> Type {
    first_of(&Shape::Str)
}

/// Sets the error `AiError.<name>(fields)` and gives 0.
fn fail(templates: i64, name: &str, fields: &[u64]) -> i64 {
    release(templates);
    match find_union("AiError", "NoKey") {
        Some(ty) => set_error(new_variant(ty, name, fields)),
        None => crate::rt::panic_with("`AiError` is missing from the program", 0),
    }
    0
}

// The group's runtime entry (`RtOp::entry`, decision 0024).
crate::rt_data::entry!(kv_rt_ai, |op, a, b, c, code, line| {
    let _ = op;
    ask(a, b, c, code, line)
});
