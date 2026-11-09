//! The `Model` of each agent and of each `ask` that names one (spec 8.4,
//! decision 0056), worked out when the program is built: native code
//! builds each request then. A `Model(…)` written with plain values, or
//! a `const` set to one, can be; anything else is an error that says so.

use crate::checker::Checker;
use kavach_ast::visit::{VisitMut, walk_file, walk_stmt};
use kavach_ast::*;
use kavach_resolver::DefId;
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
}

/// Why the compiler refuses `m`, a model it knows something about.
fn refused(m: &AiModel) -> Option<String> {
    let name = &m.name;
    if m.thinking == Some(false) && name.starts_with("claude-opus-5-5") {
        return Some(format!(
            "`{name}` always thinks before it answers, so `thinking: false` would fail: leave it out, and lower `effort` for fewer tokens (spec 8.4)"
        ));
    }
    if let (Some(false), Some(e @ ("xhigh" | "max"))) = (m.thinking, m.effort.as_deref()) {
        let shown = if e == "xhigh" { "XHigh" } else { "Max" };
        return Some(format!(
            "`thinking: false` can't go with `effort: Effort.{shown}`: with thinking off, effort goes up to `Effort.High` (spec 8.4)"
        ));
    }
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

const NOT_KNOWN: &str = "a model is chosen when the program is built: write `Model(name: \"…\")` with plain values, or call a function with no parameters that gives one, such as `careful()` (spec 8.4)";

impl Checker<'_> {
    /// Each agent's model, by the agent's name, and each `ask`'s own, by
    /// the call's span.
    pub(crate) fn ai_models(
        &mut self,
        file: &File,
    ) -> (HashMap<String, AiModel>, HashMap<(u32, u32), AiModel>) {
        let consts = self.known_values(file);
        let mut agents = HashMap::new();
        let mut asks = HashMap::new();
        let mut wrong = Vec::new();
        // Each model worked out, with where it is written, to check.
        let mut checked = Vec::new();
        for item in &file.items {
            let ItemKind::Agent(a) = &item.kind else {
                continue;
            };
            for m in &a.members {
                if let AgentMember::Model(e) = m {
                    match self.model_of(e, &consts, 0) {
                        Some(model) => {
                            checked.push((e.span, model.clone()));
                            agents.insert(a.name.name.clone(), model);
                        }
                        None => wrong.push(e.span),
                    }
                }
            }
        }
        let mut calls = Asks { found: Vec::new() };
        walk_file(&mut calls, &mut file.clone());
        for (call, e) in calls.found {
            match self.model_of(&e, &consts, 0) {
                Some(model) => {
                    checked.push((e.span, model.clone()));
                    asks.insert(call, model);
                }
                None => wrong.push(e.span),
            }
        }
        for span in wrong {
            self.error(span, NOT_KNOWN);
        }
        for (span, model) in checked {
            if let Some(why) = refused(&model) {
                self.error(span, why);
            }
        }
        (agents, asks)
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

    /// The model `e` gives, if it can be worked out now.
    fn model_of(&self, e: &Expr, consts: &HashMap<DefId, Expr>, depth: u32) -> Option<AiModel> {
        match &e.kind {
            ExprKind::Name(id) if depth < 8 => {
                let d = self.res.uses.get(&id.span.start)?;
                self.model_of(consts.get(d)?, consts, depth + 1)
            }
            ExprKind::Call { callee, args, .. } => {
                let ExprKind::Name(f) = &callee.kind else {
                    return None;
                };
                if f.name != "Model" {
                    // `careful()`: a function that gives one.
                    let d = self.res.uses.get(&f.span.start)?;
                    return match (args.is_empty(), depth < 8) {
                        (true, true) => self.model_of(consts.get(d)?, consts, depth + 1),
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

/// Each `ask` with a `model:`: the call's span, and the model's value.
struct Asks {
    found: Vec<((u32, u32), Expr)>,
}

impl VisitMut for Asks {
    fn expr(&mut self, e: &mut Expr) {
        if let ExprKind::Call { callee, args, .. } = &e.kind
            && matches!(&callee.kind, ExprKind::Name(f) if f.name == "ask")
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
