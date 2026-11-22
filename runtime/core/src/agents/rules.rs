// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! An agent's limit rules (spec 8.3): what each limits, its window, the
//! calls it counted, and the calls under way that hold a place (`held`).

use super::{Check, Guard, Num};
use std::time::Duration;

/// One limit rule: what it limits, how (as written), and the calls that
/// ran inside its window with what `sum(…)` added for each.
#[derive(Debug)]
pub struct Rule {
    /// `Payments.refund`, or `Ai` for a whole capability.
    pub target: String,
    /// `sum(amount) <= 100 per day`.
    pub text: String,
    pub(super) window: Option<Duration>,
    /// Each call's time on the steady clock (`steady_ms`), which is
    /// negative for a call from before the program started; what it
    /// added; and the id of a call still running (0 for one that ended).
    pub(super) history: Vec<(i64, Option<Num>, u64)>,
    /// The ids of the calls under way that hold a place, for `held`.
    pub(super) under_way: Vec<u64>,
}

impl Rule {
    /// A rule; `per` is its window's unit (`day`), if it has one.
    pub fn new(target: &str, expr: &str, per: Option<&str>) -> Rule {
        Rule {
            target: target.to_string(),
            text: match per {
                Some(p) => format!("{expr} per {p}"),
                None => expr.to_string(),
            },
            window: per.and_then(window),
            history: Vec::new(),
            under_way: Vec::new(),
        }
    }

    /// The key its calls are kept under in the state file: what it limits
    /// and how, so a rule that changed starts again from nothing.
    pub(super) fn key(&self) -> String {
        format!("{}: {}", self.target, self.text)
    }
}

fn window(per: &str) -> Option<Duration> {
    let hour = 3600;
    Some(Duration::from_secs(match per {
        "minute" => 60,
        "hour" => hour,
        "day" => 24 * hour,
        "week" => 7 * 24 * hour,
        "month" => 30 * 24 * hour,
        _ => return None,
    }))
}

/// Whether a rule or approval for `target` covers a call of `call`
/// (`Cap.method`): the method itself, or its whole capability.
pub fn covers(target: &str, call: &str) -> bool {
    target == call || call.split('.').next() == Some(target)
}

impl Guard {
    /// The calls under way that hold a place in rule `i` (`held`, spec
    /// 8.3), not counting the one being checked.
    pub fn held(&self, i: usize) -> i64 {
        self.rules[i].under_way.len() as i64
    }

    /// The call returned, failed or stopped: the places it held are free
    /// again.
    pub fn let_go(&mut self, c: &Check) {
        for (i, id) in &c.holding {
            self.rules[*i].under_way.retain(|x| x != id);
        }
    }
}
