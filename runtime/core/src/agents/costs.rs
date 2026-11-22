// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What `ask`s take and cost (spec 8.4, *Prices* and *Usage and cost*,
//! decision 0140), shared by the interpreter and native code: tokens as
//! the provider counts them, prices as the build worked them out, each
//! cost with the note that says how far to trust it, and the totals of
//! the run and of each agent that `ai.spent` reads.

use crate::Decimal;
use crate::json::{self, Tree};
use std::sync::Mutex;

/// Tokens as the provider counts them (`std.ai`'s `Usage`); `None`: no
/// reply said.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Usage {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub cached_input: Option<u64>,
    pub thinking: Option<u64>,
}

fn plus(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (None, None) => None,
        (a, b) => Some(a.unwrap_or(0).saturating_add(b.unwrap_or(0))),
    }
}

impl Usage {
    /// `other` added in: a count is the sum of those given, `None` when
    /// neither gave one.
    pub fn add(&mut self, other: &Usage) {
        self.input = plus(self.input, other.input);
        self.output = plus(self.output, other.output);
        self.cached_input = plus(self.cached_input, other.cached_input);
        self.thinking = plus(self.thinking, other.thinking);
    }

    /// Read from a `Usage` as JSON.
    pub fn read(tree: Option<&Tree>) -> Usage {
        let count = |name: &str| match tree.and_then(|t| field(t, name)) {
            Some(Tree::Int(n)) => Some((*n).max(0) as u64),
            _ => None,
        };
        Usage {
            input: count("input"),
            output: count("output"),
            cached_input: count("cached_input"),
            thinking: count("thinking"),
        }
    }

    /// As `std.ai`'s `Usage` reads it.
    pub fn tree(&self) -> Tree {
        let count = |n: Option<u64>| n.map_or(Tree::Null, |n| Tree::Int(n as i64));
        Tree::Object(vec![
            ("input".into(), count(self.input)),
            ("output".into(), count(self.output)),
            ("cached_input".into(), count(self.cached_input)),
            ("thinking".into(), count(self.thinking)),
        ])
    }
}

fn field<'t>(tree: &'t Tree, name: &str) -> Option<&'t Tree> {
    match tree {
        Tree::Object(fields) => fields.iter().find(|(k, _)| k == name).map(|(_, v)| v),
        _ => None,
    }
}

fn decimal(tree: Option<&Tree>) -> Option<Decimal> {
    match tree? {
        Tree::Text(t) => Decimal::parse(t),
        Tree::Int(n) => Some(Decimal::from_int(i128::from(*n))),
        _ => None,
    }
}

fn dec(n: u64) -> Decimal {
    Decimal::from_int(i128::from(n))
}

/// A model's price, as its provider's `price` gave it (`std.ai`'s
/// `Price`): dollars per `per` tokens.
#[derive(Clone, Debug, PartialEq)]
pub struct Price {
    pub input: Decimal,
    pub output: Decimal,
    pub cached_input: Option<Decimal>,
    pub per: Decimal,
    /// `2026-11-22`.
    pub as_of: String,
    /// What the note calls it: `prices`, `the program's prices`.
    pub label: String,
}

impl Price {
    /// Read from a `Price` as JSON (`null`: no price).
    pub fn read(tree: &Tree) -> Option<Price> {
        // A `Date` is ISO 8601 text in JSON (spec 2.12).
        let as_of = match field(tree, "as_of")? {
            Tree::Text(day) => day.clone(),
            _ => return None,
        };
        let label = match field(tree, "from") {
            Some(Tree::Text(t)) if t == "Program" => "the program's prices".to_string(),
            Some(Tree::Object(fs)) => match fs.first() {
                Some((k, f)) if k == "Package" => match field(f, "name") {
                    Some(Tree::Text(name)) => format!("prices from {name}"),
                    _ => "a package's prices".to_string(),
                },
                _ => "prices".to_string(),
            },
            _ => "prices".to_string(),
        };
        let per = match field(tree, "per") {
            Some(Tree::Int(n)) if *n > 0 => Decimal::from_int(i128::from(*n)),
            _ => return None,
        };
        Some(Price {
            input: decimal(field(tree, "input"))?,
            output: decimal(field(tree, "output"))?,
            cached_input: decimal(field(tree, "cached_input")),
            per,
            as_of,
            label,
        })
    }

