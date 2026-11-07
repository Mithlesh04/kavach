//! An agent's guarded call of a capability method in native code (spec
//! 8.3, "The order of the checks"): the limits, then who must approve,
//! then the implementation, with the audit log written by the runtime
//! (`runtime/native/src/rt_agents.rs`). The rules are compiled like any
//! code, with the method's parameters bound to the call's arguments;
//! `sum(…)` and `count` ask the runtime for the window's totals.

use crate::ir::{self, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer, Owner};
use crate::lower_agents::{Called, expr, int, not, rt, target_of, when};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, AgentMember};
use kavach_resolver::{DefId, DefKind};
use kavach_runtime_core::agents::covers;

impl<'a> Lowerer<'a> {
    pub(crate) fn guarded_fn(&mut self, agent: DefId, called: Called) -> Lowered<ir::Function> {
        let span = match &called {
            Called::Cap { cap, .. } => self.def_span(*cap),
            Called::Ask { span, .. } => *span,
        };
        let agent_ty = self.agent_type(agent, span)?;
        let (call, number, ret, err, line, inputs) = self.called(&called, span)?;
        self.start_function(ret);
        let a = self.new_local(agent_ty);
        let mut params = vec![a];
        let mut named = Vec::new();
        for (name, ty) in inputs {
            let local = self.new_local(ty);
            params.push(local);
            named.push((name, local));
        }
        let mut stmts = Vec::new();
        // A durable agent's event run again: the journal may have this
        // call's outcome already (spec 8.5).
        let durable = self.journal_of(agent).is_some();
        if durable {
            stmts.extend(self.journal_replay(a, &call, (ret, err), line, span)?);
        }

        // The check of this call, with its arguments for the audit log.
        let args: Vec<ir::Expr> = named
            .iter()
            .map(|(_, l)| self.local_expr(*l, line))
            .collect();
        let tuple_ty = self.tuple_type(args.iter().map(|e| e.ty).collect());
        let handle = expr(
            K::GetField(Box::new(self.local_expr(a, line)), 0),
            Type::I64,
            line,
        );
        let check = self.new_local(Type::I64);
        let begin = vec![
            handle,
            int(number, line),
            expr(K::Tuple(args), tuple_ty, line),
        ];
        stmts.push(Stmt::Let(
            check,
            rt(RtOp::AgentBegin, begin, Type::I64, line),
        ));
        let refused = self.new_local(Type::Bool);
        stmts.push(Stmt::Let(refused, expr(K::Bool(false), Type::Bool, line)));
        let needed = self.new_local(Type::Bool);
        stmts.push(Stmt::Let(needed, expr(K::Bool(false), Type::Bool, line)));
        let set = |local, value: ir::Expr| {
            Stmt::Expr(expr(K::Assign(local, Box::new(value)), Type::Unit, line))
        };

        // Step 3, the limits; step 4, the first approval whose condition holds.
        let decl_agent = self.agent_decl(agent);
        let (mut rule, mut approval) = (0, 0);
        for m in &decl_agent.members {
            match m {
                AgentMember::Limit { target, rules } => {
                    for r in rules {
                        if covers(&target_of(target), &call) {
                            self.bind_rule_params(&r.expr, &named);
                            self.agents.rule = Some((check, rule));
                            let ok = self.expr(&r.expr);
                            self.agents.rule = None;
                            let report = vec![self.local_expr(check, line), int(rule, line), ok?];
                            let refuses = rt(RtOp::AgentLimit, report, Type::Bool, line);
                            let open = not(self.local_expr(refused, line));
                            stmts.push(when(open, vec![set(refused, refuses)], line));
                        }
                        rule += 1;
                    }
                }
                AgentMember::Approve {
                    target,
                    when: cond,
                    by,
                    within,
                } => {
                    if covers(&target_of(target), &call) {
                        self.bind_rule_params(cond, &named);
                        let holds = self.expr(cond)?;
                        let by = self.expr(by)?;
                        let open = expr(
                            K::And(
                                Box::new(not(self.local_expr(refused, line))),
                                Box::new(not(self.local_expr(needed, line))),
                            ),
                            Type::Bool,
                            line,
                        );
                        let cond = expr(K::And(Box::new(open), Box::new(holds)), Type::Bool, line);
                        let need = vec![self.local_expr(check, line), int(approval, line), by];
                        let mut then = Vec::new();
                        // `within d`: `d.ms`, the field of a `Duration`.
                        if let Some(w) = within {
                            let d = self.expr(w)?;
                            let ms = expr(K::GetField(Box::new(d), 0), Type::I64, line);
                            let given = vec![self.local_expr(check, line), ms];
                            then.push(Stmt::Expr(rt(RtOp::AgentWithin, given, Type::Unit, line)));
                        }
                        then.push(Stmt::Expr(rt(RtOp::AgentNeed, need, Type::Unit, line)));
                        then.push(set(needed, expr(K::Bool(true), Type::Bool, line)));
                        stmts.push(when(cond, then, line));
                    }
                    approval += 1;
                }
                _ => {}
            }
        }
        let decide = rt(
            RtOp::AgentDecide,
            vec![self.local_expr(check, line)],
            Type::Bool,
            line,
        );
        let open = not(self.local_expr(refused, line));
        stmts.push(when(open, vec![set(refused, decide)], line));

        // Refused: `Denied(reason)`.
        let text = self.str_type();
        let reason = self.new_local(text);
        let why = rt(
            RtOp::AgentReason,
            vec![self.local_expr(check, line)],
            text,
            line,
        );
        let reason_value = self.local_expr(reason, line);
        // `ask` fails with `Error`; its refusals are `AiError.Denied` (spec 8.4).
        let refusal = match &called {
            Called::Ask { .. } => self.ai_error_type(span)?,
            Called::Cap { .. } => err,
        };
        let denied = self.denied(refusal, reason_value, line)?;
        let d = self.new_local(refusal);
        let done = vec![self.local_expr(check, line), int(0, line), int(0, line)];
        let mut then = vec![Stmt::Let(reason, why), Stmt::Let(d, denied)];
        if durable {
            let refused = self.local_expr(d, line);
            then.push(self.journal_record(a, false, refused, line));
        }
        then.extend([
            Stmt::Expr(rt(RtOp::AgentDone, done, Type::Unit, line)),
            Stmt::Expr(expr(
                K::Fail(Box::new(self.local_expr(d, line))),
                Type::Unit,
                line,
            )),
        ]);
        stmts.push(when(self.local_expr(refused, line), then, line));

        // Step 5: the implementation the agent was started with.
        let (value, error) = (self.new_local(ret), self.new_local(err));
        let run = match called {
            Called::Cap { cap, method } => {
                self.implementation(a, agent, cap, &method, &named, value, error, line)?
            }
            Called::Ask { ok, model, .. } => {
                // The call's own model, else the agent's (decision 0056).
                let name = self.c.resolution.defs[agent].name.clone();
                let chosen = model.map_or_else(
                    || self.c.ask_model((u32::MAX, u32::MAX), Some(&name)),
                    |m| *m,
                );
                let call = self.model_call(&ok, named[0].1, ret, span, &chosen)?;
                Stmt::LetResult { value, error, call }
            }
        };
        stmts.push(run);
        let tuple_err = self.tuple_type(vec![err]);
        let failed = vec![
            self.local_expr(check, line),
            int(2, line),
            expr(
                K::Tuple(vec![self.local_expr(error, line)]),
                tuple_err,
                line,
            ),
        ];
        let fail = expr(
            K::Fail(Box::new(self.local_expr(error, line))),
            Type::Unit,
            line,
        );
        let mut then = Vec::new();
        if durable {
            let e = self.local_expr(error, line);
            then.push(self.journal_record(a, false, e, line));
        }
        then.extend([
            Stmt::Expr(rt(RtOp::AgentDone, failed, Type::Unit, line)),
            Stmt::Expr(fail),
        ]);
        stmts.push(when(self.failed(error, line), then, line));
        let tuple_ok = self.tuple_type(vec![ret]);
        let ran = vec![
            self.local_expr(check, line),
            int(1, line),
            expr(K::Tuple(vec![self.local_expr(value, line)]), tuple_ok, line),
        ];
        if durable {
            let v = self.local_expr(value, line);
            stmts.push(self.journal_record(a, true, v, line));
        }
        stmts.push(Stmt::Expr(rt(RtOp::AgentDone, ran, Type::Unit, line)));
        let value = (ret != Type::Unit).then(|| Box::new(self.local_expr(value, line)));
        Ok(ir::Function {
            name: format!("{}: {call}", decl_agent.name.name),
            params,
            ret,
            locals: self.locals.clone(),
            body: Block { stmts, value },
            line,
            fails: true,
            external: None,
        })
    }

