//! `spawn process f(args)` (spec 9.8, decision 0075) in the interpreter.
//! The parent writes the arguments as JSON and runs the program again in
//! a child (`runtime/core/src/processes.rs`), which runs only that call:
//! it is found by where it is written, the same place in both. The child
//! prints its outcome last: `{"ok": value}` or `{"error": …}`, an error
//! by its type's definition or its text. `cancel()` ends the child.

use crate::interp::Interp;
use crate::json::json_encode;
use crate::tasks::Work;
use crate::value::{Eval, Flow, Value};
use kavach_ast::visit::{VisitMut, walk_expr, walk_file};
use kavach_ast::{Expr, ExprKind, Span, SpawnKind};
use kavach_checker::Ty;
use kavach_runtime_core::json::{self, Limits, Tree};
use kavach_runtime_core::processes::{self as procs, Event, Running};
use std::sync::Arc;

/// How this program is started again, set by whoever runs it (`kavach
/// run` gives its own command line); none, and `spawn process` can't run:
/// a host that runs programs inside itself, such as a test, would start
/// itself again instead.
static RESTART: std::sync::OnceLock<Vec<std::ffi::OsString>> = std::sync::OnceLock::new();

/// Sets the command that starts this program again (`spawn process`).
pub fn set_restart(command: Vec<std::ffi::OsString>) {
    let _ = RESTART.set(command);
}

/// A call to run in a child: where it is written (its number there too),
/// its arguments as JSON, and its type.
pub(crate) struct Process {
    span: Span,
    arguments: String,
    ty: Option<Ty>,
}

/// Limits wide enough for any arguments or outcome a program sends itself.
fn no_limits() -> Limits {
    Limits {
        max_depth: 10_000,
        max_bytes: usize::MAX,
        max_fields: usize::MAX,
        comments: false,
    }
}

impl<'a> Interp<'a> {
    /// `spawn process x`: its arguments now, and a task that runs the call
    /// in a child.
    pub(crate) fn spawn_process(&mut self, x: &'a Expr) -> Eval<'a> {
        let ExprKind::Call { args, .. } = &x.kind else {
            return Err(self.panic("`spawn process` takes a call", x.span));
        };
        let mut values = Vec::new();
        for a in args {
            values.push(json_encode(&self.eval(&a.value)?));
        }
        let process = Process {
            span: x.span,
            arguments: json::write(&Tree::Array(values), 0),
            ty: self.c.expr_types.get(&(x.span.start, x.span.end)).cloned(),
        };
        self.spawn_work(Work::Process(process), x.span)
    }

    /// The task's work: the child's lines are printed in turn, and its
    /// outcome is the task's.
    pub(crate) fn run_process(&mut self, p: Process) -> Eval<'a> {
        let Some(command) = RESTART.get() else {
            return Err(self.panic(
                "`spawn process` can't start the program again here: run it with `kavach run`",
                p.span,
            ));
        };
        let started = procs::start(command, i64::from(p.span.start), &p.arguments);
        let running =
            started.map_err(|e| self.panic(format!("can't start a process: {e}"), p.span))?;
        loop {
            let seen = running.count();
            match running.next() {
                Some(Event::Line(text)) => {
                    self.out.push_str(&text);
                    self.out.push('\n');
                    self.show_live();
                }
                Some(Event::Ended { outcome, code }) => return self.outcome(outcome, code, &p),
                None => {
                    if let Err(stop) = self.wait_child(&running, seen, p.span) {
                        running.kill();
                        return Err(stop);
                    }
                }
            }
        }
    }

