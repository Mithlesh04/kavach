// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `ai.count(prompt)` in native code (spec 8.4, *Counting tokens before
//! sending*): the runtime's counting (`kavach_runtime_net::ai::count`),
//! the interpreter's too, with the model of the running agent, or the
//! call's own, or the default. In an agent it has an audit entry,
//! `Ai.count` (spec 8.6); it is not an `ask`, so no limit counts it.

use crate::display::display;
use crate::heap::{release, slots};
use crate::rt::type_of;
use crate::rt_agents::{RUNNING, with_agent};
use crate::rt_ai::Asking;
use crate::rt_errors::set_error;
use crate::shapes::{find_union, with_str};
use kavach_runtime_core::agents::Asked;
use kavach_runtime_core::json::{self, Tree};
use kavach_runtime_net::ai;

/// Field `name` of an object.
fn field<'t>(tree: &'t Tree, name: &str) -> Option<&'t Tree> {
    match tree {
        Tree::Object(fs) => fs.iter().find(|(k, _)| k == name).map(|(_, v)| v),
        _ => None,
    }
}

/// Fails with `AiError` `error`, as JSON, decoded with `templates`;
/// gives the error as the audit log shows it.
fn fail_with(error: &Tree, templates: i64) -> String {
    let Some(ty) = find_union("AiError", "NoKey") else {
        crate::rt::panic_with("`AiError` is missing from the program", 0);
    };
    match crate::rt_json_decode::decode(error, ty, "", templates) {
        Ok(e) => {
            let shown = display(e as i64, ty, true);
            set_error(e as i64);
            format!("error {shown}")
        }
        Err(_) => crate::rt::panic_with("`AiError` can't hold this error", 0),
    }
}

/// `ai.count` with `pair` (owned: the prompt's text, then the closure
/// `wire` gave for each model), `request` (owned text: each model's
/// settings, and the kind of agent it is for, -1 for none) and the
/// decoding templates of `Count` (owned). Gives a `Count` of type `code`,
/// or 0 with an `AiError` set.
pub fn count(pair: i64, request: i64, templates: i64, code: i64, line: i32) -> i64 {
    crate::rt_tasks::turn();
    crate::rt_tasks::refuse_if_held(line);
    let prompt = with_str(slots(pair)[0] as i64, |s| s.to_string());
    let described = with_str(request, |s| json::parse(s, &json::Limits::default()));
    release(request);
    let described = described.unwrap_or(Tree::Null);
    let list = |name| match field(&described, name) {
        Some(Tree::Array(items)) => items.clone(),
        _ => Vec::new(),
    };
    let (kinds, models) = (list("kinds"), list("models"));
    // The running agent's model, else the one for no agent (the last).
    let top = RUNNING.with(|r| r.borrow().last().copied());
    let mine = top.and_then(|(_, kind, _)| kinds.iter().position(|k| *k == Tree::Int(kind)));
    let i = mine.unwrap_or(models.len().saturating_sub(1));
    let model = models.get(i).cloned().unwrap_or(Tree::Null);
    let name = match field(&model, "model") {
        Some(Tree::Text(m)) => m.clone(),
        _ => String::new(),
    };
    let shown = format!(
        "ai.count(\"{}\")",
        prompt.chars().take(60).collect::<String>()
    );
    let handle = top.map(|(p, ..)| slots(p)[0] as usize);
    let check = handle.map(|h| with_agent(h, |s| s.guard.begin("Ai.count", &shown)));
    // Refused only while the audit log can't be written (spec 8.6).
    if let (Some(h), Some(c)) = (handle, &check)
        && let Some(why) = c.refused.clone()
    {
        release(pair);
        with_agent(h, |s| s.guard.audit(c, &format!("denied: {why}")));
        let fields = vec![("reason".into(), Tree::Text(why))];
        fail_with(
            &Tree::Object(vec![("Denied".into(), Tree::Object(fields))]),
            templates,
        );
        release(templates);
        return 0;
    }
    let mut side = Asking {
        wires: vec![slots(pair)[1 + i] as i64],
        ty: type_of(code),
        templates,
        line,
        full: false,
    };
    let counted = ai::count(&mut side, &model, &prompt);
    // The pair holds the closures: they go once the counting is done.
    release(pair);
    let Ok(counted) = counted else {
        release(templates);
        return 0;
    };
    let (value, result) = match counted {
        Ok(c) => match crate::rt_json_decode::decode(&c.tree(), type_of(code), "", templates) {
            Ok(v) => (v as i64, format!("ok {}", c.shown())),
            Err(_) => crate::rt::panic_with("`Count` can't hold this count", line),
        },
        Err(f) => (0, fail_with(&f.tree(), templates)),
    };
    release(templates);
    if let (Some(h), Some(mut c)) = (handle, check) {
        c.model = Some(Asked {
            model: name,
            counted: true,
            ..Asked::default()
        });
        with_agent(h, |s| s.guard.audit(&c, &result));
    }
    value
}
