//! Agents and declared capabilities in native code (spec 8.2, 8.3, 8.8).
//!
//! A started agent is a record: a handle to its guard in the runtime
//! (`runtime/native/src/rt_agents.rs`, which keeps it with
//! `kavach_runtime_core::agents`, as the interpreter does), then one
//! implementation per declared capability it can use. An implementation
//! is held in a union of every type that implements the capability, so a
//! call reaches the right one.
//!
//! The functions made here ([`Synth`]): for each handler, an event that
//! puts the agent on the runtime's stack of running agents while the
//! handler runs; for each capability method, a dispatcher that finds the
//! running agent; for each agent and method, the guarded call
//! (`lower_guard.rs`).

use crate::ir::{self, Block, ExprKind as K, FieldShape, FuncId, Shape, Stmt, Type, VariantShape};
use crate::lower::{Lowered, Lowerer, Owner};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, AgentDecl, AgentMember, CapabilityDecl, ItemKind, TypeKind};
use kavach_checker::Ty;
use kavach_resolver::{DefId, DefKind};
use kavach_runtime_core::json::{self, Tree};
use std::collections::{BTreeMap, HashMap};

/// A function the compiler makes, built when its turn comes.
#[derive(Clone)]
pub(crate) enum Synth {
    /// `bot.request(…)`: runs handler `handler` of `agent`.
    Event { agent: DefId, handler: FuncId },
    /// `Payments.refund(…)`: through the running agent.
    Dispatch { cap: DefId, method: String },
    /// `agent`'s guarded call.
    Guarded { agent: DefId, called: Called },
    /// `ask<T>`, for answers of type `ok`.
    Ask {
        ok: Ty,
        err: Ty,
        span: ast::Span,
        /// The call's own `model:` (decision 0056).
        model: Option<kavach_driver::AiModel>,
    },
    /// A durable agent's `start` runs its unfinished events again.
    Resume { agent: DefId },
}

/// What a guarded call calls.
#[derive(Clone)]
pub(crate) enum Called {
    /// A capability method, on the agent's implementation.
    Cap { cap: DefId, method: String },
    /// The model, for an answer of type `ok` (spec 8.4).
    Ask {
        ok: Ty,
        err: Ty,
        span: ast::Span,
        /// The call's own `model:` (decision 0056).
        model: Option<Box<kavach_driver::AiModel>>,
    },
}

