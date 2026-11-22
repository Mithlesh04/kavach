// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `ask<T>(prompt)` and `ai.ask_full<T>(prompt)` in native code (spec
//! 8.4). Each `T` gets a function
//! that asks through the running agent's guard when an agent that can use
//! `Ai` is running (`lower_guard.rs`), and directly otherwise. The model
//! call, the decoding and the retries are the runtime's
//! (`runtime/native/src/rt_ai.rs`); the schema is built here, when the
//! program is compiled, like the interpreter builds it.

use crate::ir::{self, Block, ExprKind as K, FuncId, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::lower_agents::{Called, Synth, can_call, expr, int, rt, when};
use crate::rt_op::RtOp;
use kavach_ast as ast;
use kavach_checker::Ty;
use kavach_runtime_core::json::{self, Tree};

impl<'a> Lowerer<'a> {
    /// `ask<T>(prompt)`: a call of the function that asks for a `T`.
    pub(crate) fn ask_call(
        &mut self,
        span: ast::Span,
        prompt: &'a ast::Expr,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let Some(Ty::Result(ok, err)) = self.c.expr_types.get(&(span.start, span.end)).cloned()
        else {
            return Err(self.unsupported("this `ask`", span));
        };
        let ret = self.kir_type(&ok, span)?;
        let model = self.c.ask_models.get(&(span.start, span.end)).cloned();
        let key = (ret, format!("{model:?}"));
        let fid = match self.agents.asks.get(&key) {
            Some(f) => *f,
            None => {
                let fid = self.fns.len() as FuncId;
                self.fns.push((None, None));
                let (ok, err) = (*ok, *err);
                let synth = Synth::Ask {
                    ok,
                    err,
                    span,
                    model: model.map(Box::new),
                };
                self.agents.synth.insert(fid, synth);
                self.agents.asks.insert(key, fid);
                fid
            }
        };
        let prompt = self.expr(prompt)?;
        Ok(expr(K::Call(fid, vec![prompt]), ret, line))
    }

    /// The function that asks for a `T`: through the running agent's
    /// guard, or straight to the model when no agent is running.
    pub(crate) fn ask_fn(
        &mut self,
        ok: &Ty,
        err: &Ty,
        span: ast::Span,
        model: Option<kavach_driver::AiModel>,
    ) -> Lowered<ir::Function> {
        let ret = self.kir_type(ok, span)?;
        let err_ty = self.kir_type(err, span)?;
        // The runtime makes its errors as `AiError` values.
        self.ai_error_type(span)?;
        self.start_function(ret);
        let line = self.line(span);
        let text = self.str_type();
        let prompt = self.new_local(text);
        let mut body = Block::default();
        for (kind, (agent, adecl)) in self.agents.list.clone().into_iter().enumerate() {
            if !can_call(adecl, "Ai.ask") {
                continue;
            }
            let agent_ty = self.agent_type(agent, span)?;
            let some = self.optional_type(agent_ty);
            let current = self.new_local(some);
            let found = rt(RtOp::AgentCurrent, vec![int(kind, line)], some, line);
            body.stmts.push(Stmt::Let(current, found));
            let called = Called::Ask {
                ok: ok.clone(),
                err: err.clone(),
                span,
                model: model.clone().map(Box::new),
            };
            let guarded = self.guarded_id(agent, called, ret);
            let running = expr(
                K::Unwrap(Box::new(self.local_expr(current, line))),
                agent_ty,
                line,
            );
            let call = expr(
                K::Call(guarded, vec![running, self.local_expr(prompt, line)]),
                ret,
                line,
            );
            let mut then = self.pass_on(call, ret, err_ty, line);
            let back = (ret != Type::Unit).then(|| self.local_expr(then.1, line));
            then.0.push(Stmt::Return(back));
            let present = expr(
                K::IsSome(Box::new(self.local_expr(current, line))),
                Type::Bool,
                line,
            );
            body.stmts.push(when(present, then.0, line));
        }
        // No agent running: the call's own model, or the default.
        let chosen = model.unwrap_or_else(|| self.c.ask_model((u32::MAX, u32::MAX), None));
        let asked = self.model_call(ok, prompt, ret, span, &chosen)?;
        let (stmts, value) = self.pass_on(asked, ret, err_ty, line);
        body.stmts.extend(stmts);
        body.value = (ret != Type::Unit).then(|| Box::new(self.local_expr(value, line)));
        Ok(ir::Function {
            name: "ask".into(),
            params: vec![prompt],
            ret,
            locals: self.locals.clone(),
            body,
            line,
            fails: true,
            external: None,
            variadic: false,
        })
    }

    /// Runs `call`, which can fail: its error is failed with; gives the
    /// statements and the local holding its value.
    fn pass_on(
        &mut self,
        call: ir::Expr,
        ret: Type,
        err: Type,
        line: u32,
    ) -> (Vec<Stmt>, ir::Local) {
        let (value, error) = (self.new_local(ret), self.new_local(err));
        let fail = expr(
            K::Fail(Box::new(self.local_expr(error, line))),
            Type::Unit,
            line,
        );
        let stmts = vec![
            Stmt::LetResult { value, error, call },
            when(self.failed(error, line), vec![Stmt::Expr(fail)], line),
        ];
        (stmts, value)
    }

    /// The runtime's call of `model`, and of each model tried after it,
    /// with the text in local `prompt`, for an answer of type `ok`.
    pub(crate) fn model_call(
        &mut self,
        ok: &Ty,
        prompt: ir::Local,
        ret: Type,
        span: ast::Span,
        model: &kavach_driver::AiModel,
    ) -> Lowered<ir::Expr> {
        // `ai.ask_full<T>`: `ok` is `Asked<T>`, and the model answers a `T`.
        let full = self.c.full_asks.contains(&(span.start, span.end));
        let answer = match (full, ok) {
            (true, Ty::Adt(_, args)) => args.first().cloned().unwrap_or(Ty::Error),
            _ => ok.clone(),
        };
        let Some((schema, wrapped)) = kavach_driver::ask_schema(self.c, &answer) else {
            return Err(self.unsupported(kavach_driver::NO_SCHEMA, span));
        };
        // Each model's settings, with its price, in the order they are
        // tried (spec 8.4, *Fallbacks*), as the interpreter sends them.
        let models = model.in_turn().map(|m| self.c.priced_settings(m));
        let fields = vec![
            ("models".into(), Tree::Array(models.collect())),
            ("schema".into(), schema),
            ("wrapped".into(), Tree::Bool(wrapped)),
            ("full".into(), Tree::Bool(full)),
        ];
        let request = json::write(&Tree::Object(fields), 0);
        // The runtime reads it back when `ask` runs; one it couldn't read
        // would go out with no model and no shape, so it stops the build
        // here instead (N6).
        if let Err(e) = json::parse(&request, &json::Limits::default()) {
            let why = format!("this `ask`'s request can't be built ({e:?}): please report it");
            return Err(self.unsupported(why, span));
        }
        let line = self.line(span);
        let text = self.str_type();
        // The prompt, and the functions the runtime translates with: each
        // model's provider through `std.ai`'s `wire` (decision 0140).
        let mut given = vec![self.local_expr(prompt, line)];
        for m in model.in_turn() {
            given.push(self.provider_wire(m, span)?);
        }
        let pair_ty = self.tuple_type(given.iter().map(|e| e.ty).collect());
        let pair = expr(K::Tuple(given), pair_ty, line);
        let request = expr(K::Str(request), text, line);
        let templates = self.templates(ret, line, span)?;
        Ok(rt(RtOp::AiAsk, vec![pair, request, templates], ret, line))
    }

    /// `ai.count(prompt)` (spec 8.4): the runtime's count, with the model
    /// of each agent that can use `Ai`, chosen by the agent running when
    /// it is called, then the model for no agent; or only the call's own
    /// `model:`. A list counts with its first model.
    pub(crate) fn count_call(
        &mut self,
        span: ast::Span,
        prompt: &'a ast::Expr,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let Some(Ty::Result(ok, _)) = self.c.expr_types.get(&(span.start, span.end)).cloned()
        else {
            return Err(self.unsupported("this `ai.count`", span));
        };
        let ret = self.kir_type(&ok, span)?;
        // The runtime makes its errors as `AiError` values.
        self.ai_error_type(span)?;
        let call = (span.start, span.end);
        let (mut kinds, mut models) = (Vec::new(), Vec::new());
        if !self.c.ask_models.contains_key(&call) {
            for (kind, (agent, adecl)) in self.agents.list.clone().into_iter().enumerate() {
                if can_call(adecl, "Ai.count") {
                    let name = self.c.resolution.defs[agent].name.clone();
                    kinds.push(Tree::Int(kind as i64));
                    models.push(self.c.ask_model(call, Some(&name)));
                }
            }
        }
        kinds.push(Tree::Int(-1));
        models.push(self.c.ask_model(call, None));
        let settings = models.iter().map(kavach_driver::model_settings);
        let fields = vec![
            ("kinds".into(), Tree::Array(kinds)),
            ("models".into(), Tree::Array(settings.collect())),
        ];
        let request = json::write(&Tree::Object(fields), 0);
        let mut given = vec![self.expr(prompt)?];
        for m in &models {
            given.push(self.provider_wire(m, span)?);
        }
        let pair_ty = self.tuple_type(given.iter().map(|e| e.ty).collect());
        let pair = expr(K::Tuple(given), pair_ty, line);
        let request = expr(K::Str(request), self.str_type(), line);
        let templates = self.templates(ret, line, span)?;
        Ok(rt(RtOp::AiCount, vec![pair, request, templates], ret, line))
    }

    /// `wire(provider, settings)` from `std.ai`: a `fn(String) -> String`
    /// that translates with `model`'s provider, `none` giving
    /// `Anthropic()`. The provider's expression is a constructor written
    /// with plain values (spec 8.4), so it is lowered here as it is.
    fn provider_wire(
        &mut self,
        model: &kavach_driver::AiModel,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let c = self.c;
        let def = c.resolution.builtin_fn("std.ai", "wire");
        let found = def.and_then(|d| self.fn_ids.get(&d).copied());
        let decl = found
            .and_then(|f| self.fns.get(f as usize))
            .and_then(|(d, _)| *d);
        let (Some(fid), Some(decl)) = (found, decl) else {
            return Err(self.unsupported("`ask` without `std.ai`'s `wire`", span));
        };
        let line = self.line(span);
        let Some(given) = decl.params.first().and_then(|p| p.ty.as_ref()) else {
            return Err(self.unsupported("`wire`'s provider", span));
        };
        let given = self.written_type(given)?;
        let Some(crate::ir::Shape::Optional(iface)) = self.shapes.shape(given).cloned() else {
            return Err(self.unsupported("`wire`'s provider", span));
        };
        let provider = match model
            .provider
            .as_ref()
            .and_then(|(at, _)| c.ai_providers.get(at))
        {
            Some(e) => {
                let value = self.expr(e)?;
                let boxed = self.coerce(value, iface)?;
                self.coerce(boxed, given)?
            }
            None => expr(K::None, given, line),
        };
        let Some(ret) = decl.ret.as_ref() else {
            return Err(self.unsupported("`wire`'s result", span));
        };
        let wire_ty = self.written_type(ret)?;
        let settings = json::write(&kavach_driver::model_settings(model), 0);
        let settings = expr(K::Str(settings), self.str_type(), line);
        Ok(expr(K::Call(fid, vec![provider, settings]), wire_ty, line))
    }
}

impl Lowerer<'_> {
    /// The text the runtime reads an `ask`'s models' prices from, for a
    /// `cost` limit (spec 8.3): each one's name, `max_tokens` and price.
    pub(crate) fn ask_prices(&mut self, model: &kavach_driver::AiModel, line: u32) -> ir::Expr {
        let each = model.in_turn().map(|m| {
            let most = m.max_tokens.unwrap_or(kavach_driver::DEFAULT_MAX_TOKENS);
            let price = self.c.price_of(m);
            Tree::Object(vec![
                ("model".into(), Tree::Text(m.name.clone())),
                ("max_tokens".into(), Tree::Int(i64::from(most))),
                ("price".into(), price.cloned().unwrap_or(Tree::Null)),
            ])
        });
        let text = json::write(&Tree::Array(each.collect()), 0);
        expr(K::Str(text), self.str_type(), line)
    }

    /// `AiError` from `std.ai`, the error `ask` fails with.
    pub(crate) fn ai_error_type(&mut self, span: ast::Span) -> Lowered<Type> {
        let res = &self.c.resolution;
        let def = res
            .defs
            .iter()
            .position(|d| d.name == "AiError" && d.kind == kavach_resolver::DefKind::Type);
        match def {
            Some(def) => self.adt_instance(def, Vec::new(), span),
            None => Err(self.unsupported("`ask` without `std.ai`", span)),
        }
    }
}
