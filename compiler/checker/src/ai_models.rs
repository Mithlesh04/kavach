// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The `Model` of each agent and of each `ask` that names one (spec 8.4,
//! decision 0056), worked out when the program is built: native code
//! builds each request then. A `Model(…)` written with plain values, or
//! a `const` set to one, can be; anything else is an error that says so.

use crate::checker::Checker;
use kavach_ast::visit::{VisitMut, walk_file, walk_stmt};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};
use std::collections::HashMap;

/// A model and its settings, as an `ask` sends them (decision 0059).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AiModel {
    pub name: String,
    /// `low` to `max`; `None`: the model's own.
    pub effort: Option<String>,
    /// `None`: 4,096.
    pub max_tokens: Option<u32>,
    /// `None`: the model's own; `Some(false)`: no thinking first.
    pub thinking: Option<bool>,
    /// The system prompt.
    pub instructions: Option<String>,
    pub stop: Vec<String>,
    /// `None`: 120 seconds.
    pub timeout_ms: Option<u64>,
    /// `None`: 4.
    pub retries: Option<u32>,
    pub api_url: Option<String>,
    pub api_key_env: Option<String>,
    /// More request fields, as the text of a JSON object.
    pub extra: Option<String>,
    /// The provider's value, worked out when the program is built: where
    /// its expression is (a key of `AiModels::providers`), and its type's
    /// name (decision 0140). `None`: `Anthropic()`.
    pub provider: Option<((u32, u32), String)>,
    /// The models tried after this one, in turn, when `model` is a list
    /// (spec 8.4, *Fallbacks*); each has none of its own.
    pub then: Vec<AiModel>,
}

impl AiModel {
    /// This model, then each one tried after it.
    pub fn in_turn(&self) -> impl Iterator<Item = &AiModel> {
        std::iter::once(self).chain(&self.then)
    }

    /// The largest `max_tokens` of the models tried, `default` for one
    /// that doesn't say.
    pub fn most_tokens(&self, default: u32) -> u32 {
        let each = self.in_turn().map(|m| m.max_tokens.unwrap_or(default));
        each.max().unwrap_or(default)
    }
}

/// The models of a program (decision 0056) and their providers (0140).
#[derive(Default)]
pub struct AiModels {
    /// Each agent's model, by the agent's name.
    pub agents: HashMap<String, AiModel>,
    /// Each `ask`'s own `model:`, by the call's span.
    pub asks: HashMap<(u32, u32), AiModel>,
    /// Each provider's expression, by its span: a constructor written
    /// with plain values.
    pub providers: HashMap<(u32, u32), Expr>,
    /// Each model as written, with where: for its provider's `check`.
    pub written: Vec<(Span, AiModel)>,
}

/// Why the compiler refuses `m`, whatever its provider. What a provider
/// can't take, its own `check` says (spec 8.4).
fn refused(m: &AiModel) -> Option<String> {
    if m.max_tokens.is_some_and(|n| n == 0 || n > 128_000) {
        return Some("`max_tokens` is from 1 to 128,000 (spec 8.4)".into());
    }
    if m.retries.is_some_and(|n| n > 10) {
        return Some("`retries` is from 0 to 10 (spec 8.4)".into());
    }
    if let Some(x) = &m.extra {
        let x = x.trim();
        if !(x.starts_with('{') && x.ends_with('}')) {
            return Some(
                "`extra` is the text of a JSON object, between `{` and `}` (spec 8.4)".into(),
            );
        }
    }
    None
}

/// A plain text literal's text.
pub(crate) fn text(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Str(parts) => match &parts[..] {
            [StrPart::Text(s)] => Some(s.clone()),
            [] => Some(String::new()),
            _ => None,
        },
        _ => None,
    }
}

const NOT_KNOWN: &str = "a model is chosen when the program is built: write `Model(name: \"…\")` with plain values (its `provider` too), or call a function with no parameters that gives one, such as `careful()` (spec 8.4)";

const EMPTY: &str = "a list of models needs at least one model: they are tried in turn, the first one first (spec 8.4)";

