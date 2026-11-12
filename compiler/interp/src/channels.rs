//! Channels (spec 9.4): values passed between tasks, in order. Both ends
//! share one channel. `put` waits while it is full, `take` while it is
//! empty; the tasks waiting on a channel go on in the order they started
//! waiting, through the scheduler (`tasks.rs`), as in native code
//! (`runtime/native/src/rt_channels.rs`).

use crate::interp::Interp;
use crate::tasks::Limit;
use crate::value::{Eval, Value};
use kavach_ast::Span;
use kavach_runtime_core::tasks::{Stop, TaskId};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Debug)]
pub struct ChannelState<'a> {
    values: VecDeque<Value<'a>>,
    capacity: usize,
    closed: bool,
    /// Tasks waiting for a value, and for room, first come first.
    takers: VecDeque<TaskId>,
    putters: VecDeque<TaskId>,
}

pub type Channel<'a> = Rc<RefCell<ChannelState<'a>>>;

impl<'a> Interp<'a> {
    /// `Channel<T>(capacity: n)`: its sender and its receiver.
    pub(crate) fn new_channel(&mut self, capacity: Value<'a>, span: Span) -> Eval<'a> {
        let capacity = match capacity {
            Value::Int(n, _) if n >= 1 => n.min(usize::MAX as i128) as usize,
            other => {
                let n = other.display();
                return Err(self.panic(
                    format!("a channel's capacity must be at least 1, not {n}"),
                    span,
                ));
            }
        };
        let state = Rc::new(RefCell::new(ChannelState {
            values: VecDeque::new(),
            capacity,
            closed: false,
            takers: VecDeque::new(),
            putters: VecDeque::new(),
        }));
        Ok(Value::Tuple(vec![
            Value::Channel(Rc::clone(&state), true),
            Value::Channel(state, false),
        ]))
    }

    /// `send.put(v)`: waits while the channel is full, for at most `limit`
    /// (`.timeout`, spec 9.6).
    /// `send.try_put(v)` (spec 9.4): in only when the channel is open and
    /// has room; never waits.
    pub(crate) fn channel_try_put(&mut self, c: &Channel<'a>, v: Value<'a>) -> bool {
        let mut st = c.borrow_mut();
        if st.closed || st.values.len() >= st.capacity {
            return false;
        }
        st.values.push_back(v);
        let taker = st.takers.pop_front();
        drop(st);
        self.unpark(taker);
        true
    }

    pub(crate) fn channel_put(
        &mut self,
        c: &Channel<'a>,
        v: Value<'a>,
        limit: Option<Limit>,
        span: Span,
    ) -> Eval<'a> {
        loop {
            let mut st = c.borrow_mut();
            if st.closed {
                drop(st);
                return Err(self.panic("`put` into a closed channel", span));
            }
            if st.values.len() < st.capacity {
                st.values.push_back(v);
                let taker = st.takers.pop_front();
                drop(st);
                self.unpark(taker);
                return Ok(Value::Unit);
            }
            st.putters.push_back(self.task);
            drop(st);
            if let Err(stop) = self.park(limit) {
                let me = self.task;
                c.borrow_mut().putters.retain(|t| *t != me);
                return Err(self.wait_ended(stop, limit, span));
            }
        }
    }

    /// `receive.take()`: the next value, or `none` once the channel is
    /// closed and empty. Waits while it is empty, for at most `limit`.
    pub(crate) fn channel_take(
        &mut self,
        c: &Channel<'a>,
        limit: Option<Limit>,
        span: Span,
    ) -> Eval<'a> {
        loop {
            let mut st = c.borrow_mut();
            if let Some(v) = st.values.pop_front() {
                let putter = st.putters.pop_front();
                drop(st);
                self.unpark(putter);
                return Ok(v);
            }
            if st.closed {
                return Ok(Value::None);
            }
            st.takers.push_back(self.task);
            drop(st);
            if let Err(stop) = self.park(limit) {
                let me = self.task;
                c.borrow_mut().takers.retain(|t| *t != me);
                return Err(self.wait_ended(stop, limit, span));
            }
        }
    }

    /// `send.close()`: every task waiting on the channel goes on.
    pub(crate) fn channel_close(&mut self, c: &Channel<'a>) -> Eval<'a> {
        let waiting: Vec<TaskId> = {
            let mut st = c.borrow_mut();
            st.closed = true;
            let takers = std::mem::take(&mut st.takers);
            let putters = std::mem::take(&mut st.putters);
            takers.into_iter().chain(putters).collect()
        };
        for t in waiting {
            self.unpark(Some(t));
        }
        Ok(Value::Unit)
    }

    fn unpark(&self, task: Option<TaskId>) {
        if let (Some(t), Some(tasks)) = (task, &self.tasks) {
            tasks.sched.unpark(t);
        }
    }

    /// Waits until another task wakes this one, or `limit` passes.
    /// Without a scheduler no other task could, so that is a deadlock.
    fn park(&mut self, limit: Option<Limit>) -> Result<(), Stop> {
        let Some(sched) = self.tasks.as_ref().map(|t| std::sync::Arc::clone(&t.sched)) else {
            return Err(Stop::Deadlock);
        };
        self.flush();
        match limit {
            Some((at, _)) => sched.park_until(self.task, at),
            None => sched.park(self.task),
        }
    }
}
