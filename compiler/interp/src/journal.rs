//! Durable agents in the interpreter (spec 8.5, decision 0025): events
//! and calls go to the agent's journal (`kavach_runtime_core::journal`),
//! and the events a crash left unfinished run again when it starts.

use crate::agents::Running;
use crate::interp::Interp;
use crate::json::json_encode;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{AgentMember, FnDecl, Span, TypeExpr, TypeKind};
use kavach_checker::{Prim, Ty};
use kavach_resolver::DefKind;
use kavach_runtime_core::journal::{Outcome, Unfinished};
use kavach_runtime_core::json::Tree;
use std::rc::Rc;
use std::sync::Arc;

/// An event run again after a restart that waits for a person: its
/// handler, arguments and task (decision 0072).
#[derive(Debug)]
pub(crate) struct Replay<'a> {
    handler: String,
    args: Vec<Tree>,
    task: Arc<crate::tasks::TaskHandle<'a>>,
}

/// What the task of an event run again after a restart runs.
pub(crate) struct Replayed<'a> {
    agent: Rc<Running<'a>>,
    f: &'a FnDecl,
    args: crate::calls::Args<'a>,
    unfinished: Unfinished,
    span: Span,
}

impl Replayed<'_> {
    /// The event, as an error nobody read names it.
    pub(crate) fn event(&self) -> String {
        let u = &self.unfinished;
        format!("event #{} of {}", u.number(), u.handler)
    }
}

impl<'a> Interp<'a> {
    /// The checker's type for a written type, to read a recorded value
    /// back; `None` for what a journal can't hold.
    pub(crate) fn ty_of(&self, t: &TypeExpr) -> Option<Ty> {
        Some(match &t.kind {
            TypeKind::Named { path, args } => {
                let first = &path.segments[0];
                let def = *self.c.resolution.uses.get(&first.span.start)?;
                match self.c.resolution.defs[def].kind {
                    // A generic parameter: the running call's argument.
                    DefKind::GenericParam => self.frames.last()?.generics.get(&def)?.clone(),
                    DefKind::Builtin => match (first.name.as_str(), args.as_slice()) {
                        ("List", [item]) => Ty::List(Box::new(self.ty_of(item)?)),
                        ("Map", [k, v]) => {
                            Ty::Map(Box::new(self.ty_of(k)?), Box::new(self.ty_of(v)?))
                        }
                        (name, _) => Ty::Prim(Prim::from_name(name)?),
                    },
                    DefKind::Type => {
                        let args = args.iter().map(|a| self.ty_of(a)).collect::<Option<_>>()?;
                        Ty::Adt(def, args)
                    }
                    _ => return None,
                }
            }
            TypeKind::List(item) => Ty::List(Box::new(self.ty_of(item)?)),
            TypeKind::Optional(inner) => Ty::Optional(Box::new(self.ty_of(inner)?)),
            TypeKind::Map(k, v) => Ty::Map(Box::new(self.ty_of(k)?), Box::new(self.ty_of(v)?)),
            _ => return None,
        })
    }