impl Checker<'_> {
    /// Each agent's model, by the agent's name, and each `ask`'s own, by
    /// the call's span.
    pub(crate) fn ai_models(&mut self, file: &File) -> AiModels {
        let consts = self.known_values(file);
        let mut agents = HashMap::new();
        let mut asks = HashMap::new();
        let mut providers = HashMap::new();
        let mut wrong = Vec::new();
        // Each model worked out, with where it is written, to check.
        let mut checked = Vec::new();
        let mut found = |e: &Expr, checked: &mut Vec<(Span, AiModel)>| {
            let models = self.models_of(e, &consts, &mut providers);
            // An item of the wrong type has its error already (`check_model`).
            let models = (models.map_err(|why| why.map(|why| wrong.push((e.span, why))))).ok()?;
            checked.extend(models.iter().map(|m| (e.span, m.clone())));
            let mut models = models.into_iter();
            let mut first = models.next()?;
            first.then = models.collect();
            Some(first)
        };
        for item in &file.items {
            let ItemKind::Agent(a) = &item.kind else {
                continue;
            };
            for m in &a.members {
                if let AgentMember::Model(e) = m
                    && let Some(model) = found(e, &mut checked)
                {
                    agents.insert(a.name.name.clone(), model);
                }
            }
        }
        let mut calls = Asks { found: Vec::new() };
        walk_file(&mut calls, &mut file.clone());
        for (call, e) in calls.found {
            if let Some(model) = found(&e, &mut checked) {
                asks.insert(call, model);
            }
        }
        for (span, why) in wrong {
            self.error(span, why);
        }
        for (span, model) in &checked {
            if let Some(why) = refused(model) {
                self.error(*span, why);
            }
        }
        AiModels {
            agents,
            asks,
            providers,
            written: checked,
        }
    }

    /// What the compiler knows the value of: each `const`, and each
    /// function with no parameters whose body is one expression, the way
    /// to share a `Model` or `AgentSettings` between agents (0056, 0060).
    pub(crate) fn known_values(&self, file: &File) -> HashMap<DefId, Expr> {
        let mut consts = Consts {
            checker: self,
            found: HashMap::new(),
        };
        walk_file(&mut consts, &mut file.clone());
        let mut consts = consts.found;
        for item in &file.items {
            let ItemKind::Fn(f) = &item.kind else {
                continue;
            };
            let body = f.body.as_ref().map(|b| &b.stmts[..]);
            if let (true, Some([only])) = (f.params.is_empty(), body)
                && let (StmtKind::Expr(e), Some(d)) =
                    (&only.kind, self.decl.get(&f.name.span.start))
            {
                consts.insert(*d, e.clone());
            }
        }
        consts
    }

    /// The models `e` gives, in the order they are tried: one, or each of
    /// a list of them (spec 8.4, *Fallbacks*); or why they can't be
    /// worked out now (`None`: an item isn't a `Model`, which the type
    /// check reported already).
    fn models_of(
        &self,
        e: &Expr,
        consts: &HashMap<DefId, Expr>,
        providers: &mut HashMap<(u32, u32), Expr>,
    ) -> Result<Vec<AiModel>, Option<&'static str>> {
        let one = |e: &Expr, providers: &mut HashMap<(u32, u32), Expr>| {
            self.model_of(e, consts, 0, providers)
        };
        let why = |m: &Expr| (!self.not_a_model(m)).then_some(NOT_KNOWN);
        match &e.kind {
            ExprKind::List(items) if items.is_empty() => Err(Some(EMPTY)),
            ExprKind::List(items) => (items.iter())
                .map(|m| one(m, providers).ok_or_else(|| why(m)))
                .collect(),
            _ => one(e, providers).map(|m| vec![m]).ok_or_else(|| why(e)),
        }
    }

    /// Whether the type check found `e` is not a `Model`.
    fn not_a_model(&self, e: &Expr) -> bool {
        let model = self.builtin_ty("Model");
        (self.expr_types.iter())
            .find(|(span, _)| *span == e.span)
            .is_some_and(|(_, t)| self.inf.resolve(t) != model)
    }

    /// The model `e` gives, if it can be worked out now.
    fn model_of(
        &self,
        e: &Expr,
        consts: &HashMap<DefId, Expr>,
        depth: u32,
        providers: &mut HashMap<(u32, u32), Expr>,
    ) -> Option<AiModel> {
        match &e.kind {
            ExprKind::Name(id) if depth < 8 => {
                let d = self.res.uses.get(&id.span.start)?;
                self.model_of(consts.get(d)?, consts, depth + 1, providers)
            }
            ExprKind::Call { callee, args, .. } => {
                let ExprKind::Name(f) = &callee.kind else {
                    return None;
                };
                if f.name != "Model" {
                    // `careful()`: a function that gives one.
                    let d = self.res.uses.get(&f.span.start)?;
                    return match (args.is_empty(), depth < 8) {
                        (true, true) => self.model_of(consts.get(d)?, consts, depth + 1, providers),
                        _ => None,
                    };
                }
                let mut model = AiModel::default();
                for a in args {
                    let v = &a.value;
                    match (a.name.as_ref()?.name.as_str(), &v.kind) {
                        ("name", _) => model.name = text(v)?,
                        // `Effort.High` (decision 0070).
                        ("effort", ExprKind::Field { base, name, .. }) => {
                            let ExprKind::Name(u) = &base.kind else {
                                return None;
                            };
                            let known = ["Low", "Medium", "High", "XHigh", "Max"];
                            (u.name == "Effort" && known.contains(&name.name.as_str()))
                                .then_some(())?;
                            model.effort = Some(name.name.to_lowercase());
                        }
                        ("thinking", ExprKind::Bool(b)) => model.thinking = Some(*b),
                        ("max_tokens", ExprKind::Int(n)) => {
                            model.max_tokens = Some(u32::try_from(*n).ok()?)
                        }
                        ("retries", ExprKind::Int(n)) => {
                            model.retries = Some(u32::try_from(*n).ok()?)
                        }
                        ("timeout", ExprKind::Field { base, name, .. }) => {
                            let ExprKind::Int(n) = base.kind else {
                                return None;
                            };
                            let (_, per) = crate::DURATION_UNITS
                                .iter()
                                .find(|(u, _)| *u == name.name)?;
                            model.timeout_ms = Some(n.checked_mul(*per as u64)?);
                        }
                        ("instructions", _) => model.instructions = Some(text(v)?),
                        ("api_url", _) => model.api_url = Some(text(v)?),
                        ("api_key_env", _) => model.api_key_env = Some(text(v)?),
                        ("extra", _) => model.extra = Some(text(v)?),
                        ("provider", _) => {
                            let p = self.provider_of(v, consts, 0)?;
                            let at = (p.span.start, p.span.end);
                            let ExprKind::Call { callee, .. } = &p.kind else {
                                return None;
                            };
                            let ExprKind::Name(name) = &callee.kind else {
                                return None;
                            };
                            model.provider = Some((at, name.name.clone()));
                            providers.insert(at, p);
                        }
                        ("stop", ExprKind::List(items)) => {
                            model.stop = items.iter().map(text).collect::<Option<_>>()?;
                        }
                        // Gone (decision 0059): the field check says so.
                        ("temperature", _) => {}
                        _ => return None,
                    }
                }
                (!model.name.is_empty()).then_some(model)
            }
            _ => None,
        }
    }
}