    /// What `usage` cost at this price; `None` without an input or output
    /// count. Tokens read from the cache are at `cached_input`.
    pub fn cost(&self, usage: &Usage) -> Option<Decimal> {
        let (input, output) = (usage.input?, usage.output?);
        let cached = usage.cached_input.unwrap_or(0).min(input);
        let at_cache = self.cached_input.unwrap_or(self.input);
        let sum = dec(input - cached).mul(self.input)?;
        let sum = sum.add(dec(cached).mul(at_cache)?)?;
        let sum = sum.add(dec(output).mul(self.output)?)?;
        sum.div(self.per)
    }

    /// The most `input` input tokens and `output` output tokens can cost.
    pub fn most(&self, input: i64, output: i64) -> Decimal {
        let n = |x: i64| Decimal::from_int(i128::from(x.max(0)));
        let sum = (n(input).mul(self.input)).zip(n(output).mul(self.output));
        let sum = sum
            .and_then(|(a, b)| a.add(b))
            .and_then(|s| s.div(self.per));
        sum.unwrap_or(Decimal::from_int(i128::from(i64::MAX)))
    }
}

/// A cost in US dollars, with the prices it came from: each kind of
/// price with its oldest date, and how many `ask`s were left out as
/// their cost isn't known.
#[derive(Clone, Debug, PartialEq)]
pub struct Spend {
    pub dollars: Decimal,
    pub sources: Vec<(String, String)>,
    pub unpriced: u64,
}

impl Default for Spend {
    fn default() -> Spend {
        Spend {
            dollars: Decimal::from_int(0),
            sources: Vec::new(),
            unpriced: 0,
        }
    }
}

impl Spend {
    /// `dollars` more, at `price`.
    pub fn add(&mut self, dollars: Decimal, price: &Price) {
        self.dollars = self.dollars.add(dollars).unwrap_or(self.dollars);
        self.source(&price.label, &price.as_of);
    }

    fn source(&mut self, label: &str, as_of: &str) {
        match self.sources.iter_mut().find(|(l, _)| l == label) {
            Some((_, date)) if as_of < date.as_str() => *date = as_of.to_string(),
            Some(_) => {}
            None => self.sources.push((label.to_string(), as_of.to_string())),
        }
    }

    /// `other` added in.
    pub fn merge(&mut self, other: &Spend) {
        self.dollars = self.dollars.add(other.dollars).unwrap_or(self.dollars);
        for (label, date) in &other.sources {
            self.source(label, date);
        }
        self.unpriced += other.unpriced;
    }

    /// How far to trust it: `estimate: prices as of 2026-11-22; the
    /// provider's bill is exact` (decision 0140).
    pub fn note(&self) -> String {
        let mut parts: Vec<String> = (self.sources.iter())
            .map(|(label, date)| format!("{label} as of {date}"))
            .collect();
        if parts.is_empty() {
            parts.push("no prices used".into());
        }
        if self.unpriced > 0 {
            let asks = if self.unpriced == 1 { "ask" } else { "asks" };
            parts.push(format!(
                "{} {asks} with no known cost left out",
                self.unpriced
            ));
        }
        format!(
            "estimate: {}; the provider's bill is exact",
            parts.join("; ")
        )
    }

    /// `about $0.003568 (estimate: …)`, as a `Cost` prints.
    pub fn shown(&self) -> String {
        format!("about ${} ({})", self.dollars, self.note())
    }

    /// As `std.ai`'s `Cost` reads it, the dollars as text.
    pub fn tree(&self) -> Tree {
        Tree::Object(vec![
            ("dollars".into(), Tree::Text(self.dollars.to_string())),
            ("note".into(), Tree::Text(self.note())),
        ])
    }
}

/// The totals of some `ask`s (`std.ai`'s `Spent`).
#[derive(Clone, Debug, Default)]
struct Spent {
    asks: u64,
    usage: Usage,
    cost: Option<Spend>,
    unpriced: u64,
}

impl Spent {
    fn add(&mut self, usage: &Usage, cost: Option<&Spend>) {
        self.asks += 1;
        self.usage.add(usage);
        match cost {
            Some(c) => self.cost.get_or_insert_with(Spend::default).merge(c),
            None => self.unpriced += 1,
        }
    }

