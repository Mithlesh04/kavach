//! Agents (spec 8.2, 8.3, 8.8): `Agent.start(…)`, its handlers, and the
//! capability calls they make. A handler runs with its agent on a stack,
//! so every function it calls reaches the same implementations; a call
//! with no running agent, or none for that capability, is refused. Every
//! call goes through the agent's guard (`guard.rs`) first.

use crate::calls::Args;
use crate::guard::{Approval, RuleCode};
use crate::interp::Interp;
use crate::value::{Eval, Field, Flow, Value};
use kavach_ast::{AgentDecl, AgentMember, CapabilityDecl, FnDecl, ItemKind, Span, TypeKind};
use kavach_driver::AgentMode;
use kavach_runtime_core::agents::{Guard, Mode, Settings};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A started agent: its declaration, an implementation per capability,
/// its guard (`kavach_runtime_core::agents`), and the code of its limit
/// rules and approvals.
#[derive(Debug)]
pub(crate) struct Running<'a> {
    pub(crate) decl: &'a AgentDecl,
    pub(crate) impls: HashMap<String, Value<'a>>,
    pub(crate) guard: RefCell<Guard>,
    pub(crate) rules: Vec<RuleCode<'a>>,
    pub(crate) approvals: Vec<Approval<'a>>,
    /// The journal of a durable agent (spec 8.5).
    pub(crate) journal: Option<RefCell<kavach_runtime_core::journal::Journal>>,
    /// Events run again after a restart that went on inside the agent
    /// while they wait for a person (decision 0072): a send of the same
    /// event joins one.
    pub(crate) replays: RefCell<Vec<crate::journal::Replay<'a>>>,
    /// The tasks of this agent that wait for a person's answer now.
    pub(crate) asking: RefCell<Vec<kavach_runtime_core::tasks::TaskId>>,
}