    /// What a guarded call is: its name and number for the runtime, its
    /// result and error types, its line, and its parameters.
    #[allow(clippy::type_complexity)]
    fn called(
        &mut self,
        called: &Called,
        span: ast::Span,
    ) -> Lowered<(String, usize, Type, Type, u32, Vec<(String, Type)>)> {
        match called {
            Called::Cap { cap, method } => {
                let decl = self.cap_method(*cap, method, span)?;
                let (ret, err) = self.signature(decl)?;
                let mut inputs = Vec::new();
                for p in &decl.params {
                    let Some(t) = &p.ty else {
                        return Err(self.unsupported("this capability", p.name.span));
                    };
                    inputs.push((p.name.name.clone(), self.written_type(t)?));
                }
                let methods = self.cap_methods();
                let number = methods
                    .iter()
                    .position(|(c, f)| c == cap && f.name.name == *method);
                let call = format!("{}.{method}", self.c.resolution.defs[*cap].name);
                let line = self.line(decl.name.span);
                Ok((call, number.unwrap_or(0), ret, err, line, inputs))
            }
            Called::Ask { ok, err, .. } => {
                let (ret, err) = (self.kir_type(ok, span)?, self.kir_type(err, span)?);
                let text = self.str_type();
                let number = self.cap_methods().len();
                let line = self.line(span);
                Ok((
                    "Ai.ask".into(),
                    number,
                    ret,
                    err,
                    line,
                    vec![("prompt".into(), text)],
                ))
            }
        }
    }

