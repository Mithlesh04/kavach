// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! How `tokens` counts the `ask`s still waiting for their answer (spec
//! 8.3): at the average size of the answered ones (the default), at the
//! most each can take, or not at all. Each call's entry in a rule's
//! history has a non-zero id while it runs.

use super::costs::Price;
use super::{Check, Guard, Mode, Num};
use crate::Decimal;

/// `asks_under_way` in an agent's settings (spec 8.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnderWay {
    /// Each at the average size of the window's answered `ask`s; a new
    /// `ask` the limit would refuse waits for them, then is checked again.
    #[default]
    Average,
    /// Each at the most it can take (`largest_ask`), the new one too.
    Largest,
    /// Not counted.
    NotCounted,
}

impl UnderWay {
    /// The one named in the settings (`Largest`); `Average` for any other.
    pub fn named(name: &str) -> UnderWay {
        match name {
            "Largest" => UnderWay::Largest,
            "NotCounted" => UnderWay::NotCounted,
            _ => UnderWay::Average,
        }
    }
}

/// The most tries one `ask` makes: the first, and 2 more when the answer
/// doesn't fit its type (spec 8.4).
pub const TRIES: i64 = 3;

/// The most an `ask` can take: for each try, its prompt's length in bytes
/// (no token is shorter than a byte) and its model's `max_tokens`.
pub fn largest_ask(prompt_bytes: usize, max_tokens: i64) -> i64 {
    let prompt = i64::try_from(prompt_bytes).unwrap_or(i64::MAX);
    prompt.saturating_add(max_tokens).saturating_mul(TRIES)
}

/// The most an `ask` can cost (spec 8.3, `UnderWay.Largest`): for each
/// try, its prompt's bytes at its dearest model's input price, and that
/// model's `max_tokens` at its output price. `models` are each model's
/// `max_tokens` and price.
pub fn largest_cost(prompt_bytes: usize, models: &[(i64, &Price)]) -> Decimal {
    let prompt = i64::try_from(prompt_bytes).unwrap_or(i64::MAX);
    let most = (models.iter()).map(|(max_tokens, p)| p.most(prompt, *max_tokens));
    let most = most.fold(Decimal::from_int(0), |a, b| if b > a { b } else { a });
    most.mul(Decimal::from_int(i128::from(TRIES)))
        .unwrap_or(most)
}

impl Check {
    /// The prices of an `ask`'s models, in the order they are tried: each
    /// one's name, `max_tokens` and price. The first with no price is
    /// the one a `cost` rule refuses (spec 8.3).
    pub fn priced(&mut self, prompt_bytes: usize, models: &[(String, i64, Option<Price>)]) {
        self.unpriced = (models.iter())
            .find(|(_, _, p)| p.is_none())
            .map(|(name, ..)| name.clone());
        let priced: Vec<(i64, &Price)> = (models.iter())
            .filter_map(|(_, n, p)| Some((*n, p.as_ref()?)))
            .collect();
        self.cost_bound = largest_cost(prompt_bytes, &priced);
    }
}

/// What `tokens` reads while `ask`s are under way and none has answered,
/// under `Average`: their size isn't known, so more than any limit.
const NOT_KNOWN: i64 = i64::MAX / 4;

impl Guard {
    /// What `tokens` reads in rule `i` for the call `c` (spec 8.3).
    pub fn tokens(&mut self, i: usize, c: &Check) -> i64 {
        let sum = match self.totals(i).0 {
            Some(Num::Int(n)) => n,
            _ => 0,
        };
        match self.settings.under_way {
            UnderWay::NotCounted => sum,
            // The ones under way hold their largest size already
            // (`counted_while_under_way`); this one adds its own.
            UnderWay::Largest => sum.saturating_add(c.bound),
            UnderWay::Average => {
                let history = &self.rules[i].history;
                let under_way = history.iter().filter(|e| e.2 != 0).count() as i64;
                let answered = (history.iter())
                    .filter(|e| e.2 == 0 && e.1.is_some())
                    .count() as i64;
                match (under_way, answered) {
                    (0, _) => sum,
                    (_, 0) => NOT_KNOWN,
                    _ => sum.saturating_add(under_way.saturating_mul(sum / answered)),
                }
            }
        }
    }

    /// What `cost` reads in rule `i` for the call `c` (spec 8.3): as
    /// `tokens`, in US dollars.
    pub fn cost(&mut self, i: usize, c: &Check) -> Decimal {
        let zero = Decimal::from_int(0);
        let sum = match self.totals(i).0 {
            Some(Num::Decimal(d)) => d,
            _ => zero,
        };
        match self.settings.under_way {
            UnderWay::NotCounted => sum,
            UnderWay::Largest => sum.add(c.cost_bound).unwrap_or(sum),
            UnderWay::Average => {
                let history = &self.rules[i].history;
                let under_way = history.iter().filter(|e| e.2 != 0).count() as i128;
                let answered = (history.iter())
                    .filter(|e| e.2 == 0 && e.1.is_some())
                    .count() as i128;
                let each = sum.div(Decimal::from_int(answered.max(1))).unwrap_or(zero);
                match (under_way, answered) {
                    (0, _) => sum,
                    (_, 0) => Decimal::from_int(i128::from(NOT_KNOWN)),
                    _ => (each.mul(Decimal::from_int(under_way)))
                        .and_then(|more| sum.add(more))
                        .unwrap_or(sum),
                }
            }
        }
    }

    /// What the window's `ask`s cost so far in rule `i`, for the message
    /// of a refusal (spec 8.3).
    pub(super) fn cost_so_far(&self, i: usize) -> Decimal {
        let rule = &self.rules[i];
        let now = super::steady_ms();
        let inside = |t: i64| rule.window.is_none_or(|w| now - t < w.as_millis() as i64);
        let mut sum = Decimal::from_int(0);
        for (t, x, id) in &rule.history {
            if let (true, 0, Some(Num::Decimal(d))) = (inside(*t), id, x) {
                sum = sum.add(*d).unwrap_or(sum);
            }
        }
        sum
    }

    /// Whether the call `c`, which rule `i` would refuse, waits for the
    /// `ask`s under way and is checked again (spec 8.3): under `Average`,
    /// when the rule read `tokens` and any `ask` is under way. Never in
    /// observe mode, where nothing is refused.
    pub fn waits_for_asks(&self, c: &Check, i: usize) -> bool {
        let reads = c.tokens.contains(&i) || (c.costs.contains(&i) && c.unpriced.is_none());
        self.settings.under_way == UnderWay::Average
            && self.settings.mode != Mode::Observe
            && reads
            && self.rules[i].history.iter().any(|e| e.2 != 0)
    }

    /// What the call `c` adds to rule `i` while it runs: under `Largest`,
    /// an `ask` holds its largest size until its answer says what it took.
    pub(super) fn counted_while_under_way(
        &self,
        c: &Check,
        i: usize,
        added: Option<Num>,
    ) -> Option<Num> {
        match self.settings.under_way == UnderWay::Largest {
            true if c.tokens.contains(&i) => Some(Num::Int(c.bound)),
            true if c.costs.contains(&i) => Some(Num::Decimal(c.cost_bound)),
            _ => added,
        }
    }
}
