//! How `tokens` counts the `ask`s still waiting for their answer (spec
//! 8.3): at the average size of the answered ones (the default), at the
//! most each can take, or not at all. Each call's entry in a rule's
//! history has a non-zero id while it runs.

use super::{Check, Guard, Mode, Num};

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

    /// Whether the call `c`, which rule `i` would refuse, waits for the
    /// `ask`s under way and is checked again (spec 8.3): under `Average`,
    /// when the rule read `tokens` and any `ask` is under way. Never in
    /// observe mode, where nothing is refused.
    pub fn waits_for_asks(&self, c: &Check, i: usize) -> bool {
        self.settings.under_way == UnderWay::Average
            && self.settings.mode != Mode::Observe
            && c.tokens.contains(&i)
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
        match self.settings.under_way == UnderWay::Largest && c.tokens.contains(&i) {
            true => Some(Num::Int(c.bound)),
            false => added,
        }
    }
}