    /// Calls `method` on the implementation of `cap` held by the agent in
    /// local `a`: whichever type it is.
    #[allow(clippy::too_many_arguments)]
    fn implementation(
        &mut self,
        a: ir::Local,
        agent: DefId,
        cap: DefId,
        method: &str,
        named: &[(String, ir::Local)],
        value: ir::Local,
        error: ir::Local,
        line: u32,
    ) -> Lowered<Stmt> {
        let agent_ty = self.locals[a as usize];
        let cap_name = &self.c.resolution.defs[cap].name;
        let (field, slot_ty, variants) = match self.shapes.shape(agent_ty) {
            Some(Shape::Record { fields, .. }) => {
                let i = fields.iter().position(|f| f.name == *cap_name).unwrap_or(0);
                let variants = match self.shapes.shape(fields[i].ty) {
                    Some(Shape::Union { variants, .. }) => variants.len(),
                    _ => 0,
                };
                (i as u32, fields[i].ty, variants)
            }
            _ => (0, Type::Unit, 0),
        };
        let impls = self.impls(cap);
        let span = self.def_span(agent);
        if variants == 0 || impls.len() != variants {
            return Err(self.unsupported("this agent's implementations", span));
        }
        let slot = expr(
            K::GetField(Box::new(self.local_expr(a, line)), field),
            slot_ty,
            line,
        );
        let mut chain: Option<Block> = None;
        for (k, def) in impls.iter().enumerate().rev() {
            let Some(&fid) = self.method_ids.get(&(Owner::Adt(*def), method.to_string())) else {
                return Err(self.unsupported("this implementation", span));
            };
            let it_ty = self.adt_instance(*def, Vec::new(), span)?;
            let it = expr(K::GetField(Box::new(slot.clone()), 0), it_ty, line);
            let mut values = vec![it];
            values.extend(named.iter().map(|(_, l)| self.local_expr(*l, line)));
            let call = expr(K::Call(fid, values), self.locals[value as usize], line);
            let run = if self.decl_of(fid).errors.is_some() {
                Stmt::LetResult { value, error, call }
            } else {
                Stmt::Let(value, call)
            };
            let this = Block {
                stmts: vec![run],
                value: None,
            };
            chain = Some(match chain {
                None => this,
                Some(rest) => {
                    let is_k = expr(
                        K::HasVariant(Box::new(slot.clone()), k as u32),
                        Type::Bool,
                        line,
                    );
                    let choice = expr(K::If(Box::new(is_k), this, rest), Type::Unit, line);
                    Block {
                        stmts: vec![Stmt::Expr(choice)],
                        value: None,
                    }
                }
            });
        }
        Ok(Stmt::Expr(expr(
            K::Block(chain.unwrap_or_default()),
            Type::Unit,
            line,
        )))
    }

    /// Binds the parameters a rule names (as the resolver defined them) to
    /// the locals of the method's parameters with the same names.
    fn bind_rule_params(&mut self, e: &ast::Expr, named: &[(String, ir::Local)]) {
        let res = &self.c.resolution;
        for (&start, &def) in &res.uses {
            let d = &res.defs[def];
            let inside = e.span.start <= start && start < e.span.end;
            if inside
                && d.kind == DefKind::Param
                && d.span.is_none()
                && let Some((_, l)) = named.iter().find(|(n, _)| *n == d.name)
            {
                self.local_ids.insert(def, *l);
            }
        }
    }

    /// `sum(x)` in a limit rule: the window's total with `x` added.
    pub(crate) fn rule_sum(
        &mut self,
        arg: &'a ast::Expr,
        ty: Type,
        line: u32,
    ) -> Lowered<Option<ir::Expr>> {
        let Some((check, rule)) = self.agents.rule else {
            return Ok(None);
        };
        let x = self.expr(arg)?;
        let args = vec![self.local_expr(check, line), int(rule, line), x];
        Ok(Some(rt(RtOp::AgentSum, args, ty, line)))
    }

    /// `count` in a limit rule: the window's calls, this one too.
    pub(crate) fn rule_count(&self, line: u32) -> Option<ir::Expr> {
        let (check, rule) = self.agents.rule?;
        let args = vec![self.local_expr(check, line), int(rule, line)];
        Some(rt(RtOp::AgentCount, args, Type::I64, line))
    }

    /// `tokens` in a `limit Ai` rule: what the window's earlier asks took.
    pub(crate) fn rule_tokens(&self, line: u32) -> Option<ir::Expr> {
        let (check, rule) = self.agents.rule?;
        let args = vec![self.local_expr(check, line), int(rule, line)];
        Some(rt(RtOp::AgentTokens, args, Type::I64, line))
    }
}
