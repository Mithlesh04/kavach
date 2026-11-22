// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `tokens` and `cost` limits in native code (spec 8.3): what `tokens`
//! and `cost` read, with the `ask`s still waiting for their answer
//! counted as the agent's settings say; a rule's verdict, which under
//! `UnderWay::Average` can be "wait for the `ask`s under way, then check
//! again"; the most an `ask` can take; and its models' prices. The
//! bookkeeping is `kavach_runtime_core::agents`, the interpreter's too.

use crate::heap::{Data, alloc, release};
use crate::rt::type_of;
use crate::rt_agents::with_pending;
use crate::shapes::with_str;
use kavach_kir::RtOp;
use kavach_runtime_core::agents::costs::Price;
use kavach_runtime_core::agents::largest_ask;
use kavach_runtime_core::json::{self, Tree};

/// How often an `ask` waiting for those under way looks again.
const LOOK: std::time::Duration = std::time::Duration::from_millis(20);

/// The verdict that checks the rule again (`compiler/kir/src/lower_guard.rs`).
const AGAIN: i64 = 2;

/// `AgentTokens`, `AgentCost`, `AgentLimit`, `AgentBound` and
/// `AgentPrices`, for the call being checked `a` and its rule `b` (for
/// `AgentBound`, `b` is `max_tokens`; for `AgentPrices`, the owned text
/// of its models' prices). `code` is the result type.
pub(crate) fn run(op: RtOp, a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    let rule = b as usize;
    match op {
        RtOp::AgentTokens => with_pending(a, |p, s| {
            p.used = true;
            if !p.check.tokens.contains(&rule) {
                p.check.tokens.push(rule);
            }
            s.guard.tokens(rule, &p.check)
        }),
        RtOp::AgentCost => {
            let cost = with_pending(a, |p, s| {
                p.used = true;
                if !p.check.costs.contains(&rule) {
                    p.check.costs.push(rule);
                }
                s.guard.cost(rule, &p.check)
            });
            alloc(type_of(code), Data::Decimal(Box::new(cost)))
        }
        RtOp::AgentBound => with_pending(a, |p, _| {
            p.check.bound = largest_ask(p.prompt_bytes, b);
            0
        }),
        RtOp::AgentPrices => {
            let text = with_str(b, str::to_string);
            release(b);
            let tree = json::parse(&text, &json::Limits::default()).unwrap_or(Tree::Null);
            let models = prices(&tree);
            with_pending(a, |p, _| {
                p.check.priced(p.prompt_bytes, &models);
                0
            })
        }
        RtOp::AgentLimit => {
            let verdict = with_pending(a, |p, s| {
                let ok = c != 0;
                if !ok && s.guard.waits_for_asks(&p.check, rule) {
                    p.used = false;
                    p.added = None;
                    return None;
                }
                let (used, added) = (p.used, p.added.take());
                p.used = false;
                Some(i64::from(s.guard.limit(
                    &mut p.check,
                    rule,
                    ok,
                    used,
                    added,
                )))
            });
            if let Some(v) = verdict {
                return v;
            }
            // The locks are let go while it waits, so the `ask`s under
            // way can answer.
            crate::rt_waits::sleep_until(std::time::Instant::now() + LOOK, line);
            // Cancelled (the program ends): refused, so the handler ends.
            if crate::rt_tasks::CANCELLING.get() {
                return 1;
            }
            AGAIN
        }
        _ => 0,
    }
}

/// Each model's name, `max_tokens` and price, from the text the compiler
/// wrote (`compiler/kir/src/lower_ai.rs`).
fn prices(tree: &Tree) -> Vec<(String, i64, Option<Price>)> {
    let Tree::Array(items) = tree else {
        return Vec::new();
    };
    let field = |t: &Tree, name: &str| match t {
        Tree::Object(fs) => fs.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone()),
        _ => None,
    };
    (items.iter())
        .map(|m| {
            let name = match field(m, "model") {
                Some(Tree::Text(n)) => n,
                _ => String::new(),
            };
            let most = match field(m, "max_tokens") {
                Some(Tree::Int(n)) => n,
                _ => 0,
            };
            let price = field(m, "price").and_then(|p| Price::read(&p));
            (name, most, price)
        })
        .collect()
}