    /// Reads a recorded outcome back as the call's value, or its error.
    fn outcome_value(
        &mut self,
        o: Outcome,
        ok: Option<Ty>,
        err: Option<Ty>,
        span: Span,
    ) -> Eval<'a> {
        let (tree, ty, failed) = match o {
            Outcome::Ok(t) => (t, ok.unwrap_or(Ty::Unit), false),
            Outcome::Err(t) => (t, err.unwrap_or(Ty::Unit), true),
        };
        let value = if ty == Ty::Unit {
            Value::Unit
        } else {
            self.decode(&tree, &ty, "")
                .map_err(|_| self.panic("the journal holds a value of another type", span))?
        };
        if failed {
            Err(Flow::Fail(value))
        } else {
            Ok(value)
        }
    }

    /// Before a call of the running agent (`Cap.method`, or `Ai.ask`): its
    /// recorded outcome, if the event is being run again and got that far.
    pub(crate) fn journal_recorded(
        &mut self,
        call: &str,
        ok: Option<Ty>,
        err: Option<Ty>,
        span: Span,
    ) -> Option<Eval<'a>> {
        let agent = self.running.last().cloned()?;
        let event = *self.events.last()?;
        let found = agent.journal.as_ref()?.borrow_mut().recorded(event, call);
        match found {
            Ok(Some(o)) => Some(self.outcome_value(o, ok, err, span)),
            Ok(None) => None,
            Err(why) => Some(Err(self.panic(why, span))),
        }
    }

    /// A durable handler's clock read or a wait's end, `call` (spec 8.5,
    /// decision 0062): the moment the journal has for it, or `now`,
    /// journaled. Inside a capability's implementation nothing is: the
    /// call's outcome is.
    pub(crate) fn journaled_time(
        &mut self,
        call: &str,
        now: i128,
        span: Span,
    ) -> Result<i128, Flow<'a>> {
        if self.in_call > 0 {
            return Ok(now);
        }
        if let Some(recorded) = self.journal_recorded(call, Some(Ty::Prim(Prim::Int)), None, span) {
            return match recorded? {
                Value::Int(ms, _) => Ok(ms),
                _ => Ok(now),
            };
        }
        self.journal_record(call, &Ok(Value::Int(now, Prim::Int)), span)?;
        Ok(now)
    }

    /// After a call of the running agent ran: its outcome. A journal that
    /// can't be written stops the program (spec 8.5).
    pub(crate) fn journal_record(
        &self,
        call: &str,
        result: &Eval<'a>,
        span: Span,
    ) -> Result<(), Flow<'a>> {
        let Some(agent) = self.running.last() else {
            return Ok(());
        };
        let Some(journal) = &agent.journal else {
            return Ok(());
        };
        let outcome = match result {
            Ok(v) => Outcome::Ok(json_encode(v)),
            Err(Flow::Fail(e)) => Outcome::Err(json_encode(e)),
            Err(_) => return Ok(()),
        };
        let Some(&event) = self.events.last() else {
            return Ok(());
        };
        let written = journal.borrow_mut().record(event, call, outcome);
        written.map_err(|why| self.panic(why, span))
    }

    /// Runs handler `f` of `agent` as an event: a new one (written to the
    /// journal), or one being run again (`unfinished`).
    pub(crate) fn journaled_event(
        &mut self,
        agent: &Rc<Running<'a>>,
        f: &'a FnDecl,
        args: crate::calls::Args<'a>,
        unfinished: Option<Unfinished>,
        span: Span,
    ) -> Eval<'a> {
        let Some(journal) = &agent.journal else {
            return self.call_fn(f, None, args, span);
        };
        let begun = match unfinished {
            Some(u) => journal.borrow_mut().again(u),
            None => {
                let trees = args.iter().map(|(_, v)| json_encode(v)).collect();
                journal.borrow_mut().begin(&f.name.name, trees)
            }
        };
        let event = begun.map_err(|why| self.panic(why, span))?;
        self.events.push(event);
        let result = self.call_fn(f, None, args, span);
        self.events.pop();
        // Only a handler that returned ends its event (spec 8.5). A panic,
        // `cli.exit` or the program ending while it waits (its task is
        // cancelled) leaves it unfinished, as a crash does: it runs again
        // next time.
        if matches!(result, Ok(_) | Err(Flow::Fail(_))) {
            let ended = journal.borrow_mut().end(event);
            ended.map_err(|why| self.panic(why, span))?;
        }
        result
    }

    /// When a durable agent starts: its events that never ended run again,
    /// in order (spec 8.5), each in a task of its own. `start` waits for
    /// each until it ends or waits for a person; then the event goes on
    /// inside the agent, until the function that started the agent
    /// returns (decision 0072, spec 9.2).
    pub(crate) fn resume(
        &mut self,
        agent: &Rc<Running<'a>>,
        unfinished: Vec<Unfinished>,
        span: Span,
    ) -> Result<(), Flow<'a>> {
        for u in unfinished {
            let handler = agent.decl.members.iter().find_map(|m| match m {
                AgentMember::On(f) if f.name.name == u.handler => Some(f),
                _ => None,
            });
            let Some(f) = handler else {
                return Err(self.panic(
                    format!(
                        "the journal has an event of `{}`, which this agent no longer handles",
                        u.handler
                    ),
                    span,
                ));
            };
            let mut args = Vec::new();
            for (p, t) in f.params.iter().zip(&u.args) {
                let Some(ty) = p.ty.as_ref().and_then(|t| self.ty_of(t)) else {
                    return Err(self.panic("the journal can't hold this handler's arguments", span));
                };
                let v = self.decode(t, &ty, "").map_err(|_| {
                    self.panic("the journal holds an argument of another type", span)
                })?;
                args.push((None, v));
            }
            let (handler, trees) = (u.handler.clone(), u.args.clone());
            let replayed = Replayed {
                agent: Rc::clone(agent),
                f,
                args,
                unfinished: u,
                span,
            };
            let Some(sched) = self.tasks.as_ref().map(|t| Arc::clone(&t.sched)) else {
                // No tasks: the event runs to its end here.
                let event = replayed.event();
                match self.replay(replayed) {
                    Err(e @ (Flow::Panic(..) | Flow::Exit(_))) => return Err(e),
                    Err(Flow::Fail(e)) => {
                        eprint!("{}", crate::task_errors::replay_error(&event, &e.display()));
                    }
                    _ => {}
                }
                continue;
            };
            let Value::Task(task) = self.spawn_work(crate::tasks::Work::Replay(replayed), span)?
            else {
                continue;
            };
            loop {
                if sched.is_done(task.id) {
                    break;
                }
                if agent.asking.borrow().contains(&task.id) {
                    agent.replays.borrow_mut().push(Replay {
                        handler,
                        args: trees,
                        task,
                    });
                    break;
                }
                let soon = std::time::Instant::now() + std::time::Duration::from_millis(10);
                self.sleep_until(soon, span)?;
            }
        }
        Ok(())
    }

    /// Runs event `r` again. It has no sender: its outcome goes to the
    /// audit log (spec 8.6).
    pub(crate) fn replay(&mut self, r: Replayed<'a>) -> Eval<'a> {
        let (agent, n) = (r.agent, r.unfinished.number());
        let handler = r.unfinished.handler.clone();
        agent.guard.borrow_mut().next_request();
        self.running.push(Rc::clone(&agent));
        let result = self.journaled_event(&agent, r.f, r.args, Some(r.unfinished), r.span);
        self.running.pop();
        // A panic goes to the audit log, as any handler's does (8.6); the
        // task's end prints it.
        let panicked = match &result {
            Err(Flow::Panic(message, at)) => {
                let (path, line) = self.c.shown_location(at.start);
                let why = format!("panicked: {message} at {path}:{line}");
                let _ = agent.guard.borrow().note(&format!("{handler} {why}"));
                Some(why)
            }
            _ => None,
        };
        let task = self.task;
        agent.replays.borrow_mut().retain(|w| w.task.id != task);
        let outcome = match &result {
            Ok(v) => Some(format!("ok {}", v.display_code())),
            Err(Flow::Fail(e)) => Some(format!("error {}", e.display_code())),
            Err(_) => panicked.map(|why| {
                let error = Value::BuiltinError(why.into());
                format!("error {}", error.display_code())
            }),
        };
        if let Some(o) = outcome {
            let text = format!("finished after a restart: event #{n} of {handler}: {o}");
            let _ = agent.guard.borrow().note(&text);
        }
        result
    }

    /// The task of an event run again after a restart, of handler `name`
    /// with arguments equal to `args`, that still waits for a person.
    pub(crate) fn waiting_replay(
        &self,
        agent: &Running<'a>,
        name: &str,
        args: &crate::calls::Args<'a>,
    ) -> Option<Arc<crate::tasks::TaskHandle<'a>>> {
        let replays = agent.replays.borrow();
        if replays.is_empty() {
            return None;
        }
        let trees: Vec<Tree> = args.iter().map(|(_, v)| json_encode(v)).collect();
        replays
            .iter()
            .find(|w| w.handler == name && w.args == trees)
            .map(|w| Arc::clone(&w.task))
    }
}
