//! Channels (spec 9.4): values passed between tasks, in order. Both ends
//! share one channel. `put` waits while it is full, `take` while it is
//! empty; the tasks waiting on a channel go on in the order they started
//! waiting, through the scheduler (`tasks.rs`), as in native code
//! (`runtime/native/src/rt_channels.rs`).

use crate::interp::Interp;
use crate::tasks::Limit;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_runtime_core::tasks::{Scheduler, Stop, TaskId};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::sync::Arc;

pub struct ChannelState<'a> {
    values: VecDeque<Value<'a>>,
    capacity: usize,
    closed: bool,
    /// Tasks waiting for a value, and for room, first come first.
    takers: VecDeque<TaskId>,
    putters: VecDeque<TaskId>,
    /// Whether every `Receiver` was let go: no value put could be taken.
    no_receivers: bool,
    /// The scheduler of the tasks waiting, to wake them when the last
    /// receiver goes.
    sched: Option<Arc<Scheduler>>,
}

pub type Channel<'a> = Rc<RefCell<ChannelState<'a>>>;

impl std::fmt::Debug for ChannelState<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelState")
            .field("values", &self.values.len())
            .field("closed", &self.closed)
            .field("no_receivers", &self.no_receivers)
            .finish()
    }
}

/// What every copy of a `Receiver` holds: when the last copy is let go,
/// the channel has no receivers left, and a `put` waiting for room is
/// woken to panic (spec 9.4).
#[derive(Debug)]
pub struct Receiving<'a>(Weak<RefCell<ChannelState<'a>>>);

impl Drop for Receiving<'_> {
    fn drop(&mut self) {
        let Some(c) = self.0.upgrade() else { return };
        let Ok(mut st) = c.try_borrow_mut() else {
            return;
        };
        st.no_receivers = true;
        let putters = std::mem::take(&mut st.putters);
        if let Some(sched) = &st.sched {
            putters.into_iter().for_each(|t| sched.unpark(t));
        }
    }
}

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
            no_receivers: false,
            sched: None,
        }));
        let receiving = Rc::new(Receiving(Rc::downgrade(&state)));
        Ok(Value::Tuple(vec![
            Value::Channel(Rc::clone(&state), None),
            Value::Channel(state, Some(receiving)),
        ]))
    }

    /// `send.put(v)`: waits while the channel is full, for at most `limit`
    /// (`.timeout`, spec 9.6).
    /// `send.try_put(v)` (spec 9.4): in only when the channel is open and
    /// has room; never waits.
    pub(crate) fn channel_try_put(&mut self, c: &Channel<'a>, v: Value<'a>) -> bool {
        let mut st = c.borrow_mut();
        if st.closed || st.no_receivers || st.values.len() >= st.capacity {
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
            if st.no_receivers {
                drop(st);
                return Err(self.panic("`put` into a channel whose receivers are all gone", span));
            }
            if st.values.len() < st.capacity {
                st.values.push_back(v);
                let taker = st.takers.pop_front();
                drop(st);
                self.unpark(taker);
                return Ok(Value::Unit);
            }
            st.putters.push_back(self.task);
            st.sched = self.tasks.as_ref().map(|t| Arc::clone(&t.sched));
            drop(st);
            if let Err(stop) = self.park(limit) {
                let me = self.task;
                c.borrow_mut().putters.retain(|t| *t != me);
                return Err(self.wait_ended(stop, limit, span));
            }
        }
    }

    /// The next value of a channel: `None` once it is closed and empty
    /// (a `none` sent on a `Channel<T?>` is `Some(Value::None)`).
    pub(crate) fn channel_next(
        &mut self,
        c: &Channel<'a>,
        limit: Option<Limit>,
        span: Span,
    ) -> Result<Option<Value<'a>>, Flow<'a>> {
        loop {
            let mut st = c.borrow_mut();
            if let Some(v) = st.values.pop_front() {
                let putter = st.putters.pop_front();
                drop(st);
                self.unpark(putter);
                return Ok(Some(v));
            }
            if st.closed {
                return Ok(None);
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

    /// `receive.take()`: `Taken.Item(value)`, or `Taken.Closed` (spec 9.4,
    /// decision 0097).
    pub(crate) fn channel_take(
        &mut self,
        c: &Channel<'a>,
        limit: Option<Limit>,
        span: Span,
    ) -> Eval<'a> {
        Ok(match self.channel_next(c, limit, span)? {
            Some(v) => self.builtin_variant("Taken", "Item", vec![("value", v)]),
            None => self.builtin_variant("Taken", "Closed", Vec::new()),
        })
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
