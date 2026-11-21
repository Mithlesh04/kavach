// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What crosses between the runtime and a provider (spec 8.4, decision
//! 0140), as `std.ai`'s `wire` writes it: `json.stringify` of a request
//! with where its key goes, and of a `Reply`. And `AiError`, as the JSON
//! each backend decodes into the program's value.

use super::{Request, Tokens, gate};
use kavach_runtime_core::json::{self, Limits, Tree};
use std::time::Duration;

/// Why an `ask` failed: `std.ai`'s `AiError`.
#[derive(Clone, Debug, PartialEq)]
pub enum Failure {
    NoKey(String),
    RateLimited(String),
    QuotaUsedUp(String),
    Overloaded(String),
    BadKey(String),
    NotFound(String),
    BadRequest(String),
    Refused(Option<String>),
    CutOff(i64),
    /// After this many milliseconds.
    TimedOut(u64),
    Network(String),
    InvalidOutput(String),
    Failed(i64, String),
}

impl Failure {
    /// The variant's name, as the audit log shows a model an `ask` moved
    /// on from (spec 8.6): `RateLimited`.
    pub fn variant(&self) -> String {
        match self.tree() {
            Tree::Object(mut fields) if !fields.is_empty() => fields.swap_remove(0).0,
            _ => String::new(),
        }
    }

    /// The `AiError` value as JSON (spec 2.12): `{"Variant": {fields}}`.
    pub fn tree(&self) -> Tree {
        let text = |s: &str| Tree::Text(s.to_string());
        let (name, fields): (&str, Vec<(&str, Tree)>) = match self {
            Failure::NoKey(v) => ("NoKey", vec![("variable", text(v))]),
            Failure::RateLimited(m) => ("RateLimited", vec![("message", text(m))]),
            Failure::QuotaUsedUp(m) => ("QuotaUsedUp", vec![("message", text(m))]),
            Failure::Overloaded(m) => ("Overloaded", vec![("message", text(m))]),
            Failure::BadKey(m) => ("BadKey", vec![("message", text(m))]),
            Failure::NotFound(m) => ("NotFound", vec![("model", text(m))]),
            Failure::BadRequest(m) => ("BadRequest", vec![("message", text(m))]),
            Failure::Refused(r) => {
                let reason = r.as_deref().map_or(Tree::Null, text);
                ("Refused", vec![("reason", reason)])
            }
            Failure::CutOff(n) => ("CutOff", vec![("max_tokens", Tree::Int(*n))]),
            Failure::TimedOut(ms) => {
                let after = Tree::Object(vec![("ms".into(), Tree::Int(*ms as i64))]);
                ("TimedOut", vec![("after", after)])
            }
            Failure::Network(m) => ("Network", vec![("message", text(m))]),
            Failure::InvalidOutput(a) => ("InvalidOutput", vec![("answer", text(a))]),
            Failure::Failed(s, m) => (
                "Failed",
                vec![("status", Tree::Int(*s)), ("message", text(m))],
            ),
        };
        let fields = fields.into_iter().map(|(k, v)| (k.to_string(), v));
        Tree::Object(vec![(name.into(), Tree::Object(fields.collect()))])
    }
}

/// Field `name` of an object.
pub(crate) fn field<'t>(tree: &'t Tree, name: &str) -> Option<&'t Tree> {
    match tree {
        Tree::Object(fields) => fields.iter().find(|(k, _)| k == name).map(|(_, v)| v),
        _ => None,
    }
}

fn text_of(tree: Option<&Tree>) -> Option<String> {
    match tree {
        Some(Tree::Text(s)) => Some(s.clone()),
        _ => None,
    }
}

fn int_of(tree: Option<&Tree>) -> Option<i64> {
    match tree {
        Some(Tree::Int(n)) => Some(*n),
        _ => None,
    }
}

/// A union variant as `json.stringify` writes it: its name, and its
/// fields (an empty object for a variant without them).
fn variant(tree: &Tree) -> Option<(&str, &Tree)> {
    static EMPTY: Tree = Tree::Object(Vec::new());
    match tree {
        Tree::Text(name) => Some((name, &EMPTY)),
        Tree::Object(fields) if fields.len() == 1 => Some((&fields[0].0, &fields[0].1)),
        _ => None,
    }
}

fn parse(text: &str) -> Tree {
    json::parse(text, &Limits::default()).unwrap_or(Tree::Null)
}

/// The provider's code gave something the runtime can't read.
fn unreadable(what: &str) -> Failure {
    Failure::Failed(
        0,
        format!("the provider gave no {what} the runtime can read (spec 8.4)"),
    )
}

/// Where the key goes (`std.ai`'s `Auth`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Auth {
    NoKey,
    Bearer,
    InHeader(String),
}

/// A request to send, as the provider wrote it.
#[derive(Debug)]
pub(crate) struct Call {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub auth: Auth,
    pub key_variable: String,
}

impl Call {
    pub(crate) fn read(text: &str) -> Result<Call, Failure> {
        let tree = parse(text);
        let headers = match field(&tree, "headers") {
            Some(Tree::Object(hs)) => hs
                .iter()
                .filter_map(|(k, v)| Some((k.clone(), text_of(Some(v))?)))
                .collect(),
            _ => Vec::new(),
        };
        let auth = match field(&tree, "auth").and_then(variant) {
            Some(("NoKey", _)) => Auth::NoKey,
            Some(("Bearer", _)) => Auth::Bearer,
            Some(("InHeader", f)) => Auth::InHeader(text_of(field(f, "name")).unwrap_or_default()),
            Some((other, _)) => {
                return Err(Failure::Failed(
                    0,
                    format!(
                        "`Auth.{other}` is not built yet: give the key as a plain key (decision 0140)"
                    ),
                ));
            }
            None => return Err(unreadable("request")),
        };
        Ok(Call {
            url: text_of(field(&tree, "url")).ok_or_else(|| unreadable("request"))?,
            headers,
            body: text_of(field(&tree, "body")).unwrap_or_default(),
            auth,
            key_variable: text_of(field(&tree, "key_variable")).unwrap_or_default(),
        })
    }
}

