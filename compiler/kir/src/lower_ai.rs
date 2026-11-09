//! `ask<T>(prompt)` in native code (spec 8.4). Each `T` gets a function
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
                    model,
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

    /// The runtime's call of `model` with the text in local `prompt`, for
    /// an answer of type `ok`.
    pub(crate) fn model_call(
        &mut self,
        ok: &Ty,
        prompt: ir::Local,
        ret: Type,
        span: ast::Span,
        model: &kavach_driver::AiModel,
    ) -> Lowered<ir::Expr> {
        let Some((schema, wrapped)) = kavach_driver::ask_schema(self.c, ok) else {
            return Err(self.unsupported(kavach_driver::NO_SCHEMA, span));
        };
        // The model's settings, as the interpreter sends them.
        let Tree::Object(mut fields) = kavach_driver::model_settings(model) else {
            unreachable!("settings are an object");
        };
        fields.push(("schema".into(), schema));
        fields.push(("wrapped".into(), Tree::Bool(wrapped)));
        let request = Tree::Object(fields);
        let line = self.line(span);
        let text = self.str_type();
        let request = expr(K::Str(json::write(&request, 0)), text, line);
        let templates = self.templates(ret, line, span)?;
        let args = vec![self.local_expr(prompt, line), request, templates];
        Ok(rt(RtOp::AiAsk, args, ret, line))
    }
}

impl Lowerer<'_> {
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
