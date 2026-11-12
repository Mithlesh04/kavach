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
use kavach_runtime_core::tasks::{Scheduler, Stop, TaskId};
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

/// What a task runs: `spawn x`, or a function value on one item
/// (`parallel_map`, spec 9.3).
pub(crate) enum Work<'a> {
    Expr(&'a Expr),
    Call(Value<'a>, Value<'a>, Span),
    /// `time.after(d, f)` (spec 14.4): `f` after `d` milliseconds.
    After(u64, Value<'a>, Span),
    /// A durable agent's event run again after a restart (decision 0072).
    Replay(crate::journal::Replayed<'a>),
    /// `spawn process f(args)` (spec 9.8): the call, run in a child.
    Process(crate::processes::Process),
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
fn copy<'a>(r: &Eval<'a>) -> Eval<'a> {
    match r {
        Ok(v) => Ok(v.clone()),
        Err(f) => Err(copy_flow(f)),
    }
}

fn copy_flow<'a>(f: &Flow<'a>) -> Flow<'a> {
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
    /// `spawn x` (spec 9.1): a new task that will evaluate `x`.
    pub(crate) fn spawn(&mut self, x: &'a Expr) -> Eval<'a> {
        self.spawn_work(Work::Expr(x), x.span)
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
            dry_calls: 0,
            log: self.log.clone(),
            methods: self.methods.clone(),
            ext_methods: self.ext_methods.clone(),
            frames: vec![frame],
            result: Vec::new(),
            invariant_fields: Vec::new(),
            out: String::new(),
            err: String::new(),
            steps: self.steps,
            step_limit: self.step_limit,
            std_modules: self.std_modules.clone(),
            args: self.args.clone(),
            testing: self.testing,
            tasks: Some(tasks),
            task: id,
            fs_scopes: self.fs_scopes.clone(),
            gen_rng: crate::testing::values::Rng::new(0),
            random: self.random.as_ref().map(|r| r.fork(id as u64)),
            inout_out: Vec::new(),
            next_generics: None,
            leaving: false,
            fake: self.fake.clone(),
            frozen: self.frozen.clone(),
            drops: std::rc::Rc::clone(&self.drops),
            drop_types: self.drop_types.clone(),
            block_lets: Vec::new(),
            drop_floor: 0,
            // A task spawned in a handler makes its calls for the event.
            events: self.events.clone(),
        }
    }

    /// Runs task `x` on this (new) thread, when its turn comes.
    fn run_work(&mut self, work: Work<'a>) -> Eval<'a> {
        match work {
            Work::Expr(x) => match self.eval(x) {
                Err(Flow::Return(v)) => Ok(v),
                other => other,
            },
            Work::Call(f, item, span) => self.call_value(f, vec![item], span),
            // A wait like `sleep`'s: under `kavach test` the fake clock
            // moves on at once (spec 11.5).
            Work::Replay(r) => self.replay(r),
            Work::Process(p) => self.run_process(p),
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

    /// `task.wait()` (spec 9.1): the task's value, or its error. Waits
    /// for at most `limit`.
    pub(crate) fn wait_task(
        &mut self,
        t: &Arc<TaskHandle<'a>>,
        limit: Option<Limit>,
        span: Span,
    ) -> Eval<'a> {
        if let Some(sched) = self.tasks.as_ref().map(|t| Arc::clone(&t.sched)) {
            self.flush();
            let waited = match limit {
                Some((at, _)) => sched.wait_for_until(self.task, t.id, at),
                None => sched.wait_for(self.task, t.id),
            };
            if let Err(stop) = waited {
                return Err(self.wait_ended(stop, limit, span));
            }
        }
        let result = t.result.lock().unwrap_or_else(|e| e.into_inner());
        t.unread
            .read
            .store(true, std::sync::atomic::Ordering::Relaxed);
        match result.as_ref() {
            Some(Baton(r)) => copy(r),
            None => Err(self.panic("the task has no result", span)),
        }
    }

    /// `items.parallel_map(f, max: m)` (spec 9.3): each item in a task of
    /// its own; before starting task `i`, task `i - m` is waited for, so
    /// at most `m` run at once (0: no limit). The results are in order.
    pub(crate) fn parallel_map(
        &mut self,
        items: &[Value<'a>],
        f: Value<'a>,
        max: Option<Value<'a>>,
        span: Span,
    ) -> Eval<'a> {
        let max = match max {
            Some(Value::Int(n, _)) => n.max(0) as usize,
            _ => 0,
        };
        let mut started: Vec<Arc<TaskHandle<'a>>> = Vec::new();
        for (i, item) in items.iter().enumerate() {
            if max > 0 && i >= max {
                let earlier = Arc::clone(&started[i - max]);
                self.wait_task(&earlier, None, span)?;
            }
            match self.spawn_work(Work::Call(f.clone(), item.clone(), span), span)? {
                Value::Task(t) => started.push(t),
                _ => unreachable!("spawning gives a task"),
            }
        }
        let mut results = Vec::with_capacity(started.len());
        for t in &started {
            results.push(self.wait_task(t, None, span)?);
        }
        Ok(Value::List(std::rc::Rc::new(results)))
    }

    /// `time.sleep_ms` and the rounds of `time.every_ms` (spec 14.4):
    /// other tasks run meanwhile.
    pub(crate) fn sleep_until(&mut self, at: Instant, span: Span) -> Result<(), Flow<'a>> {
        let Some(sched) = self.tasks.as_ref().map(|t| Arc::clone(&t.sched)) else {
            self.refuse_if_held(span)?;
            std::thread::sleep(at.saturating_duration_since(Instant::now()));
            return Ok(());
        };
        self.flush();
        sched
            .sleep_until(self.task, at)
            .map_err(|stop| self.stopped(stop, span))
    }

    /// At a loop's round or a call: a task cancelled with `task.cancel()`
    /// stops here too, not only at its waits (spec 9.6, decision 0075).
    pub(crate) fn stop_point(&self) -> Result<(), Flow<'a>> {
        match &self.tasks {
            Some(t)
                if kavach_runtime_core::tasks::any_stop_now() && t.sched.must_stop(self.task) =>
            {
                Err(Flow::Cancelled)
            }
            _ => Ok(()),
        }
    }

    /// When a call returns, the tasks it spawned that are still running
    /// are cancelled and waited for (spec 9.2).
    pub(crate) fn join_spawned(&mut self, result: Eval<'a>, span: Span) -> Eval<'a> {
        let spawned = match self.frames.last_mut() {
            Some(f) if !f.spawned.is_empty() => std::mem::take(&mut f.spawned),
            _ => return result,
        };
        let Some(sched) = self.tasks.as_ref().map(|t| Arc::clone(&t.sched)) else {
            return result;
        };
        for t in &spawned {
            sched.cancel_with(self.task, t.id);
        }
        for t in spawned {
            if sched.is_done(t.id) {
                continue;
            }
            self.flush();
            if let Err(stop) = sched.join(self.task, t.id) {
                return Err(self.stopped(stop, span));
            }
        }
        result
    }

    /// Refuses a wait inside `Shared.update` (spec 6.2), for waits that
    /// don't go through the scheduler, which refuses its own.
    pub(crate) fn refuse_if_held(&self, span: Span) -> Result<(), Flow<'a>> {
        if kavach_runtime_core::tasks::holding() {
            return Err(self.panic(kavach_runtime_core::tasks::HELD, span));
        }
        Ok(())
    }

    /// A wait that ended without what it waited for: its time limit
    /// passed (an error), or `stopped` says why.
    pub(crate) fn wait_ended(&mut self, stop: Stop, limit: Option<Limit>, span: Span) -> Flow<'a> {
        match (stop, limit) {
            (Stop::TimedOut, Some((_, ms))) => Flow::Fail(Value::BuiltinError(
                kavach_runtime_core::tasks::timed_out(ms).into(),
            )),
            _ => self.stopped(stop, span),
        }
    }

    /// Why a task can't go on: a deadlock, or the panic that ends the
    /// program.
    pub(crate) fn stopped(&mut self, stop: Stop, span: Span) -> Flow<'a> {
        let ended = self.tasks.as_ref().and_then(|t| {
            let e = t.ended.lock().unwrap_or_else(|e| e.into_inner());
            e.as_ref().map(|Baton(f)| copy_flow(f))
        });
        match (stop, ended) {
            (Stop::Cancelled, _) => Flow::Cancelled,
            (Stop::Held, _) => self.panic(kavach_runtime_core::tasks::HELD, span),
            (Stop::Stopped, Some(f)) => f,
            _ => self.panic("every task is waiting for another one (a deadlock)", span),
        }
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
