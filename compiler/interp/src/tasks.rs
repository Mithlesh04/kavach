// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Tasks (spec 9, decision 0020). Each task runs on its own thread, and
//! they take turns through the scheduler in `kavach_runtime_core::tasks`,
//! the one native code uses, so both run tasks in the same order.
//!
//! A task is an `Interp` of its own, with the program's tables and a copy
//! of the frame that spawned it (spec 9.5: a task works on copies). It
//! moves what it printed into the program's output whenever it hands the
//! turn on, so the output is in the order things happened.

use crate::interp::{Frame, Interp};
use crate::value::{Eval, Flow, Value};
use kavach_ast::{Expr, Span};
use kavach_runtime_core::tasks::{Scheduler, TaskId};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Instant;

/// Work for the thread that owns the scope task threads live in: it
/// starts each as a thread (`on_big_stack` in `lib.rs`).
pub(crate) type Job<'a> = Box<dyn FnOnce() + Send + 'a>;

/// What every task of one run shares.
#[derive(Clone)]
pub(crate) struct Tasks<'a> {
    pub(crate) sched: Arc<Scheduler>,
    pub(crate) spawner: mpsc::Sender<Job<'a>>,
    pub(crate) out: Arc<Mutex<String>>,
    pub(crate) err: Arc<Mutex<String>>,
    /// Output goes to the terminal as it comes (`kavach run`), instead of
    /// being gathered for the caller (tests).
    pub(crate) live: bool,
    /// What ended the program, when a task other than `main` panicked
    /// (or called `cli.exit`).
    pub(crate) ended: Arc<Mutex<Option<Baton<Flow<'a>>>>>,
    /// Every `Shared` made and where, so one a cycle keeps alive to the
    /// end is reported (spec 6.2).
    pub(crate) made: Arc<Mutex<Vec<Baton<MadeShared<'a>>>>>,
    /// Set from outside to stop the whole program at its next loop round
    /// or call: a time limit (`kavach mcp`, spec 13.8).
    pub(crate) stop: Arc<std::sync::atomic::AtomicBool>,
    /// The `on threads` rounds running in this run (spec 4.9).
    pub(crate) rounds: Arc<kavach_runtime_core::RoundCount>,
    /// What each `on threads` loop writes, worked out once a run.
    pub(crate) loops: crate::on_threads::Loops,
}

/// A `Shared` value, without keeping it alive, and the offset of the
/// call that made it.
pub(crate) type MadeShared<'a> = (std::rc::Weak<std::cell::RefCell<Value<'a>>>, u32);

impl<'a> Tasks<'a> {
    pub(crate) fn new(spawner: mpsc::Sender<Job<'a>>) -> Tasks<'a> {
        Tasks {
            sched: Scheduler::new(),
            spawner,
            out: Arc::default(),
            err: Arc::default(),
            live: false,
            ended: Arc::default(),
            made: Arc::default(),
            stop: Arc::default(),
            rounds: Arc::default(),
            loops: Arc::default(),
        }
    }
}

/// A value moved to, or read from, another task's thread.
///
/// Values hold `Rc`s, which threads may not share. Tasks take turns: only
/// the task holding the scheduler's turn runs, and the turn passes
/// through a mutex, which orders memory between the threads. A `Baton` is
/// only made, moved and opened by the task holding the turn, so no two
/// threads ever touch its `Rc`s at the same time.
pub(crate) struct Baton<T>(pub(crate) T);

// SAFETY: see `Baton` above: its contents are only used by the one
// thread running at a time, after the turn passed to it through a mutex.
#[allow(unsafe_code)]
unsafe impl<T> Send for Baton<T> {}

impl<T> Baton<T> {
    fn into_inner(self) -> T {
        self.0
    }
}

impl<'a> Tasks<'a> {
    /// Notes a `Shared` value just made, at offset `at`.
    pub(crate) fn made_shared(&self, v: &std::rc::Rc<std::cell::RefCell<Value<'a>>>, at: u32) {
        let mut made = self.made.lock().unwrap_or_else(|e| e.into_inner());
        made.push(Baton((std::rc::Rc::downgrade(v), at)));
    }
}

/// The offsets of the `Shared` values still alive once every task has
/// ended: only a cycle keeps one alive then.
pub(crate) fn leaked_shared(made: &Mutex<Vec<Baton<MadeShared<'_>>>>) -> Vec<u32> {
    let made = made.lock().unwrap_or_else(|e| e.into_inner());
    made.iter()
        .filter(|b| b.0.0.strong_count() > 0)
        .map(|b| b.0.1)
        .collect()
}

/// The parts of a spawned call worked out before its task starts, by
/// span (spec 9.1).
pub(crate) type Held<'a> = Vec<((u32, u32), Value<'a>)>;

