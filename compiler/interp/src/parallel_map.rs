//! `items.parallel_map(f, max: m)` (spec 9.3) in the interpreter: a task
//! for each item, then each result in order.

use crate::interp::Interp;
use crate::tasks::{TaskHandle, Work};
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use std::sync::Arc;

impl<'a> Interp<'a> {
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
        // The first error, in the items' order, is the call's (spec 5.2):
        // the other tasks' errors are no one else's to read.
        let read_all = |started: &[Arc<TaskHandle<'a>>]| {
            for t in started {
                t.unread
                    .read
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
        };
        for (i, item) in items.iter().enumerate() {
            if max > 0 && i >= max {
                let earlier = Arc::clone(&started[i - max]);
                if let Err(e) = self.wait_task(&earlier, None, span) {
                    read_all(&started);
                    return Err(self.passed_on(&earlier, e, span));
                }
            }
            match self.spawn_work(Work::Call(f.clone(), item.clone(), span), span)? {
                Value::Task(t) => started.push(t),
                _ => unreachable!("spawning gives a task"),
            }
        }
        let mut results = Vec::with_capacity(started.len());
        for t in &started {
            match self.wait_task(t, None, span) {
                Ok(v) => results.push(v),
                Err(e) => {
                    read_all(&started);
                    return Err(self.passed_on(t, e, span));
                }
            }
        }
        Ok(Value::List(std::rc::Rc::new(results)))
    }

    /// What task `t`'s ending `e` is for the call: its function's error
    /// as it is, and a panic as a panic here (spec 9.3), since the call
    /// can't fail by its type.
    fn passed_on(&self, t: &TaskHandle<'a>, e: Flow<'a>, span: Span) -> Flow<'a> {
        // A task that panicked is marked so its error isn't printed again.
        let marked = t
            .unread
            .cancelled
            .load(std::sync::atomic::Ordering::Relaxed);
        match e {
            Flow::Fail(Value::BuiltinError(why)) if marked && why.starts_with("panicked: ") => {
                self.panic(format!("an item of `parallel_map` {why}"), span)
            }
            other => other,
        }
    }
}
