// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `ask<T>(prompt)` and `ai.ask_full<T>(prompt)` (spec 8.4): the model's
//! answer as a checked `T`, alone or with what it took. The
//! answer's shape is a JSON Schema built from `T` (`kavach_driver::
//! ask_schema`); the reply is decoded like `json.decode<T>`, and asked
//! again (twice at most) when it doesn't fit. The call itself is
//! `kavach_runtime_net::ai`, which calls the model's provider, written in
//! Kavach (decision 0140); waiting for it lets other tasks run.

use crate::calls::Args;
use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Expr;
use kavach_checker::Ty;
use kavach_runtime_core::agents::costs::Price;
use kavach_runtime_core::agents::{Asked, largest_ask};
use kavach_runtime_core::json::{self, Tree};
use kavach_runtime_net::ai;
use std::time::Instant;

impl<'a> Interp<'a> {
    pub(crate) fn ask(&mut self, args: Args<'a>, e: &'a Expr) -> Eval<'a> {
        let span = e.span;
        self.refuse_if_held(span)?;
        let Some(Ty::Result(t, _)) = self.c.expr_types.get(&(span.start, span.end)).cloned() else {
            return Err(self.panic("`ask` needs the type of its answer: `ask<T>(prompt)`", span));
        };
        let prompt = match args.into_iter().next() {
            Some((_, Value::Str(s))) => s.to_string(),
            Some((_, v)) => v.display(),
            None => String::new(),
        };
        // Inside an agent, an AI call goes through its guard like any
        // capability call (spec 8.3): `limit Ai { count <= … }`.
        let agent = self.running.last().cloned();
        // An event run again after a crash: an answer the journal has is
        // given back, and the model is not asked again (spec 8.5).
        let ai_error = self
            .c
            .resolution
            .std_type("AiError")
            .map(|d| Ty::Adt(d, Vec::new()));
        if let Some(recorded) = self.journal_recorded("Ai.ask", Some((*t).clone()), ai_error, span)
        {
            return recorded;
        }
        let shown = format!("ask(\"{}\")", prompt.chars().take(60).collect::<String>());
        // The call's own model, else its agent's (decision 0056).
        let named = agent.as_ref().map(|a| a.decl.name.name.as_str());
        let model = self.c.ask_model((span.start, span.end), named);
        let mut check = None;
        if let Some(agent) = &agent {
            let args = vec![("prompt".to_string(), self.str_value(&prompt))];
            // The largest a model of the list may give (spec 8.4).
            let max_tokens = model.most_tokens(kavach_driver::DEFAULT_MAX_TOKENS);
            let max_tokens = i64::from(max_tokens);
            let bound = largest_ask(prompt.len(), max_tokens);
            // Each model's price, for a `cost` rule (spec 8.3).
            let priced: crate::guard::Priced = (model.in_turn())
                .map(|m| {
                    let most = m.max_tokens.unwrap_or(kavach_driver::DEFAULT_MAX_TOKENS);
                    let price = (self.c.price_of(m)).and_then(Price::read);
                    (m.name.clone(), i64::from(most), price)
                })
                .collect();
            let priced = Some((prompt.len(), &priced));
            let c = self.check_call(agent, ("Ai.ask", &shown, bound), priced, &args, span)?;
            if let Some(why) = c.refused.clone() {
                agent.guard.borrow().audit(&c, &format!("denied: {why}"));
                if self.dry_run && !self.testing {
                    self.err
                        .push_str(&format!("[dry run] {shown} refused: {why}\n"));
                }
                let v = vec![("reason", self.str_value(&why))];
                // Journaled as native code does, so a run after a crash
                // gives the same refusal instead of asking the limit again.
                let refused = Err(Flow::Fail(self.builtin_variant("AiError", "Denied", v)));
                self.journal_record("Ai.ask", &refused, span)?;
                return refused;
            }
            let mut c = c;
            // It counts from now (decision 0062); its tokens are added
            // once the answer is in.
            if !self.dry_run && !self.testing {
                agent.guard.borrow_mut().start(&mut c);
            }
            check = Some(c);
        }
        // A dry run, or a generated test (spec 11.5): a generated answer.
        let mut used = None;
        let mut dry = None;
        let result = if self.dry_run || self.testing {
            let written = match &e.kind {
                kavach_ast::ExprKind::Call { generics, .. } => generics.first(),
                _ => None,
            };
            // Which model a real run would use first, at no cost (spec 8.6).
            dry = Some(model.name.clone());
            Ok(self.simulate(written, &shown))
        } else {
            let (result, asked) = self.ask_model(&t, &prompt, &model, span);
            used = Some(asked);
            result
        };
        if let (Some(agent), Some(mut c)) = (&agent, check) {
            if self.dry_run {
                c.notes.push("dry run".into());
            }
            if let Some((i, o)) = used.as_ref().and_then(|a| a.tokens) {
                c.took(i64::try_from(i + o).unwrap_or(i64::MAX));
            }
            // A cost that isn't known counts as 0 (spec 8.3).
            if let Some(a) = &used {
                let zero = kavach_runtime_core::Decimal::from_int(0);
                c.cost(a.cost.as_ref().map_or(zero, |s| s.dollars));
            }
            c.model = used.or(dry.map(|model| Asked {
                model,
                ..Asked::default()
            }));
            let mut guard = agent.guard.borrow_mut();
            let shown_result = match &result {
                Ok(answer) => {
                    guard.record(&c);
                    format!("ok {}", answer.display())
                }
                Err(Flow::Fail(e)) => {
                    guard.unrecord(&c);
                    format!("error {}", e.display())
                }
                Err(_) => String::new(),
            };
            guard.audit(&c, &shown_result);
        }
        self.journal_record("Ai.ask", &result, span)?;
        result
    }

    /// The model call itself, through the runtime's driver
    /// (`kavach_runtime_net::ai::ask`), with each model's provider written
    /// in Kavach doing the translating (decision 0140); with the model of
    /// the last try, the tokens every try took, and the models moved on
    /// from (spec 8.4, *Fallbacks*), for the audit log.
    fn ask_model(
        &mut self,
        t: &Ty,
        prompt: &str,
        model: &kavach_driver::AiModel,
        span: kavach_ast::Span,
    ) -> (Eval<'a>, Asked) {
        let names: Vec<String> = model.in_turn().map(|m| m.name.clone()).collect();
        let mut shown = Asked {
            model: model.name.clone(),
            tokens: Some((0, 0)),
            ..Asked::default()
        };
        // `ai.ask_full<T>`: `t` is `Asked<T>`, and the model answers a `T`.
        let full = self.c.full_asks.contains(&(span.start, span.end));
        let answer = match (full, t) {
            (true, Ty::Adt(_, args)) => args.first().cloned().unwrap_or(Ty::Error),
            _ => t.clone(),
        };
        // Built where native code builds it (`kavach_driver::ask_schema`).
        let Some((schema, wrapped)) = kavach_driver::ask_schema(self.c, &answer) else {
            return (Err(self.panic(kavach_driver::NO_SCHEMA, span)), shown);
        };
        let mut settings = Vec::new();
        let mut wires = Vec::new();
        for m in model.in_turn() {
            let s = self.c.priced_settings(m);
            match self.provider_wire(m, &s, span) {
                Ok(w) => wires.push(w),
                Err(e) => return (Err(e), shown),
            }
            settings.push(s);
        }
        let mut side = Asking {
            i: self,
            wires,
            t: answer,
            span,
        };
        let asked = match ai::ask(&mut side, &settings, prompt, &schema, wrapped) {
            Ok(a) => a,
            Err(stop) => return (Err(stop), shown),
        };
        shown.model = names[asked.used].clone();
        shown.tokens = Some((asked.tokens.input, asked.tokens.output));
        shown.moved = (asked.moved.iter())
            .map(|(i, f)| (names[*i].clone(), f.variant()))
            .collect();
        shown.usage = asked.usage;
        shown.cost = asked.cost.clone();
        let result = match asked.result {
            // With what it took (spec 8.4, *Usage and cost*).
            Ok(v) if full => self.asked_value(v, &shown, t, span),
            Ok(v) => Ok(v),
            Err(f) => Err(self.ai_error(&f.tree(), span)),
        };
        (result, shown)
    }

    /// The function `std.ai`'s `wire` gives for `model`'s provider: what
    /// the runtime translates every request and reply with.
    pub(crate) fn provider_wire(
        &mut self,
        model: &kavach_driver::AiModel,
        settings: &Tree,
        span: kavach_ast::Span,
    ) -> Eval<'a> {
        let c = self.c;
        let wire = c.resolution.builtin_fn("std.ai", "wire");
        let Some(f) = wire.and_then(|d| self.fns.get(&d).copied()) else {
            return Err(self.panic("`ask` needs `std.ai`'s `wire`", span));
        };
        let provider = match model
            .provider
            .as_ref()
            .and_then(|(at, _)| c.ai_providers.get(at))
        {
            Some(e) => self.eval(e)?,
            None => Value::None,
        };
        let settings = self.str_value(&json::write(settings, 0));
        self.call_fn(f, None, vec![(None, provider), (None, settings)], span)
    }

    /// The `Asked<T>` (type `t`) of `ai.ask_full`: the answer `value`, and
    /// what the `ask` took, from the model that answered (spec 8.4).
    fn asked_value(
        &mut self,
        value: Value<'a>,
        asked: &Asked,
        t: &Ty,
        span: kavach_ast::Span,
    ) -> Eval<'a> {
        let std_ty = |c: &kavach_driver::Compiled, name| c.resolution.std_type(name);
        let (Ty::Adt(def, _), Some(usage), Some(cost)) =
            (t, std_ty(self.c, "Usage"), std_ty(self.c, "Cost"))
        else {
            return Err(self.panic("`ai.ask_full` needs `std.ai`'s `Asked`", span));
        };
        let usage = self.decode(&asked.usage.tree(), &Ty::Adt(usage, Vec::new()), "")?;
        let cost = match &asked.cost {
            Ok(spend) => self.decode(&spend.tree(), &Ty::Adt(cost, Vec::new()), "")?,
            Err(_) => Value::None,
        };
        let model = self.str_value(&asked.model);
        let field = |name: &str, value| crate::value::Field {
            name: std::rc::Rc::from(name),
            value,
            secret: false,
        };
        Ok(Value::Record {
            def: *def,
            name: std::rc::Rc::from("Asked"),
            fields: vec![
                field("value", value),
                field("usage", usage),
                field("cost", cost),
                field("model", model),
            ],
            life: None,
        })
    }

    /// `AiError` from its JSON (spec 2.12).
    pub(crate) fn ai_error(&mut self, tree: &Tree, span: kavach_ast::Span) -> Flow<'a> {
        let ty = self
            .c
            .resolution
            .std_type("AiError")
            .map(|d| Ty::Adt(d, Vec::new()));
        match ty.map(|ty| self.decode(tree, &ty, "")) {
            Some(Ok(v)) => Flow::Fail(v),
            _ => self.panic("`AiError` is missing from the program", span),
        }
    }
}

/// The interpreter's side of an `ask`, or of a count: the `wire`
/// function of each model's provider, and the type its answer must have.
pub(crate) struct Asking<'i, 'a> {
    pub(crate) i: &'i mut Interp<'a>,
    pub(crate) wires: Vec<Value<'a>>,
    pub(crate) t: Ty,
    pub(crate) span: kavach_ast::Span,
}

impl<'a> ai::Side for Asking<'_, 'a> {
    type Value = Value<'a>;
    type Stop = Flow<'a>;

    fn translate(&mut self, model: usize, input: &str) -> Result<String, Flow<'a>> {
        let arg = self.i.str_value(input);
        let wire = self.wires[model].clone();
        Ok(match self.i.call_value(wire, vec![arg], self.span)? {
            Value::Str(s) => s.to_string(),
            _ => "null".into(),
        })
    }

    fn wait_until(&mut self, until: Instant) -> Result<(), Flow<'a>> {
        self.i.sleep_until(until, self.span)
    }

    fn fits(&mut self, answer: &Tree) -> Option<Value<'a>> {
        self.i.decode(answer, &self.t, "").ok()
    }
}

mod count;