/// What a task runs: `spawn x`, with the parts of its call worked out
/// before (spec 9.1), or a function value on one item (`parallel_map`,
/// spec 9.3).
pub(crate) enum Work<'a> {
    Expr(&'a Expr, Held<'a>),
    Call(Value<'a>, Value<'a>, Span),
    /// `time.after(d, f)` (spec 14.4): `f` after `d` milliseconds.
    After(u64, Value<'a>, Span),
    /// A durable agent's event run again after a restart (decision 0072).
    Replay(crate::journal::Replayed<'a>),
    /// `spawn process f(args)` (spec 9.8): the call, run in a child.
    Process(crate::processes::Process),
    /// A round of an `on threads` loop, then what it reads back (4.9).
    Round(&'a kavach_ast::Block, &'a [Expr]),
}

/// A time limit on a wait (`.timeout(d)`, spec 9.6): when it ends, and
/// the milliseconds it was given (for the error).
pub(crate) type Limit = (Instant, i64);

/// A spawned task: its id, and its result once it has ended.
pub struct TaskHandle<'a> {
    pub(crate) id: TaskId,
    pub(crate) result: Mutex<Option<Baton<Eval<'a>>>>,
    /// For an error nobody reads (`task_errors.rs`).
    pub(crate) unread: crate::task_errors::Unread,
}

impl std::fmt::Debug for TaskHandle<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task({})", self.id)
    }
}

/// A copy of a task's result, to give to whoever waits for it.
pub(crate) fn copy<'a>(r: &Eval<'a>) -> Eval<'a> {
    match r {
        Ok(v) => Ok(v.clone()),
        Err(f) => Err(copy_flow(f)),
    }
}

pub(crate) fn copy_flow<'a>(f: &Flow<'a>) -> Flow<'a> {
    match f {
        Flow::Return(v) => Flow::Return(v.clone()),
        Flow::Break(l) => Flow::Break(l.clone()),
        Flow::Continue(l) => Flow::Continue(l.clone()),
        Flow::Fail(v) => Flow::Fail(v.clone()),
        Flow::Panic(m, s) => Flow::Panic(m.clone(), *s),
        Flow::Exit(c) => Flow::Exit(*c),
        Flow::Cancelled => Flow::Cancelled,
    }
}

impl<'a> Interp<'a> {
    /// `spawn x` (spec 9.1): a new task that will evaluate `x`, once the
    /// parts of its call are worked out here.
    pub(crate) fn spawn(&mut self, x: &'a Expr) -> Eval<'a> {
        let held = self.spawn_parts(x)?;
        self.spawn_work(Work::Expr(x, held), x.span)
    }

    /// The parts of spawned call `x` that are worked out before its task
    /// starts (spec 9.1, found by the checker), in the order they are
    /// written: the value a method is called on or the function worked
    /// out from an expression, then the arguments.
    fn spawn_parts(&mut self, x: &'a Expr) -> Result<Held<'a>, Flow<'a>> {
        let kavach_ast::ExprKind::Call { callee, args, .. } = &x.kind else {
            return Ok(Vec::new());
        };
        // The value before the dot, or a function worked out from an
        // expression; the checker leaves out a plain name.
        let receiver = match &callee.kind {
            kavach_ast::ExprKind::Field { base, .. } => Some(&**base),
            _ => Some(&**callee),
        };
        // A part of this task's own spawned call, not yet used, is no part
        // of this one.
        let outer = std::mem::take(&mut self.held);
        let mut held = Vec::new();
        let mut found = Ok(());
        for p in receiver.into_iter().chain(args.iter().map(|a| &a.value)) {
            let span = (p.span.start, p.span.end);
            if !self.c.spawn_parts.contains(&span) {
                continue;
            }
            match self.eval(p) {
                Ok(v) => held.push((span, v)),
                Err(f) => {
                    found = Err(f);
                    break;
                }
            }
        }
        self.held = outer;
        found.map(|()| held)
    }

    /// A part of a spawned call worked out before its task started.
    pub(crate) fn take_held(&mut self, e: &Expr) -> Option<Value<'a>> {
        let span = (e.span.start, e.span.end);
        let i = self.held.iter().position(|(s, _)| *s == span)?;
        Some(self.held.swap_remove(i).1)
    }

