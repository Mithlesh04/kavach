// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `ask<T>(prompt)` and `ai.ask_full<T>(prompt)` in native code (spec
//! 8.4): the runtime's driver
//! (`kavach_runtime_net::ai::ask`), the interpreter's too, with the
//! model's provider, written in Kavach, called through the function
//! `std.ai`'s `wire` gave (decision 0140); the answer decoded as a `T`
//! like `json.decode<T>`. Waiting for the reply lets other tasks run.

use crate::heap::{first_of, release, slots};
use crate::rt::type_of;
use crate::rt_errors::set_error;
use crate::shapes::{find_union, new_text, str_of, with_str};
use kavach_kir::{Shape, Type};
use kavach_runtime_core::json::{self, Tree};
use kavach_runtime_net::ai;
use std::time::Instant;

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

/// Native code's side of an `ask`, or of a count.
pub(crate) struct Asking {
    /// The `fn(String) -> String` closure `wire` gave for each model's
    /// provider, in the order the models are tried.
    pub(crate) wires: Vec<i64>,
    pub(crate) ty: Type,
    pub(crate) templates: i64,
    pub(crate) line: i32,
    /// `ty` is `Asked<T>` (`ai.ask_full`): an answer fits as its `value`.
    pub(crate) full: bool,
}

/// The task was cancelled while it waited.
pub(crate) struct Cancelled;

impl ai::Side for Asking {
    type Value = u64;
    type Stop = Cancelled;

    fn translate(&mut self, model: usize, input: &str) -> Result<String, Cancelled> {
        let wire = self.wires[model];
        let f = slots(wire)[0] as usize;
        // SAFETY: slot 0 of a closure is its function's address; the code
        // generator compiled `wire`'s lambda as `fn(closure, String) ->
        // String`, borrowing the closure and taking the text, and the code
        // lives as long as the program.
        #[allow(unsafe_code)]
        let f: extern "C-unwind" fn(i64, i64) -> i64 = unsafe { std::mem::transmute(f) };
        let given = new_text(input, text_type());
        let out = f(wire, given);
        let text = str_of(out);
        release(out);
        Ok(text)
    }

    fn wait_until(&mut self, until: Instant) -> Result<(), Cancelled> {
        crate::rt_tasks::sleep_until(until, self.line);
        match crate::rt_tasks::cancelling() {
            true => Err(Cancelled),
            false => Ok(()),
        }
    }

    fn fits(&mut self, answer: &Tree) -> Option<u64> {
        let full;
        let answer = match self.full {
            true => {
                full = ai::as_full(answer);
                &full
            }
            false => answer,
        };
        let v = crate::rt_json_decode::decode(answer, self.ty, "", self.templates).ok()?;
        // The value is decoded again with what the `ask` took (`ask`).
        if self.full {
            release(v as i64);
        }
        Some(v)
    }
}

/// `ask` with `pair` (owned: the prompt's text, then the closure `wire`
/// gave for each model), the settings the compiler described in `request`
/// (owned text: each model's, in the order they are tried, the schema,
/// and whether the answer is wrapped in `{"value": …}`) and the decoding
/// templates of `T` (owned). Gives a `T` of type `code`, or 0 with an
/// `AiError` set.
pub fn ask(pair: i64, request: i64, templates: i64, code: i64, line: i32) -> i64 {
    crate::rt_tasks::turn();
    crate::rt_tasks::refuse_if_held(line);
    let prompt = slots(pair)[0] as i64;
    let prompt_text = with_str(prompt, |s| s.to_string());
    let described = with_str(request, |s| json::parse(s, &json::Limits::default()));
    release(request);
    let settings = described.unwrap_or(Tree::Null);
    let field = |name: &str| match &settings {
        Tree::Object(fs) => fs.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone()),
        _ => None,
    };
    let models = match field("models") {
        Some(Tree::Array(ms)) => ms,
        _ => Vec::new(),
    };
    let name = |i: usize| match models.get(i) {
        Some(Tree::Object(fs)) => {
            fs.iter()
                .find(|(k, _)| k == "model")
                .and_then(|(_, v)| match v {
                    Tree::Text(m) => Some(m.clone()),
                    _ => None,
                })
        }
        _ => None,
    };
    let wrapped = matches!(field("wrapped"), Some(Tree::Bool(true)));
    let full = matches!(field("full"), Some(Tree::Bool(true)));
    let schema = field("schema").unwrap_or(Tree::Null);
    let wires = (1..=models.len()).map(|i| slots(pair)[i] as i64).collect();
    let mut side = Asking {
        wires,
        ty: type_of(code),
        templates,
        line,
        full,
    };
    let asked = ai::ask(&mut side, &models, &prompt_text, &schema, wrapped);
    // The pair holds the closures: they go once the asking is done.
    release(pair);
    let Ok(asked) = asked else {
        release(templates);
        return 0;
    };
    // The model of the last try, and each one moved on from (spec 8.6).
    let shown = kavach_runtime_core::agents::Asked {
        model: name(asked.used).unwrap_or_default(),
        tokens: Some((asked.tokens.input, asked.tokens.output)),
        moved: (asked.moved.iter())
            .map(|(i, f)| (name(*i).unwrap_or_default(), f.variant()))
            .collect(),
        usage: asked.usage,
        cost: asked.cost.clone(),
        counted: false,
    };
    let took = i64::try_from(asked.tokens.input + asked.tokens.output).unwrap_or(i64::MAX);
    LAST.with(|l| *l.borrow_mut() = Some((shown, took)));
    let result = match asked.result {
        // With what it took (spec 8.4, *Usage and cost*).
        Ok(_) if full => {
            let whole = ai::full_answer(&asked, &name(asked.used).unwrap_or_default());
            match crate::rt_json_decode::decode(&whole, type_of(code), "", templates) {
                Ok(v) => v as i64,
                Err(_) => crate::rt::panic_with("`Asked` can't hold this answer", line),
            }
        }
        Ok(slot) => slot as i64,
        Err(f) => {
            match find_union("AiError", "NoKey") {
                Some(ty) => match crate::rt_json_decode::decode(&f.tree(), ty, "", templates) {
                    Ok(e) => set_error(e as i64),
                    Err(_) => crate::rt::panic_with("`AiError` can't hold this error", 0),
                },
                None => crate::rt::panic_with("`AiError` is missing from the program", 0),
            }
            0
        }
    };
    release(templates);
    result
}

fn text_type() -> Type {
    first_of(&Shape::Str)
}

// The group's runtime entry (`RtOp::entry`, decision 0024).
crate::rt_data::entry!(kv_rt_ai, |op, a, b, c, code, line| {
    match op {
        // What the run's `ask`s took, or one agent's (`ai.spent`, 8.4).
        kavach_kir::RtOp::AiSpent => {
            let agent = with_str(a, str::to_string);
            release(a);
            let spent = kavach_runtime_core::agents::costs::spent_json(&agent);
            new_text(&spent, type_of(code))
        }
        kavach_kir::RtOp::AiCount => crate::rt_ai_count::count(a, b, c, code, line),
        _ => ask(a, b, c, code, line),
    }
});