    /// Waits for the child's next event after the count `seen`.
    fn wait_child(&mut self, running: &Running, seen: u64, span: Span) -> Result<(), Flow<'a>> {
        let Some(sched) = self.tasks.as_ref().map(|t| Arc::clone(&t.sched)) else {
            std::thread::sleep(std::time::Duration::from_millis(1));
            return Ok(());
        };
        self.flush();
        let me = self.task;
        let (wait, waker) = (sched.next_wait(me), Arc::clone(&sched));
        if !running.on_event(seen, Box::new(move || waker.wake_from_thread(me, wait))) {
            return Ok(());
        }
        sched
            .wait_io(me, None)
            .map_err(|stop| self.stopped(stop, span))
    }

    /// The task's value or error, from the child's last line. A child that
    /// ended without one panicked (it said why) or exited: so does the
    /// program (spec 9.1).
    fn outcome(&mut self, outcome: Option<String>, code: Option<i32>, p: &Process) -> Eval<'a> {
        let tree = outcome.and_then(|o| json::parse(&o, &no_limits()).ok());
        let Some(Tree::Object(mut fields)) = tree else {
            return Err(Flow::Exit(code.unwrap_or(101)));
        };
        let (key, value) = fields.remove(0);
        // The call panicked (the child printed it): the task's error, as
        // for any task (spec 9.1).
        if let ("panicked", Tree::Text(why)) = (key.as_str(), &value) {
            return Err(Flow::Fail(Value::BuiltinError(why.as_str().into())));
        }
        let (ok, error) = match &p.ty {
            Some(Ty::Result(ok, error)) => ((**ok).clone(), Some((**error).clone())),
            Some(t) => (t.clone(), None),
            None => (Ty::Unit, None),
        };
        if key == "ok" {
            return match ok {
                Ty::Unit => Ok(Value::Unit),
                ok => self.decode(&value, &ok, "the process's result"),
            };
        }
        let error = match value {
            Tree::Object(mut e) if e.first().is_some_and(|(k, _)| k == "def") => {
                let (_, def) = e.remove(0);
                let (_, value) = e.remove(0);
                let def = match def {
                    Tree::Int(d) => d as usize,
                    _ => 0,
                };
                self.decode(&value, &Ty::Adt(def, Vec::new()), "the process's error")?
            }
            Tree::Object(mut e) => match e.pop() {
                Some((_, Tree::Text(t))) => Value::BuiltinError(t.into()),
                _ => Value::BuiltinError("the process failed".into()),
            },
            _ => {
                let _ = error;
                Value::BuiltinError("the process failed".into())
            }
        };
        Err(Flow::Fail(error))
    }

    /// In a child: runs the `spawn process` call written at byte `site`
    /// with the arguments it was given, and prints its outcome last.
    pub(crate) fn run_as_child(&mut self, site: u32) -> Result<(), Flow<'a>> {
        let Some(call) = find_call(&self.c.file, site) else {
            return Err(self.panic(
                "this program has no such `spawn process` call",
                Span::new(0, 0),
            ));
        };
        let f = (self.c.resolution.uses.get(&call.callee))
            .and_then(|d| self.fns.get(d))
            .copied();
        let Some(f) = f else {
            return Err(self.panic("`spawn process` takes a call of a function", call.span));
        };
        let given = json::parse(&procs::arguments(), &no_limits()).ok();
        let items = match given {
            Some(Tree::Array(items)) => items,
            _ => Vec::new(),
        };
        let mut values = Vec::new();
        for (a, item) in call.args.iter().zip(&items) {
            let ty = self.c.expr_types.get(&(a.start, a.end)).cloned();
            let v = match ty {
                Some(ty) => self.decode(item, &ty, "an argument")?,
                None => Value::Unit,
            };
            values.push((None, v));
        }
        let outcome = match self.call_fn(f, None, values, call.span) {
            Ok(v) => Tree::Object(vec![("ok".into(), json_encode(&v))]),
            Err(Flow::Fail(e)) => Tree::Object(vec![("error".into(), error_tree(&e))]),
            Err(Flow::Panic(message, at)) => {
                let (path, line) = self.c.shown_location(at.start);
                let why = format!("panicked: {message} at {path}:{line}");
                let outcome = Tree::Object(vec![("panicked".into(), Tree::Text(why))]);
                self.out
                    .push_str(&procs::outcome_line(&json::write(&outcome, 0)));
                self.show_live();
                return Err(Flow::Panic(message, at));
            }
            Err(other) => return Err(other),
        };
        self.out
            .push_str(&procs::outcome_line(&json::write(&outcome, 0)));
        self.show_live();
        Ok(())
    }
}

/// An error, for the parent: a value of a declared type by that type's
/// definition, any other by its text.
fn error_tree(e: &Value<'_>) -> Tree {
    let def = match e {
        Value::Record { def, .. } => Some(*def),
        Value::Variant { union, .. } => Some(*union),
        _ => None,
    };
    match def {
        Some(def) => Tree::Object(vec![
            ("def".into(), Tree::Int(def as i64)),
            ("value".into(), json_encode(e)),
        ]),
        None => Tree::Object(vec![("text".into(), Tree::Text(e.display().to_string()))]),
    }
}

/// A `spawn process` call, as the child needs it: where its function's
/// name is, its arguments, and where it is.
struct Call {
    callee: u32,
    args: Vec<Span>,
    span: Span,
}

/// The call of the `spawn process` written at byte `site`.
fn find_call(file: &kavach_ast::File, site: u32) -> Option<Call> {
    struct Find(u32, Option<Call>);
    impl VisitMut for Find {
        fn expr(&mut self, e: &mut Expr) {
            if let ExprKind::Spawn(SpawnKind::Process, x, _) = &e.kind
                && x.span.start == self.0
                && let ExprKind::Call { callee, args, .. } = &x.kind
                && let ExprKind::Name(id) = &callee.kind
            {
                self.1 = Some(Call {
                    callee: id.span.start,
                    args: args.iter().map(|a| a.value.span).collect(),
                    span: x.span,
                });
            }
            walk_expr(self, e);
        }
    }
    let mut find = Find(site, None);
    walk_file(&mut find, &mut file.clone());
    find.1
}
