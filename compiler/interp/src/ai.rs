//! `ask<T>(prompt)` (spec 8.4): the model's answer as a checked `T`. The
//! answer's shape is a JSON Schema built from `T` (`kavach_driver::
//! ask_schema`); the reply is decoded like `json.decode<T>`, and asked
//! again (twice at most) when it doesn't fit. The call itself is
//! `kavach_runtime_net::ai`; waiting for it lets other tasks run.

use crate::calls::Args;
use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Expr;
use kavach_checker::{Prim, Ty};
use kavach_runtime_core::agents::Asked;
use kavach_runtime_core::json::{self, Tree};
use kavach_runtime_net::ai::{self, Outcome, Request, Tokens};
use std::time::Instant;

/// How many times an answer that doesn't fit `T` is asked for again.
const RETRIES: usize = 2;

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
        let mut check = None;
        if let Some(agent) = &agent {
            let args = vec![("prompt".to_string(), self.str_value(&prompt))];
            let c = self.check_call(agent, "Ai.ask", &shown, &args, span)?;
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
        // The call's own model, else its agent's (decision 0056).
        let named = agent.as_ref().map(|a| a.decl.name.name.as_str());
        let model = self.c.ask_model((span.start, span.end), named);
        let result = if self.dry_run || self.testing {
            let written = match &e.kind {
                kavach_ast::ExprKind::Call { generics, .. } => generics.first(),
                _ => None,
            };
            // Which model a real run would use, at no cost (spec 8.6).
            dry = Some(model.name.clone());
            Ok(self.simulate(written, &shown))
        } else {
            let (result, tokens) = self.ask_model(&t, &prompt, &model, span);
            used = Some((model.name, tokens));
            result
        };
        if let (Some(agent), Some(mut c)) = (&agent, check) {
            if self.dry_run {
                c.notes.push("dry run".into());
            }
            if let Some((_, t)) = &used {
                c.took(i64::try_from(t.input + t.output).unwrap_or(i64::MAX));
            }
            c.model = used
                .map(|(model, t)| Asked {
                    model,
                    tokens: Some((t.input, t.output)),
                })
                .or(dry.map(|model| Asked {
                    model,
                    tokens: None,
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

    /// The model call itself, retried while the answer doesn't fit `t`;
    /// with the tokens every try took.
    fn ask_model(
        &mut self,
        t: &Ty,
        prompt: &str,
        model: &kavach_driver::AiModel,
        span: kavach_ast::Span,
    ) -> (Eval<'a>, Tokens) {
        let mut used = Tokens::default();
        let result = self.ask_tries(t, prompt, model, span, &mut used);
        (result, used)
    }

    fn ask_tries(
        &mut self,
        t: &Ty,
        prompt: &str,
        model: &kavach_driver::AiModel,
        span: kavach_ast::Span,
        used: &mut Tokens,
    ) -> Eval<'a> {
        // Built where native code builds it (`kavach_driver::ask_schema`).
        let Some((schema, wrapped)) = kavach_driver::ask_schema(self.c, t) else {
            return Err(self.panic(kavach_driver::NO_SCHEMA, span));
        };
        let mut last = String::new();
        for attempt in 0..=RETRIES {
            // The type in the prompt first; then held to it (0057).
            let settings = kavach_driver::model_settings(model);
            let request = Request::new(&settings, prompt.to_string(), schema.clone(), attempt > 0);
            let pending = ai::start(request);
            let mut n = 0;
            let outcome = loop {
                if let Some(o) = pending.poll() {
                    break o;
                }
                let until = Instant::now() + kavach_runtime_net::net::backoff(n);
                self.sleep_until(until, span)?;
                n += 1;
            };
            let answer = match outcome {
                Outcome::NoKey(variable) => {
                    let v = vec![("variable", self.str_value(&variable))];
                    return Err(Flow::Fail(self.builtin_variant("AiError", "NoKey", v)));
                }
                Outcome::Failed { status, message } => {
                    let v = vec![
                        ("status", Value::Int(i128::from(status), Prim::Int)),
                        ("message", self.str_value(&message)),
                    ];
                    return Err(Flow::Fail(self.builtin_variant("AiError", "Failed", v)));
                }
                Outcome::Answer(tree, tokens) => {
                    used.input += tokens.input;
                    used.output += tokens.output;
                    tree
                }
            };
            let answer = if wrapped {
                field(&answer, "value")
            } else {
                answer
            };
            if let Ok(v) = self.decode(&answer, t, "") {
                return Ok(v);
            }
            last = json::write(&answer, 0);
        }
        let v = vec![("answer", self.str_value(&last))];
        Err(Flow::Fail(self.builtin_variant(
            "AiError",
            "InvalidOutput",
            v,
        )))
    }
}

/// Field `name` of an object, or `null`.
fn field(tree: &Tree, name: &str) -> Tree {
    match tree {
        Tree::Object(fields) => fields
            .iter()
            .find(|(k, _)| k == name)
            .map_or(Tree::Null, |(_, v)| v.clone()),
        _ => Tree::Null,
    }
}