impl Checker<'_> {
    /// An agent's `model`, or a call's `model:`: a `Model`, or a list
    /// literal of them, tried in turn (spec 8.4, *Fallbacks*).
    pub(crate) fn check_model(&mut self, e: &Expr) {
        let model = self.builtin_ty("Model");
        match &e.kind {
            ExprKind::List(_) => self.check_expect(e, &crate::ty::Ty::List(Box::new(model))),
            _ => self.check_expect(e, &model),
        };
    }

    /// The provider `e` gives, as the constructor call it comes to: one
    /// written with plain values, a `const` set to one, or a function
    /// with no parameters that gives one (spec 8.4).
    fn provider_of(&self, e: &Expr, consts: &HashMap<DefId, Expr>, depth: u32) -> Option<Expr> {
        let followed = |id: &Ident| {
            let d = self.res.uses.get(&id.span.start)?;
            (depth < 8).then(|| self.provider_of(consts.get(d)?, consts, depth + 1))?
        };
        match &e.kind {
            ExprKind::Name(id) => followed(id),
            ExprKind::Call { callee, args, .. } => {
                let ExprKind::Name(f) = &callee.kind else {
                    return None;
                };
                let d = *self.res.uses.get(&f.span.start)?;
                if self.res.defs.get(d)?.kind == DefKind::Type {
                    return args.iter().all(|a| plain(&a.value)).then(|| e.clone());
                }
                args.is_empty().then(|| followed(f))?
            }
            _ => None,
        }
    }
}

/// Whether `e` is written with plain values only: literals, lists of
/// them, variants, durations, and constructors of them.
fn plain(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::None => true,
        ExprKind::Str(_) => text(e).is_some(),
        ExprKind::List(items) => items.iter().all(plain),
        ExprKind::Field { base, .. } => matches!(base.kind, ExprKind::Name(_) | ExprKind::Int(_)),
        ExprKind::Call { callee, args, .. } => {
            matches!(callee.kind, ExprKind::Name(_)) && args.iter().all(|a| plain(&a.value))
        }
        _ => false,
    }
}

/// Each `const` set to a value, by its definition.
struct Consts<'c, 'r> {
    checker: &'c Checker<'r>,
    found: HashMap<DefId, Expr>,
}

impl VisitMut for Consts<'_, '_> {
    fn stmt(&mut self, s: &mut Stmt) {
        if let StmtKind::Let {
            mutable: false,
            pattern,
            value,
            ..
        } = &s.kind
            && let PatternKind::Binding(id) = &pattern.kind
            && let Some(d) = self.checker.decl.get(&id.span.start)
        {
            self.found.insert(*d, value.clone());
        }
        walk_stmt(self, s);
    }
}

/// Each `ask`, `ai.ask_full` or `ai.count` with a `model:`: the call's
/// span, and the model's value.
struct Asks {
    found: Vec<((u32, u32), Expr)>,
}

impl VisitMut for Asks {
    fn expr(&mut self, e: &mut Expr) {
        // `ask<T>(…)`, `ai.ask_full<T>(…)` or `ai.count(…)` (spec 8.4).
        let asks = |callee: &Expr| match &callee.kind {
            ExprKind::Name(f) => f.name == "ask",
            ExprKind::Field { base, name, .. } => {
                matches!(name.name.as_str(), "ask_full" | "count")
                    && matches!(base.kind, ExprKind::Name(_))
            }
            _ => false,
        };
        if let ExprKind::Call { callee, args, .. } = &e.kind
            && asks(callee)
        {
            let model = args
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|n| n.name == "model"));
            if let Some(m) = model {
                self.found
                    .push(((e.span.start, e.span.end), m.value.clone()));
            }
        }
        kavach_ast::visit::walk_expr(self, e);
    }
}