#[derive(Default)]
pub(crate) struct Agents<'a> {
    /// Every agent, in order: the runtime knows an agent's kind by its
    /// place here.
    pub(crate) list: Vec<(DefId, &'a AgentDecl)>,
    pub(crate) caps: Vec<(DefId, &'a CapabilityDecl)>,
    pub(crate) synth: HashMap<FuncId, Synth>,
    pub(crate) dispatchers: HashMap<(DefId, String), FuncId>,
    /// By agent, call (`Payments.refund`, `Ai.ask`) and result type.
    pub(crate) guarded: HashMap<(DefId, String, Type), FuncId>,
    /// The `ask` function of each answer type.
    /// By the answer's type and the call's own `model:`, as written.
    pub(crate) asks: HashMap<(Type, String), FuncId>,
    /// While a `limit` rule is lowered: the local holding the call's
    /// check, and the rule's number (for `sum` and `count`).
    pub(crate) rule: Option<(ir::Local, usize)>,
}

pub(crate) fn expr(kind: K, ty: Type, line: u32) -> ir::Expr {
    ir::Expr { kind, ty, line }
}

pub(crate) fn int(n: usize, line: u32) -> ir::Expr {
    expr(K::Int(n as i128), Type::I64, line)
}

pub(crate) fn rt(op: RtOp, args: Vec<ir::Expr>, ty: Type, line: u32) -> ir::Expr {
    expr(K::Rt(op, args), ty, line)
}

/// `if cond { then }`, as a statement.
pub(crate) fn when(cond: ir::Expr, then: Vec<Stmt>, line: u32) -> Stmt {
    let then = Block {
        stmts: then,
        value: None,
    };
    Stmt::Expr(expr(
        K::If(Box::new(cond), then, Block::default()),
        Type::Unit,
        line,
    ))
}

pub(crate) fn not(e: ir::Expr) -> ir::Expr {
    let line = e.line;
    expr(K::Unary(ir::UnOp::Not, Box::new(e)), Type::Bool, line)
}

impl<'a> Lowerer<'a> {
    /// Registers an agent: each handler is a function, called through an
    /// event that is the agent's method.
    pub(crate) fn add_agent(&mut self, a: &'a AgentDecl) {
        let Some(&def) = self.decl.get(&a.name.span.start) else {
            return;
        };
        self.agents.list.push((def, a));
        for m in &a.members {
            if let AgentMember::On(f) = m {
                let handler = self.fns.len() as FuncId;
                self.fns.push((Some(f), None));
                let event = self.fns.len() as FuncId;
                self.fns.push((Some(f), Some(Owner::Adt(def))));
                self.method_ids
                    .insert((Owner::Adt(def), f.name.name.clone()), event);
                self.agents.synth.insert(
                    event,
                    Synth::Event {
                        agent: def,
                        handler,
                    },
                );
            }
        }
    }

    pub(crate) fn synth_function(&mut self, s: Synth) -> Lowered<ir::Function> {
        match s {
            // The event's function, run by the function a send calls
            // (`lower_events.rs`).
            Synth::Event { agent, handler } => {
                let inner = self.event_fn(agent, handler)?;
                let types: Vec<Type> = (inner.params.iter())
                    .map(|p| inner.locals[*p as usize])
                    .collect();
                let (ret, fails, line) = (inner.ret, inner.fails, inner.line);
                let name = self.decl_of(handler).name.name.clone();
                let inner = self.add_function(inner);
                self.event_send(inner, agent, &types, (ret, fails), &name, line)
            }
            Synth::Dispatch { cap, method } => self.dispatch_fn(cap, &method),
            Synth::Guarded { agent, called } => self.guarded_fn(agent, called),
            Synth::Ask {
                ok,
                err,
                span,
                model,
            } => self.ask_fn(&ok, &err, span, model),
            Synth::Resume { agent } => self.resume_fn(agent),
        }
    }

    /// Where `def` is declared.
    pub(crate) fn def_span(&self, def: DefId) -> ast::Span {
        let start = ast::Span { start: 0, end: 0 };
        self.c.resolution.defs[def].span.unwrap_or(start)
    }

    pub(crate) fn agent_decl(&self, def: DefId) -> &'a AgentDecl {
        let found = self.agents.list.iter().find(|(d, _)| *d == def);
        found.expect("a registered agent").1
    }

    /// The declared capabilities an agent `can` use, by name, as `start`
    /// takes them.
    pub(crate) fn needs(&self, decl: &AgentDecl) -> BTreeMap<String, DefId> {
        let mut needs = BTreeMap::new();
        for m in &decl.members {
            let AgentMember::Can(paths) = m else {
                continue;
            };
            for p in paths {
                let first = &p.segments[0];
                if let Some(&cap) = self.c.resolution.uses.get(&first.span.start)
                    && self.c.resolution.defs[cap].kind == DefKind::Capability
                {
                    needs.insert(first.name.clone(), cap);
                }
            }
        }
        needs
    }

    /// The types that implement capability `cap`, in the order declared.
    pub(crate) fn impls(&self, cap: DefId) -> Vec<DefId> {
        let res = &self.c.resolution;
        let mut out = Vec::new();
        for item in &self.c.file.items {
            let ItemKind::Type(t) = &item.kind else {
                continue;
            };
            let implements = t.implements.iter().any(|i| match &i.kind {
                TypeKind::Named { path, .. } => {
                    res.uses.get(&path.segments[0].span.start) == Some(&cap)
                }
                _ => false,
            });
            if let (true, Some(&def)) = (implements, self.decl.get(&t.name.span.start)) {
                out.push(def);
            }
        }
        out
    }

    /// The record a started agent is: its guard's handle, then an
    /// implementation per capability, in the order of [`Self::needs`].
    pub(crate) fn agent_type(&mut self, def: DefId, span: ast::Span) -> Lowered<Type> {
        if let Some(t) = self.shapes.adt(def) {
            return Ok(t);
        }
        let decl = self.agent_decl(def);
        let mut fields = vec![FieldShape {
            name: "guard".into(),
            ty: Type::I64,
            secret: false,
        }];
        for (name, cap) in self.needs(decl) {
            let mut variants = Vec::new();
            for d in self.impls(cap) {
                let ty = self.adt_instance(d, Vec::new(), span)?;
                variants.push(VariantShape {
                    name: self.c.resolution.defs[d].name.clone(),
                    fields: vec![FieldShape {
                        name: "it".into(),
                        ty,
                        secret: false,
                    }],
                });
            }
            let ty = self.shapes_intern(Shape::Union {
                name: name.clone(),
                variants,
            });
            fields.push(FieldShape {
                name,
                ty,
                secret: false,
            });
        }
        let name = decl.name.name.clone();
        Ok(self.shapes.add_adt(
            def,
            Shape::Record {
                name,
                fields,
                drop: false,
            },
        ))
    }

    /// `Bot.start(Payments: impl, …)`: a new guard and the implementations.
    pub(crate) fn agent_start(
        &mut self,
        def: DefId,
        args: &'a [ast::Arg],
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let ty = self.agent_type(def, span)?;
        let Some(Shape::Record { fields, .. }) = self.shapes.shape(ty).cloned() else {
            return Err(self.unsupported("this agent", span));
        };
        let needs = self.needs(self.agent_decl(def));
        let description = self.description(def);
        let text = self.str_type();
        let guard = expr(K::Str(description), text, line);
        let mut values = vec![rt(RtOp::AgentStart, vec![guard], Type::I64, line)];
        for f in &fields[1..] {
            let arg = args
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|n| n.name == f.name))
                .ok_or_else(|| self.unsupported("this `start`", span))?;
            let value = self.expr(&arg.value)?;
            let variant = self
                .shapes
                .def_of(value.ty)
                .and_then(|d| self.impls(needs[&f.name]).iter().position(|x| *x == d))
                .ok_or_else(|| self.unsupported("this implementation", arg.value.span))?;
            let fields = vec![value];
            values.push(expr(
                K::Record {
                    variant: variant as u32,
                    fields,
                },
                f.ty,
                line,
            ));
        }
        let started = expr(
            K::Record {
                variant: 0,
                fields: values,
            },
            ty,
            line,
        );
        Ok(match self.journal_of(def) {
            Some(journal) => self.durable_start(def, started, journal, line),
            None => started,
        })
    }

    /// Every capability method in the program, in order: a call's number
    /// for the runtime.
    pub(crate) fn cap_methods(&self) -> Vec<(DefId, &'a ast::FnDecl)> {
        let mut out = Vec::new();
        for (cap, decl) in &self.agents.caps {
            for m in &decl.methods {
                if let ItemKind::Fn(f) = &m.kind {
                    out.push((*cap, f));
                }
            }
        }
        out
    }

    /// What the runtime needs to know of an agent, as JSON: its settings
    /// from `kavach.toml` (read when the program is compiled), its rules
    /// and approvals as written, and every capability method's parameters.
    fn description(&self, def: DefId) -> String {
        let decl = self.agent_decl(def);
        let path = std::path::Path::new(&self.c.path);
        let config = kavach_driver::Config::for_file(path).unwrap_or_default();
        // Its settings in code, with the defaults worked out (0060).
        let settings = self.c.agent_config(&decl.name.name, false);
        let text = |s: &str| Tree::Text(s.to_string());
        let file = |p: Option<String>| p.map_or(Tree::Null, |p| text(&p));
        let mode = match settings.mode {
            kavach_driver::AgentMode::Observe => "observe",
            kavach_driver::AgentMode::Suggest => "suggest",
            kavach_driver::AgentMode::Enforce => "enforce",
        };
        let (mut rules, mut approvals) = (Vec::new(), Vec::new());
        for m in &decl.members {
            match m {
                AgentMember::Limit { target, rules: rs } => {
                    for r in rs {
                        let per = r.per.as_ref().map_or(Tree::Null, |p| text(&p.name));
                        let rule =
                            vec![text(&target_of(target)), text(self.text(r.expr.span)), per];
                        rules.push(Tree::Array(rule));
                    }
                }
                AgentMember::Approve { when, .. } => approvals.push(text(self.text(when.span))),
                _ => {}
            }
        }
        let calls = self
            .cap_methods()
            .into_iter()
            .map(|(cap, f)| {
                let params = f
                    .params
                    .iter()
                    .map(|p| Tree::Array(vec![text(&p.name.name), Tree::Bool(p.is_secret)]))
                    .collect();
                let call = format!("{}.{}", self.c.resolution.defs[cap].name, f.name.name);
                Tree::Array(vec![text(&call), Tree::Array(params)])
            })
            .chain([Tree::Array(vec![
                text("Ai.ask"),
                Tree::Array(vec![Tree::Array(vec![text("prompt"), Tree::Bool(false)])]),
            ])])
            .collect::<Vec<_>>();
        let tree = Tree::Object(vec![
            ("name".into(), text(&decl.name.name)),
            ("mode".into(), text(mode)),
            ("audit".into(), file(settings.audit_log)),
            (
                "audit_json".into(),
                Tree::Bool(settings.audit_format.as_deref() == Some("json")),
            ),
            (
                "otlp".into(),
                match settings
                    .audit_otlp
                    .then(|| config.otlp_collector())
                    .flatten()
                {
                    Some((endpoint, service)) => Tree::Array(vec![text(&endpoint), text(&service)]),
                    None => Tree::Null,
                },
            ),
            ("state".into(), file(settings.state)),
            ("rules".into(), Tree::Array(rules)),
            ("approvals".into(), Tree::Array(approvals)),
            ("calls".into(), Tree::Array(calls)),
        ]);
        json::write(&tree, 0)
    }

    /// The function that runs handler `handler` with its agent running.
    fn event_fn(&mut self, agent: DefId, handler: FuncId) -> Lowered<ir::Function> {
        let f = self.decl_of(handler);
        let ret = match &f.ret {
            Some(t) => self.written_type(t)?,
            None => Type::Unit,
        };
        self.start_function(ret);
        let line = self.line(f.name.span);
        let agent_ty = self.agent_type(agent, f.name.span)?;
        let a = self.new_local(agent_ty);
        let (mut params, mut values) = (vec![a], Vec::new());
        for p in &f.params {
            let Some(t) = &p.ty else {
                return Err(self.unsupported("`self` in a handler", p.name.span));
            };
            let ty = self.written_type(t)?;
            let local = self.new_local(ty);
            params.push(local);
            values.push(self.local_expr(local, line));
        }
        let kind = self.agents.list.iter().position(|(d, _)| *d == agent);
        let push = vec![self.local_expr(a, line), int(kind.unwrap_or(0), line)];
        let mut body = Block::default();
        body.stmts
            .push(Stmt::Expr(rt(RtOp::AgentPush, push, Type::Unit, line)));
        // A durable agent's event goes to its journal (spec 8.5): a panic
        // ends the program before its end does, so it runs again.
        let durable = self.journal_of(agent).is_some();
        if durable {
            let begin = self.journal_begin(a, &f.name.name, values.clone(), line);
            body.stmts.push(begin);
        }
        let call = expr(K::Call(handler, values), ret, line);
        // The event ends while its agent still runs: the end finds the
        // event's number there.
        let mut pop = Vec::new();
        if durable {
            pop.push(self.journal_end(a, line));
        }
        pop.push(Stmt::Expr(rt(RtOp::AgentPop, Vec::new(), Type::Unit, line)));
        let value = self.new_local(ret);
        match &f.errors {
            Some(errors) => {
                let err = self.written_type(errors)?;
                let error = self.new_local(err);
                body.stmts.push(Stmt::LetResult { value, error, call });
                body.stmts.extend(pop);
                let fail = expr(
                    K::Fail(Box::new(self.local_expr(error, line))),
                    Type::Unit,
                    line,
                );
                body.stmts
                    .push(when(self.failed(error, line), vec![Stmt::Expr(fail)], line));
            }
            None => {
                body.stmts.push(Stmt::Let(value, call));
                body.stmts.extend(pop);
            }
        }
        body.value = (ret != Type::Unit).then(|| Box::new(self.local_expr(value, line)));
        Ok(ir::Function {
            name: format!("{}.{}", self.agent_decl(agent).name.name, f.name.name),
            params,
            ret,
            locals: self.locals.clone(),
            body,
            line,
            fails: f.errors.is_some(),
            external: None,
        })
    }
}

/// `Payments.refund`, or `Ai`.
pub(crate) fn target_of(path: &ast::Path) -> String {
    let names: Vec<&str> = path.segments.iter().map(|s| s.name.as_str()).collect();
    names.join(".")
}

/// Whether an agent's `can` covers `call` (`Cap.method`).
pub(crate) fn can_call(decl: &AgentDecl, call: &str) -> bool {
    decl.members.iter().any(|m| match m {
        AgentMember::Can(paths) => paths
            .iter()
            .any(|p| kavach_runtime_core::agents::covers(&target_of(p), call)),
        _ => false,
    })
}