/// What went wrong (`std.ai`'s `Trouble`), and `Network`, which only the
/// runtime finds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Kind {
    RateLimited,
    QuotaUsedUp,
    Overloaded,
    BadKey,
    NotFound,
    BadRequest,
    TimedOut,
    Network,
    Other(i64),
}

/// A reply, as the provider read it (`std.ai`'s `Reply`).
#[derive(Debug)]
pub(crate) enum Reply {
    Answer(String, Tokens),
    Refused(Option<String>, Tokens),
    CutOff(Tokens),
    /// What went wrong, the provider's message, and how long it asked to
    /// wait.
    Problem(Kind, String, Option<Duration>),
}

impl Reply {
    pub(crate) fn read(text: &str) -> Result<Reply, Failure> {
        let tree = parse(text);
        let (name, f) = variant(&tree).ok_or_else(|| unreadable("reply"))?;
        // A count the provider didn't give is none: 0 here (spec 8.4).
        let usage = |f: &Tree| {
            let u = field(f, "usage");
            let count = |n: &str| u.and_then(|u| int_of(field(u, n))).unwrap_or(0).max(0) as u64;
            Tokens {
                input: count("input"),
                output: count("output"),
            }
        };
        Ok(match name {
            "Answer" => Reply::Answer(text_of(field(f, "text")).unwrap_or_default(), usage(f)),
            "Refused" => Reply::Refused(text_of(field(f, "reason")), usage(f)),
            "CutOff" => Reply::CutOff(usage(f)),
            "Problem" => {
                let kind = match field(f, "kind").and_then(variant) {
                    Some(("RateLimited", _)) => Kind::RateLimited,
                    Some(("QuotaUsedUp", _)) => Kind::QuotaUsedUp,
                    Some(("Overloaded", _)) => Kind::Overloaded,
                    Some(("BadKey", _)) => Kind::BadKey,
                    Some(("NotFound", _)) => Kind::NotFound,
                    Some(("BadRequest", _)) => Kind::BadRequest,
                    Some(("TimedOut", _)) => Kind::TimedOut,
                    Some(("Other", o)) => Kind::Other(int_of(field(o, "status")).unwrap_or(0)),
                    _ => return Err(unreadable("reply")),
                };
                let wait = field(f, "wait")
                    .and_then(|w| int_of(field(w, "ms")))
                    .map(|ms| Duration::from_millis(ms.max(0) as u64));
                Reply::Problem(kind, text_of(field(f, "message")).unwrap_or_default(), wait)
            }
            _ => return Err(unreadable("reply")),
        })
    }

    /// The tokens the reply says it took, if it says.
    pub(crate) fn usage(&self) -> Option<Tokens> {
        match self {
            Reply::Answer(_, t) | Reply::Refused(_, t) | Reply::CutOff(t) => Some(*t),
            Reply::Problem(..) => None,
        }
    }

    /// How long to wait before try `k + 1` is sent, or `None` when it
    /// isn't: not after an answer, after trouble a wait can't fix, nor
    /// after the `retries` (spec 8.4). The provider's own hint comes
    /// first, up to the `timeout`.
    pub(crate) fn wait(&self, k: usize, req: &Request) -> Option<Duration> {
        let Reply::Problem(kind, _, hint) = self else {
            return None;
        };
        let again = matches!(
            kind,
            Kind::RateLimited | Kind::Overloaded | Kind::TimedOut | Kind::Network
        );
        if !again || k >= req.retries {
            return None;
        }
        let step = gate::wait_before(k);
        Some(hint.map_or(step, |h| h.min(req.timeout).max(step)))
    }

    /// Whether the call goes on to the next model after this reply,
    /// when `model` is a list: never after a refusal, so a fallback
    /// can't get round a safety filter (spec 8.4, *Fallbacks*).
    pub(crate) fn moves_on(&self) -> bool {
        matches!(
            self,
            Reply::Problem(
                Kind::RateLimited
                    | Kind::QuotaUsedUp
                    | Kind::Overloaded
                    | Kind::BadKey
                    | Kind::NotFound
                    | Kind::TimedOut
                    | Kind::Network,
                ..
            )
        )
    }

    /// The `AiError` a reply that is not an answer fails the `ask` with.
    pub(crate) fn failure(&self, req: &Request) -> Failure {
        match self {
            Reply::Answer(..) => Failure::Failed(200, String::new()),
            Reply::Refused(reason, _) => Failure::Refused(reason.clone()),
            Reply::CutOff(_) => Failure::CutOff(req.max_tokens),
            Reply::Problem(kind, m, _) => {
                let m = m.clone();
                match kind {
                    Kind::RateLimited => Failure::RateLimited(m),
                    Kind::QuotaUsedUp => Failure::QuotaUsedUp(m),
                    Kind::Overloaded => Failure::Overloaded(m),
                    Kind::BadKey => Failure::BadKey(m),
                    Kind::NotFound => Failure::NotFound(req.model.clone()),
                    Kind::BadRequest => Failure::BadRequest(m),
                    Kind::TimedOut => Failure::TimedOut(req.timeout.as_millis() as u64),
                    Kind::Network => Failure::Network(m),
                    Kind::Other(status) => Failure::Failed(*status, m),
                }
            }
        }
    }
}