impl<'a> Interp<'a> {
    /// `Bot.start(Payments: impl, …)`: the checker made sure every
    /// declared capability the agent can use has one. The mode, audit log,
    /// state and journal come from the agent's `settings` (decision 0060).
    pub(crate) fn start_agent(
        &mut self,
        decl: &'a AgentDecl,
        args: Args<'a>,
        span: Span,
    ) -> Eval<'a> {
        let impls = args
            .into_iter()
            .filter_map(|(name, v)| Some((name?.to_string(), v)))
            .collect();
        // Its settings in code, with the defaults worked out (decision
        // 0060): under `kavach test` or a dry run, no state or journal.
        let settings = self
            .c
            .agent_config(&decl.name.name, self.testing || self.dry_run);
        let (rules, code, approvals) = self.guards(&decl.members);
        // One program per agent (decision 0073): its folder is locked.
        for file in [&settings.state, &settings.journal].into_iter().flatten() {
            let locked = kavach_runtime_core::agents::lock_folder(file.as_ref());
            locked.map_err(|why| self.panic(why, span))?;
        }
        let journal = (settings.journal.as_ref())
            .map(|p| kavach_runtime_core::journal::Journal::open(p.into()));
        let settings = Settings {
            name: decl.name.name.clone(),
            mode: match settings.mode {
                AgentMode::Observe => Mode::Observe,
                AgentMode::Suggest => Mode::Suggest,
                AgentMode::Enforce => Mode::Enforce,
            },
            audit: settings.audit_log.map(Into::into),
            audit_json: settings.audit_format.as_deref() == Some("json"),
            otlp: self.audit_otlp(settings.audit_otlp),
            state: settings.state.map(Into::into),
            under_way: kavach_runtime_core::agents::UnderWay::named(&settings.under_way),
        };
        let (journal, unfinished) = match journal {
            Some((j, u)) => (Some(RefCell::new(j)), u),
            None => (None, Vec::new()),
        };
        let agent = Rc::new(Running {
            decl,
            impls,
            guard: RefCell::new(Guard::new(settings, rules)),
            rules: code,
            approvals,
            journal,
            replays: RefCell::default(),
            asking: RefCell::default(),
        });
        if let Some(j) = &agent.journal {
            agent.guard.borrow().set_aside(&j.borrow().set_aside);
        }
        // A durable agent first finishes what a crash left (spec 8.5).
        self.resume(&agent, unfinished, span)?;
        Ok(Value::Agent(agent))
    }

    /// `bot.request(order)`: the handler, run with the agent's
    /// implementations.
    pub(crate) fn agent_event(
        &mut self,
        agent: Rc<Running<'a>>,
        name: &str,
        args: Args<'a>,
        span: Span,
    ) -> Eval<'a> {
        let handler = agent.decl.members.iter().find_map(|m| match m {
            AgentMember::On(f) if f.name.name == name => Some(f),
            _ => None,
        });
        let Some(f) = handler else {
            let agent_name = &agent.decl.name.name;
            return Err(self.panic(
                format!("agent `{agent_name}` has no handler `{name}`"),
                span,
            ));
        };
        // The same event, run again after a restart, waits for a person:
        // this send waits for it, and no second request is made.
        if let Some(waiting) = self.waiting_replay(&agent, name, &args) {
            return self.wait_task(&waiting, None, span);
        }
        agent.guard.borrow_mut().next_request();
        self.running.push(Rc::clone(&agent));
        let queued = self.drops.borrow().len();
        let result = self.journaled_event(&agent, f, args, None, span);
        self.running.pop();
        // A handler that panics is stopped; the agent goes on serving, and
        // the send fails (5.6, decision 0076).
        match result {
            Err(Flow::Panic(message, at)) => {
                self.forget_drops(queued);
                let (path, line) = self.c.shown_location(at.start);
                self.err
                    .push_str(&format!("panic: {message}\n  at {path}:{line}\n"));
                let why = format!("panicked: {message} at {path}:{line}");
                let _ = agent.guard.borrow().note(&format!("{name} {why}"));
                Err(Flow::Fail(Value::BuiltinError(why.into())))
            }
            other => other,
        }
    }

    /// `Payments.refund(…)`: through the guard, then the running agent's
    /// implementation.
    pub(crate) fn capability_call(
        &mut self,
        cap: &'a CapabilityDecl,
        method: &str,
        args: Args<'a>,
        span: Span,
    ) -> Eval<'a> {
        let decl = cap.methods.iter().find_map(|m| match &m.kind {
            ItemKind::Fn(f) if f.name.name == method => Some(f),
            _ => None,
        });
        let Some(decl) = decl else {
            return Err(self.panic(
                format!("`{}` has no function `{method}`", cap.name.name),
                span,
            ));
        };
        let cap_name = cap.name.name.as_str();
        // In a generated test, outside an agent: a fake answer (11.5).
        if self.testing && self.running.is_empty() {
            return self.fake(decl);
        }
        if self.dry_run && self.running.is_empty() {
            let shown = shown_call(
                &format!("{cap_name}.{method}"),
                &named_args(decl, &args),
                decl,
            );
            return Ok(self.simulate(decl.ret.as_ref(), &shown));
        }
        let agent = self.running.last().cloned();
        let implementation = agent.as_ref().and_then(|a| a.impls.get(cap_name)).cloned();
        let (Some(agent), Some(implementation)) = (agent, implementation) else {
            return Err(self.denied(decl, &format!("no implementation of {cap_name}")));
        };
        let def = match &implementation {
            Value::Record { def, .. } => *def,
            Value::Variant { union, .. } => *union,
            _ => return Err(self.denied(decl, &format!("no implementation of {cap_name}"))),
        };
        let Some(f) = self.methods.get(&def).and_then(|m| m.get(method)).copied() else {
            return Err(self.panic(
                format!("the implementation of {cap_name} has no `{method}`"),
                span,
            ));
        };
        let named = named_args(decl, &args);
        let call = format!("{cap_name}.{method}");
        // An event run again after a crash: a call the journal has gives
        // its recorded outcome, without running (spec 8.5).
        let (ok_ty, err_ty) = (
            decl.ret.as_ref().and_then(|t| self.ty_of(t)),
            decl.errors.as_ref().and_then(|t| self.ty_of(t)),
        );
        if let Some(recorded) = self.journal_recorded(&call, ok_ty, err_ty, span) {
            return recorded;
        }
        let result = self.guarded_call(&agent, decl, f, implementation, args, named, &call, span);
        self.journal_record(&call, &result, span)?;
        result
    }

    /// The guard's checks, then the call, and the audit log.
    #[allow(clippy::too_many_arguments)]
    fn guarded_call(
        &mut self,
        agent: &Rc<Running<'a>>,
        decl: &'a FnDecl,
        f: &'a FnDecl,
        implementation: Value<'a>,
        args: Args<'a>,
        named: Vec<(String, Value<'a>)>,
        call: &str,
        span: Span,
    ) -> Eval<'a> {
        let shown = shown_call(call, &named, decl);
        let mut check = self.check_call(agent, (call, &shown, 0), &named, span)?;
        if let Some(why) = check.refused.clone() {
            agent
                .guard
                .borrow()
                .audit(&check, &format!("denied: {why}"));
            if self.dry_run && !self.testing {
                self.err
                    .push_str(&format!("[dry run] {shown} refused: {why}\n"));
            }
            return Err(self.denied(decl, &why));
        }
        if self.dry_run {
            let v = self.simulate(decl.ret.as_ref(), &shown);
            check.notes.push("dry run".into());
            let mut guard = agent.guard.borrow_mut();
            guard.record(&check);
            guard.audit(&check, &format!("simulated {}", v.display()));
            return Ok(v);
        }
        // It counts from now, so a crash while it runs can't leave it
        // uncounted (decision 0062).
        agent.guard.borrow_mut().start(&mut check);
        self.in_call += 1;
        let result = self.call_fn(f, Some(implementation), args, span);
        self.in_call -= 1;
        match &result {
            Ok(v) => {
                let shown_result = if *v == Value::Unit {
                    "ok".to_string()
                } else {
                    format!("ok {}", v.display())
                };
                let mut guard = agent.guard.borrow_mut();
                guard.record(&check);
                guard.audit(&check, &shown_result);
            }
            Err(Flow::Fail(e)) => {
                let shown_result = format!("error {}", e.display());
                let mut guard = agent.guard.borrow_mut();
                guard.unrecord(&check);
                guard.audit(&check, &shown_result);
            }
            Err(_) => {}
        }
        result
    }

    /// A capability's fake in a generated test (spec 11.5): one call in
    /// three fails with one of the method's errors, the others give a
    /// generated value of its return type.
    fn fake(&mut self, decl: &'a FnDecl) -> Eval<'a> {
        self.dry_calls += 1;
        let mut rng = crate::testing::values::Rng::new(self.dry_calls);
        if let (Some(errors), 0) = (&decl.errors, rng.below(3)) {
            let g = self.gen_of(errors);
            let pick = crate::testing::values::Pick::Random;
            if let Some(e) = g.and_then(|g| self.generate(&g, pick, &[], &mut rng, 0)) {
                return Err(Flow::Fail(e));
            }
        }
        Ok(self.simulate(decl.ret.as_ref(), ""))
    }

    /// A dry run's answer (spec 8.5): a value of `ret` generated like a
    /// test input, from a seed that is the same on every run; printed
    /// (not in a generated test, whose fakes are quiet).
    pub(crate) fn simulate(
        &mut self,
        ret: Option<&'a kavach_ast::TypeExpr>,
        shown: &str,
    ) -> Value<'a> {
        self.dry_calls += 1;
        let mut rng = crate::testing::values::Rng::new(self.dry_calls);
        let value = ret
            .and_then(|t| self.gen_of(t))
            .and_then(|g| self.generate(&g, crate::testing::values::Pick::Random, &[], &mut rng, 0))
            .unwrap_or(Value::Unit);
        if self.testing {
            return value;
        }
        if value == Value::Unit {
            self.err.push_str(&format!("[dry run] {shown}\n"));
        } else {
            self.err
                .push_str(&format!("[dry run] {shown} -> {}\n", value.display()));
        }
        value
    }

    /// The error `Denied(reason)` of the capability method `f` (spec 8.2):
    /// its error type has that variant, as the checker made sure.
    pub(crate) fn denied(&self, f: &FnDecl, reason: &str) -> Flow<'a> {
        let union = f.errors.as_ref().and_then(|e| match &e.kind {
            TypeKind::Named { path, .. } => path
                .segments
                .last()
                .and_then(|s| self.c.resolution.uses.get(&s.span.start).copied()),
            _ => None,
        });
        let Some(union) = union else {
            return Flow::Fail(Value::BuiltinError(format!("denied: {reason}").into()));
        };
        Flow::Fail(Value::Variant {
            union,
            name: "Denied".into(),
            fields: vec![Field {
                name: "reason".into(),
                value: self.str_value(reason),
                secret: false,
            }],
        })
    }
}

