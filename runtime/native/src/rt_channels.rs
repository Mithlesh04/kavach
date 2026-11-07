//! Channels in native code (spec 9.4), as in the interpreter
//! (`compiler/interp/src/channels.rs`): values passed between tasks in
//! order. `put` waits while the channel is full, `take` while it is
//! empty; the tasks waiting go on in the order they started waiting.
//! Both ends are the one channel object.

use crate::heap::{Data, alloc, alloc_slots, release, release_slot};
use crate::rt::{panic_with, type_of};
use crate::shapes::shape;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_core::tasks::{TaskId, many_workers};
use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};

/// A channel: its state behind a lock, since with tasks on all cores
/// (decision 0049) tasks on several workers use it at once.
pub struct ChanObj {
    state: Mutex<ChanState>,
    capacity: usize,
}

struct ChanState {
    values: VecDeque<u64>,
    closed: bool,
    /// Tasks waiting for a value, and for room, first come first, with
    /// the wait each started (`Waiter`).
    takers: VecDeque<Waiter>,
    putters: VecDeque<Waiter>,
}

/// A waiting task, and the number of its wait: with several workers, a
/// wake-up can come just before the task starts waiting, and must still
/// end that wait (`wake_wait`).
type Waiter = (TaskId, u64);

impl ChanObj {
    fn lock(&self) -> MutexGuard<'_, ChanState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// This task, as a waiter about to wait.
fn waiter() -> Waiter {
    let me = crate::rt_tasks::me();
    let wait = if many_workers() {
        crate::rt_tasks::scheduler().next_wait(me)
    } else {
        0
    };
    (me, wait)
}

/// Ends the wait of `w`. With one worker only the task holding the turn
/// runs, so the waiter is waiting already.
fn wake((task, wait): Waiter) {
    if many_workers() {
        crate::rt_tasks::scheduler().wake_wait(task, wait);
    } else {
        crate::rt_tasks::unpark(task);
    }
}

/// A value going into a channel: with several workers, the task taking it
/// may run on another thread, so it gets a copy that shares no object
/// with the sender (`send.rs`).
fn sent(value: i64, item: Type, line: i32) -> u64 {
    if !many_workers() || !item.is_ref() {
        return value as u64;
    }
    if !crate::send::copyable(value as u64, item) {
        panic_with("this value can't go to a task on another core", line);
    }
    let copy = crate::send::copy(value as u64, item);
    release_slot(value, item);
    copy
}

/// The channel ops (all operands owned). `code` is the result type.
pub fn chan_op(op: RtOp, a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    if op == RtOp::ChanNew {
        return new(a, code, line);
    }
    crate::rt_tasks::turn();
    let result = match op {
        RtOp::ChanPut => {
            put(a, b, None, line);
            0
        }
        // `.timeout(d)`: the milliseconds come last.
        RtOp::ChanPutFor => {
            put(a, b, crate::rt_tasks::limit(c, line), line);
            0
        }
        RtOp::ChanTakeFor => take(a, code, crate::rt_tasks::limit(b, line), line),
        RtOp::ChanClose => {
            let waiting: Vec<Waiter> = with(a, |c| {
                let mut st = c.lock();
                st.closed = true;
                let takers = std::mem::take(&mut st.takers);
                let putters = std::mem::take(&mut st.putters);
                takers.into_iter().chain(putters).collect()
            });
            waiting.into_iter().for_each(wake);
            0
        }
        _ => take(a, code, None, line),
    };
    release(a);
    result
}

/// `Channel<T>(capacity: n)`: a tuple (of type `code`) of both ends.
fn new(capacity: i64, code: i64, line: i32) -> i64 {
    if capacity < 1 {
        panic_with(
            &format!("a channel's capacity must be at least 1, not {capacity}"),
            line,
        );
    }
    let tuple = type_of(code);
    let end = match shape(tuple) {
        Shape::Tuple(ts) => ts[0],
        _ => Type::Unit,
    };
    let chan = alloc(
        end,
        Data::Channel(std::sync::Arc::new(ChanObj {
            state: Mutex::new(ChanState {
                values: VecDeque::new(),
                closed: false,
                takers: VecDeque::new(),
                putters: VecDeque::new(),
            }),
            capacity: capacity as usize,
        })),
    );
    crate::heap::retain(chan);
    alloc_slots(tuple, 0, vec![chan as u64, chan as u64])
}

/// `put` (the value owned): waits while the channel is full.
fn put(chan: i64, value: i64, limit: crate::rt_tasks::Limit, line: i32) {
    let item = item_type(chan);
    let value = sent(value, item, line);
    loop {
        let me = waiter();
        // Room: the value goes in, and the first taker waiting wakes.
        // No room: this task waits, noted under the same lock, so a
        // taker that makes room finds it.
        let (done, taker) = with(chan, |c| {
            let mut st = c.lock();
            if st.closed {
                drop(st);
                panic_with("`put` into a closed channel", line);
            }
            if st.values.len() < c.capacity {
                st.values.push_back(value);
                (true, st.takers.pop_front())
            } else {
                st.putters.push_back(me);
                (false, None)
            }
        });
        if let Some(t) = taker {
            wake(t);
        }
        if done {
            return;
        }
        if !crate::rt_tasks::park(limit, line) {
            // Cancelled, or out of time: the value is let go of.
            with(chan, |c| c.lock().putters.retain(|w| w.0 != me.0));
            release_slot(value as i64, item);
            return;
        }
    }
}

/// The next value as an optional of type `code`, or `none` once the
/// channel is closed and empty.
fn take(chan: i64, code: i64, limit: crate::rt_tasks::Limit, line: i32) -> i64 {
    loop {
        let me = waiter();
        let (value, putter, closed) = with(chan, |c| {
            let mut st = c.lock();
            let v = st.values.pop_front();
            let p = v.and_then(|_| st.putters.pop_front());
            if v.is_none() && !st.closed {
                st.takers.push_back(me);
            }
            (v, p, st.closed)
        });
        if let Some(t) = putter {
            wake(t);
        }
        if let Some(v) = value {
            return crate::rt_data::optional(Some(v as i64), code);
        }
        if closed {
            return 0;
        }
        if !crate::rt_tasks::park(limit, line) {
            with(chan, |c| c.lock().takers.retain(|w| w.0 != me.0));
            return 0;
        }
    }
}

fn item_type(chan: i64) -> Type {
    match shape(crate::heap::obj(chan).ty()) {
        Shape::Channel(t) => *t,
        _ => Type::Unit,
    }
}

fn with<R>(chan: i64, f: impl FnOnce(&ChanObj) -> R) -> R {
    match &*crate::heap::data(chan).borrow() {
        Data::Channel(c) => f(c),
        _ => unreachable!("a channel object"),
    }
}

/// Releases the values still in a channel, when it is freed.
pub fn free_chan(c: &ChanObj, item: Type) {
    for v in c.lock().values.drain(..) {
        release_slot(v as i64, item);
    }
}