    fn tree(&self) -> Tree {
        let cost = self.cost.clone().map(|mut c| {
            c.unpriced = self.unpriced;
            c.tree()
        });
        Tree::Object(vec![
            ("asks".into(), Tree::Int(self.asks as i64)),
            ("usage".into(), self.usage.tree()),
            ("cost".into(), cost.unwrap_or(Tree::Null)),
            ("unpriced".into(), Tree::Int(self.unpriced as i64)),
        ])
    }
}

/// The run's totals, then each agent's, by name.
type Totals = (Spent, Vec<(String, Spent)>);

static TOTALS: Mutex<Option<Totals>> = Mutex::new(None);

/// An `ask` that reached a model took `usage` and cost `cost` (`None`:
/// not known): it counts for the run, or, with `agent`, for that agent.
pub fn record(agent: Option<&str>, usage: &Usage, cost: Option<&Spend>) {
    let mut totals = TOTALS.lock().unwrap_or_else(|e| e.into_inner());
    let (run, agents) = totals.get_or_insert_with(Default::default);
    let Some(name) = agent else {
        run.add(usage, cost);
        return;
    };
    match agents.iter_mut().find(|(n, _)| n == name) {
        Some((_, s)) => s.add(usage, cost),
        None => {
            let mut s = Spent::default();
            s.add(usage, cost);
            agents.push((name.to_string(), s));
        }
    }
}

/// The run's totals as JSON, or with a name (not empty) that agent's.
pub fn spent_json(agent: &str) -> String {
    let totals = TOTALS.lock().unwrap_or_else(|e| e.into_inner());
    let spent = match (totals.as_ref(), agent) {
        (Some((run, _)), "") => run.clone(),
        (Some((_, agents)), name) => (agents.iter())
            .find(|(n, _)| n == name)
            .map(|(_, s)| s.clone())
            .unwrap_or_default(),
        (None, _) => Spent::default(),
    };
    json::write(&spent.tree(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn price(input: &str, output: &str, cached: Option<&str>) -> Price {
        Price {
            input: Decimal::parse(input).unwrap(),
            output: Decimal::parse(output).unwrap(),
            cached_input: cached.and_then(Decimal::parse),
            per: Decimal::from_int(1_000_000),
            as_of: "2026-11-22".into(),
            label: "prices".into(),
        }
    }

    #[test]
    fn a_cost_is_each_kind_of_token_at_its_price() {
        let used = |i, o, c| Usage {
            input: Some(i),
            output: Some(o),
            cached_input: c,
            thinking: None,
        };
        for (p, u, want) in [
            (
                price("4.00", "20.00", None),
                used(412, 96, None),
                "0.003568",
            ),
            (
                price("0.10", "0.50", None),
                used(412, 96, None),
                "0.0000892",
            ),
            // 400 of the 412 read from the cache, at 0.20.
            (
                price("4.00", "20.00", Some("0.20")),
                used(412, 96, Some(400)),
                "0.002048",
            ),
        ] {
            assert_eq!(p.cost(&u).unwrap().to_string(), want);
        }
        let no_output = Usage {
            input: Some(1),
            ..Usage::default()
        };
        assert_eq!(price("1", "1", None).cost(&no_output), None);
    }

    #[test]
    fn a_price_reads_from_its_json_and_names_its_source() {
        let text = r#"{"input":"4.00","output":"20.00","cached_input":null,"per":1000000,"as_of":"2026-11-01","from":{"Package":{"name":"shop-prices"}}}"#;
        let p = Price::read(&json::parse(text, &Default::default()).unwrap()).unwrap();
        assert_eq!(
            (p.as_of.as_str(), p.label.as_str()),
            ("2026-11-01", "prices from shop-prices")
        );
        assert_eq!(Price::read(&Tree::Null), None);
    }

    #[test]
    fn a_note_keeps_the_oldest_date_and_counts_what_was_left_out() {
        let mut s = Spend::default();
        let mut older = price("1", "1", None);
        older.as_of = "2026-10-01".into();
        s.add(Decimal::from_int(1), &price("1", "1", None));
        s.add(Decimal::from_int(2), &older);
        s.unpriced = 2;
        assert_eq!(
            s.shown(),
            "about $3 (estimate: prices as of 2026-10-01; 2 asks with no known cost left out; the provider's bill is exact)"
        );
    }
}