/// `Payments.refund(order_id: 1042, amount: 1499)`.
/// Arguments of `secret` parameters are written `<secret>` (spec 8.7).
fn shown_call(call: &str, named: &[(String, Value<'_>)], decl: &FnDecl) -> String {
    let secret = |n: &str| decl.params.iter().any(|p| p.is_secret && p.name.name == n);
    let args: Vec<String> = named
        .iter()
        .map(|(n, v)| match secret(n) {
            true => format!("{n}: <secret>"),
            false => format!("{n}: {}", v.display_code()),
        })
        .collect();
    format!("{call}({})", args.join(", "))
}

/// The call's arguments by the names of the method's parameters.
fn named_args<'a>(decl: &FnDecl, args: &Args<'a>) -> Vec<(String, Value<'a>)> {
    let params: Vec<&str> = decl
        .params
        .iter()
        .map(|p| p.name.name.as_str())
        .filter(|n| *n != "self")
        .collect();
    let mut next = 0;
    args.iter()
        .filter_map(|(name, v)| {
            let name = match name {
                Some(n) => n.to_string(),
                None => {
                    next += 1;
                    params.get(next - 1)?.to_string()
                }
            };
            Some((name, v.clone()))
        })
        .collect()
}

impl Interp<'_> {
    /// Where `agent`'s audit entries go besides its file (spec 8.6); the
    /// first such agent sets how they are sent.
    fn audit_otlp(&self, wanted: bool) -> Option<(String, String)> {
        let config = kavach_driver::Config::for_file(std::path::Path::new(&self.c.path)).ok()?;
        let target = wanted.then(|| config.otlp_collector()).flatten()?;
        kavach_runtime_core::agents::set_audit_sink(kavach_runtime_net::otlp::export_audit);
        Some(target)
    }
}
