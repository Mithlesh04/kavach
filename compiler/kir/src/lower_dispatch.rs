//! Capability calls in native code (spec 8.2): each method has a
//! dispatcher, which finds the running agent and makes its guarded call
//! (`lower_guard.rs`), or fails with `Denied` when no agent that can make
//! the call is running.

use crate::ir::{self, Block, ExprKind as K, FuncId, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::lower_agents::{Called, Synth, can_call, expr, int, rt, when};
use crate::rt_op::RtOp;
use kavach_ast as ast;
use kavach_resolver::DefId;

impl<'a> Lowerer<'a> {
    /// `Payments.refund(…)`: a call of the dispatcher of that method.
    pub(crate) fn capability_call(
        &mut self,
        cap: DefId,
        name: &ast::Ident,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let key = (cap, name.name.clone());
        let fid = match self.agents.dispatchers.get(&key) {
            Some(f) => *f,
            None => {
                let decl = self.cap_method(cap, &name.name, span)?;
                let fid = self.fns.len() as FuncId;
                self.fns.push((Some(decl), None));
                let method = name.name.clone();
                self.agents
                    .synth
                    .insert(fid, Synth::Dispatch { cap, method });
                self.agents.dispatchers.insert(key, fid);
                fid
            }
        };
        let values = self.arguments(self.decl_of(fid), args, span)?;
        Ok(expr(K::Call(fid, values), ty, line))
    }

    pub(crate) fn cap_method(
        &self,
        cap: DefId,
        method: &str,
        span: ast::Span,
    ) -> Lowered<&'a ast::FnDecl> {
        self.cap_methods()
            .into_iter()
            .find(|(c, f)| *c == cap && f.name.name == method)
            .map(|(_, f)| f)
            .ok_or_else(|| self.unsupported("this capability call", span))
    }

    /// The function that makes `agent`'s guarded call, for results of
    /// type `ret`.
    pub(crate) fn guarded_id(&mut self, agent: DefId, called: Called, ret: Type) -> FuncId {
        let name = match &called {
            Called::Cap { cap, method } => {
                format!("{}.{method}", self.c.resolution.defs[*cap].name)
            }
            // One guarded call for each model a call names.
            Called::Ask { model, .. } => match model {
                Some(m) => format!("Ai.ask {m:?}"),
                None => "Ai.ask".to_string(),
            },
        };
        let key = (agent, name, ret);
        if let Some(f) = self.agents.guarded.get(&key) {
            return *f;
        }
        let fid = self.fns.len() as FuncId;
        self.fns.push((None, None));
        self.agents
            .synth
            .insert(fid, Synth::Guarded { agent, called });
        self.agents.guarded.insert(key, fid);
        fid
    }

    /// The dispatcher of `cap.method`: the running agent's guarded call,
    /// or `Denied` when no agent that can make it is running.
    pub(crate) fn dispatch_fn(&mut self, cap: DefId, method: &str) -> Lowered<ir::Function> {
        let cap_name = self.c.resolution.defs[cap].name.clone();
        let span = self.def_span(cap);
        let decl = self.cap_method(cap, method, span)?;
        let (ret, err) = self.signature(decl)?;
        self.start_function(ret);
        let line = self.line(decl.name.span);
        let mut params = Vec::new();
        for p in &decl.params {
            let ty = match &p.ty {
                Some(t) => self.written_type(t)?,
                None => return Err(self.unsupported("this capability", p.name.span)),
            };
            params.push(self.new_local(ty));
        }
        let call = format!("{cap_name}.{method}");
        let mut body = Block::default();
        for (kind, (agent, adecl)) in self.agents.list.clone().into_iter().enumerate() {
            if !can_call(adecl, &call) {
                continue;
            }
            let agent_ty = self.agent_type(agent, span)?;
            let some = self.optional_type(agent_ty);
            let current = self.new_local(some);
            let found = rt(RtOp::AgentCurrent, vec![int(kind, line)], some, line);
            body.stmts.push(Stmt::Let(current, found));
            let called = Called::Cap {
                cap,
                method: method.to_string(),
            };
            let guarded = self.guarded_id(agent, called, ret);
            let running = expr(
                K::Unwrap(Box::new(self.local_expr(current, line))),
                agent_ty,
                line,
            );
            let mut values = vec![running];
            values.extend(params.iter().map(|p| self.local_expr(*p, line)));
            let (value, error) = (self.new_local(ret), self.new_local(err));
            let call = expr(K::Call(guarded, values), ret, line);
            let fail = expr(
                K::Fail(Box::new(self.local_expr(error, line))),
                Type::Unit,
                line,
            );
            let back = (ret != Type::Unit).then(|| self.local_expr(value, line));
            let then = vec![
                Stmt::LetResult { value, error, call },
                when(self.failed(error, line), vec![Stmt::Expr(fail)], line),
                Stmt::Return(back),
            ];
            let present = expr(
                K::IsSome(Box::new(self.local_expr(current, line))),
                Type::Bool,
                line,
            );
            body.stmts.push(when(present, then, line));
        }
        let reason = expr(
            K::Str(format!("no implementation of {cap_name}")),
            self.str_type(),
            line,
        );
        let denied = self.denied(err, reason, line)?;
        body.stmts.push(Stmt::Expr(expr(
            K::Fail(Box::new(denied)),
            Type::Unit,
            line,
        )));
        Ok(ir::Function {
            name: call,
            params,
            ret,
            locals: self.locals.clone(),
            body,
            line,
            fails: true,
            external: None,
        })
    }

    /// A capability method's return and error types.
    pub(crate) fn signature(&mut self, f: &ast::FnDecl) -> Lowered<(Type, Type)> {
        let ret = match &f.ret {
            Some(t) => self.written_type(t)?,
            None => Type::Unit,
        };
        let err = match &f.errors {
            Some(t) => self.written_type(t)?,
            None => return Err(self.unsupported("a capability method without errors", f.name.span)),
        };
        Ok((ret, err))
    }

    /// `Denied(reason)` of error union `err` (spec 8.2).
    pub(crate) fn denied(&mut self, err: Type, reason: ir::Expr, line: u32) -> Lowered<ir::Expr> {
        let index = match self.shapes.shape(err) {
            Some(Shape::Union { variants, .. }) => variants.iter().position(|v| v.name == "Denied"),
            _ => None,
        };
        let Some(index) = index else {
            let feature = "an error type without `Denied`".into();
            return Err(crate::Unsupported { feature, line });
        };
        Ok(expr(
            K::Record {
                variant: index as u32,
                fields: vec![reason],
            },
            err,
            line,
        ))
    }
}
