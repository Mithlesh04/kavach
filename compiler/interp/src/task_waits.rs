// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Waits in a task (spec 9, decision 0020): for another task, until a
//! time, and for the tasks a call spawned; the points where a cancelled
//! task stops; and why a wait or a task ends early.

use crate::baton::Baton;
use crate::interp::Interp;
use crate::tasks::{Limit, TaskHandle, copy, copy_flow};
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_runtime_core::tasks::Stop;
use std::sync::Arc;
use std::time::Instant;

impl<'a> Interp<'a> {
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
        if self.cleaning {
            return Ok(());
        }
        match &self.tasks {
            Some(t) if t.stop.load(std::sync::atomic::Ordering::Relaxed) => {
                Err(Flow::Exit(crate::STOPPED))
            }
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
}