    /// A new task that will run `work`.
    pub(crate) fn spawn_work(&mut self, work: Work<'a>, span: Span) -> Eval<'a> {
        let (path, line) = self.c.shown_location(span.start);
        let at = format!("{path}:{line}");
        let event = match &work {
            Work::Replay(r) => Some(r.event()),
            _ => None,
        };
        let Some(tasks) = self.tasks.clone() else {
            // No scheduler: the task runs at once.
            let result = self.run_work(work);
            let handle = TaskHandle {
                id: 0,
                result: Mutex::new(Some(Baton(result))),
                unread: crate::task_errors::Unread {
                    at,
                    event,
                    ..Default::default()
                },
            };
            return Ok(Value::Task(Arc::new(handle)));
        };
        let id = tasks.sched.add();
        let handle = Arc::new(TaskHandle {
            id,
            result: Mutex::new(None),
            unread: crate::task_errors::Unread {
                at,
                event,
                sink: Some((Arc::clone(&tasks.err), tasks.live)),
                ..Default::default()
            },
        });
        let child = self.child(tasks.clone(), id);
        let job = Baton((child, Arc::clone(&handle), work));
        let start: Job<'a> = Box::new(move || {
            let (child, handle, work) = job.into_inner();
            child.run_task(work, span, &handle);
        });
        if tasks.spawner.send(start).is_err() {
            return Err(self.panic("the program is ending", span));
        }
        let frame = self.frames.last_mut().expect("a frame");
        // A loop that never returns (a server's) would keep every task it
        // spawned: finished ones are let go of as the list grows. The join
        // skips finished tasks anyway, and a handle the program holds
        // still has the result.
        if frame.spawned.len() >= 64 && frame.spawned.len().is_power_of_two() {
            frame.spawned.retain(|t| !tasks.sched.is_done(t.id));
        }
        frame.spawned.push(Arc::clone(&handle));
        Ok(Value::Task(handle))
    }

    /// The interpreter task `id` runs in: the program's tables, and a
    /// copy of the frame that spawned it.
    fn child(&self, tasks: Tasks<'a>, id: TaskId) -> Interp<'a> {
        let top = self.frames.last().expect("a frame");
        let frame = Frame {
            vars: top.vars.clone(),
            self_value: top.self_value.clone(),
            its: top.its.clone(),
            spawned: Vec::new(),
            generics: top.generics.clone(),
        };
        Interp {
            c: self.c,
            decl: self.decl.clone(),
            fns: self.fns.clone(),
            records: self.records.clone(),
            unions: self.unions.clone(),
            agents: self.agents.clone(),
            capabilities: self.capabilities.clone(),
            // A task keeps the agent of the handler that spawned it.
            running: self.running.clone(),
            in_call: self.in_call,
            totals: None,
            dry_run: self.dry_run,
            // A task a test spawns can't change the machine either.
            no_changes: self.no_changes,
            dry_calls: 0,
            log: self.log.clone(),
            methods: self.methods.clone(),
            ext_methods: self.ext_methods.clone(),
            consts: self.consts.clone(),
            consts_busy: Default::default(),
            frames: vec![frame],
            result: Vec::new(),
            invariant_fields: Vec::new(),
            emptying: false,
            held_receiver: None,
            panic_trail: Vec::new(),
            out: String::new(),
            err: String::new(),
            steps: self.steps,
            step_limit: self.step_limit,
            std_modules: self.std_modules.clone(),
            args: self.args.clone(),
            testing: self.testing,
            tasks: Some(tasks),
            task: id,
            fs_scopes: kavach_runtime_core::fs_scope::inherited(&self.fs_scopes),
            gen_rng: crate::testing::values::Rng::new(0),
            random: self.random.as_ref().map(|r| r.fork(id as u64)),
            inout_out: Vec::new(),
            checked_ctor: None,
            next_generics: None,
            leaving: false,
            fake: self.fake.clone(),
            frozen: self.frozen.clone(),
            reached_c: false,
            cleaning: false,
            drops: std::rc::Rc::clone(&self.drops),
            drop_types: self.drop_types.clone(),
            block_lets: Vec::new(),
            drop_floor: 0,
            // A task spawned in a handler makes its calls for the event.
            events: self.events.clone(),
            // A task runs the code it started with (spec 10.5).
            hot: None,
            held: Vec::new(),
        }
    }

    /// Runs task `x` on this (new) thread, when its turn comes.
    fn run_work(&mut self, work: Work<'a>) -> Eval<'a> {
        match work {
            Work::Expr(x, held) => {
                let outer = std::mem::replace(&mut self.held, held);
                let result = self.eval(x);
                self.held = outer;
                match result {
                    Err(Flow::Return(v)) => Ok(v),
                    other => other,
                }
            }
            Work::Call(f, item, span) => self.call_value(f, vec![item], span),
            // A wait like `sleep`'s: under `kavach test` the fake clock
            // moves on at once (spec 11.5).
            Work::Replay(r) => self.replay(r),
            Work::Process(p) => self.run_process(p),
            Work::Round(body, reads) => self.run_round(body, reads),
            Work::After(ms, f, span) => {
                let wait = std::time::Duration::from_millis(ms);
                self.journaled_sleep(Instant::now() + wait, span)?;
                self.call_value(f, Vec::new(), span)
            }
        }
    }

    fn run_task(mut self, work: Work<'a>, span: Span, handle: &TaskHandle<'a>) {
        let json = self.c.resolution.builtin_type("std.json", "Json");
        crate::json::set_json_type(json);
        let tasks = self.tasks.clone().expect("a task has tasks");
        let id = self.task;
        if tasks.sched.begin(id).is_ok() {
            let result = self.run_work(work);
            let result = match self.join_spawned(result, span) {
                // Whoever waits for a cancelled task gets an error; it
                // is no error nobody read.
                Err(Flow::Cancelled) => {
                    // What it held is released, and its `drop`s run (9.6).
                    self.cleaning = true;
                    let _ = self.run_drops(span);
                    self.cleaning = false;
                    let cancelled = &handle.unread.cancelled;
                    cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
                    let why = match tasks.sched.timed_out(id) {
                        Some(ms) => kavach_runtime_core::tasks::timed_out(ms),
                        None => "the task was cancelled".into(),
                    };
                    Err(Flow::Fail(Value::BuiltinError(why.into())))
                }
                // A panic stops only this task (spec 9.1): printed now, and
                // `.wait()` gets it as an error, printed nowhere else.
                Err(Flow::Panic(message, at)) => {
                    self.forget_drops(0);
                    let (path, line) = self.c.shown_location(at.start);
                    self.err
                        .push_str(&format!("panic: {message}\n  at {path}:{line}\n"));
                    let quiet = &handle.unread.cancelled;
                    quiet.store(true, std::sync::atomic::Ordering::Relaxed);
                    let why = format!("panicked: {message} at {path}:{line}");
                    kavach_runtime_core::tasks::panic_shown(&why);
                    Err(Flow::Fail(Value::BuiltinError(why.into())))
                }
                other => other,
            };
            if let Err(f @ (Flow::Panic(..) | Flow::Exit(_))) = &result {
                let mut ended = tasks.ended.lock().unwrap_or_else(|e| e.into_inner());
                if ended.is_none() {
                    *ended = Some(Baton(copy_flow(f)));
                }
                tasks.sched.stop();
            }
            self.flush();
            *handle.result.lock().unwrap_or_else(|e| e.into_inner()) = Some(Baton(result));
        }
        // Everything this task holds goes before the turn passes on.
        drop(self);
        let sched = Arc::clone(&tasks.sched);
        drop(tasks);
        sched.end(id);
    }

    /// Moves what this task printed into the program's output: onto the
    /// terminal when it is shown live.
    pub(crate) fn flush(&mut self) {
        let Some(tasks) = &self.tasks else { return };
        if tasks.live {
            use std::io::Write;
            let _ = std::io::stdout().write_all(std::mem::take(&mut self.out).as_bytes());
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().write_all(std::mem::take(&mut self.err).as_bytes());
            return;
        }
        let mut out = tasks.out.lock().unwrap_or_else(|e| e.into_inner());
        out.push_str(&std::mem::take(&mut self.out));
        let mut err = tasks.err.lock().unwrap_or_else(|e| e.into_inner());
        err.push_str(&std::mem::take(&mut self.err));
    }
}
