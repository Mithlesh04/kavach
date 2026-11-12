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
use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};

/// A channel: its state behind a lock, since with tasks on all cores
/// (decision 0049) tasks on several workers use it at once.
pub struct ChanObj {
    state: Spin<ChanState>,
    capacity: usize,
}

/// A lock held only while a value or a waiter goes in or out, a few
/// nanoseconds: taking it is one atomic exchange and letting it go one
/// store, where a `Mutex` lets go with another exchange, as it must look
/// for sleepers. A task that finds it held spins, and gives the core up
/// after a while in case the holder's thread was paused.
struct Spin<T> {
    held: AtomicBool,
    value: UnsafeCell<T>,
}

// SAFETY: the value is reached only through a `Held`, and `held` lets one
// `Held` exist at a time, so threads never touch it at once.
#[allow(unsafe_code)]
unsafe impl<T: Send> Sync for Spin<T> {}

struct Held<'a, T>(&'a Spin<T>);

impl<T> Spin<T> {
    fn new(value: T) -> Self {
        Spin {
            held: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    fn lock(&self) -> Held<'_, T> {
        let mut tries = 0u32;
        while self
            .held
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            tries += 1;
            if tries < 100 {
                std::hint::spin_loop();
            } else {
                std::thread::yield_now();
            }
        }
        Held(self)
    }
}

impl<T> std::ops::Deref for Held<'_, T> {
    type Target = T;
    #[allow(unsafe_code)]
    fn deref(&self) -> &T {
        // SAFETY: this `Held` is the only one (see `Sync` above).
        unsafe { &*self.0.value.get() }
    }
}

impl<T> std::ops::DerefMut for Held<'_, T> {
    #[allow(unsafe_code)]
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: this `Held` is the only one (see `Sync` above).
        unsafe { &mut *self.0.value.get() }
    }
}

impl<T> Drop for Held<'_, T> {
    fn drop(&mut self) {
        self.0.held.store(false, Ordering::Release);
    }
}

struct ChanState {
    values: VecDeque<u64>,
    closed: bool,
    /// Tasks waiting for a value, and for room, first come first, with
    /// the wait each started (`Waiter`).
    takers: VecDeque<Waiter>,
    putters: VecDeque<Waiter>,
    /// With several workers, a putter woken that hasn't tried again yet.
    /// While one is on its way no other is woken: the first to run fills
    /// the room, so more wake-ups would mostly find the channel full, and
    /// each costs the taker a system call. A woken putter that leaves room
    /// wakes the next itself, so room and a putter waiting always mean a
    /// wake-up is on its way.
    woken: Option<TaskId>,
}

impl ChanState {
    /// A putter to wake now that there is room, if any should be.
    /// `many`: whether several workers run tasks (`many_workers`).
    fn putter_to_wake(&mut self, capacity: usize, many: bool) -> Option<Waiter> {
        if !many {
            return self.putters.pop_front();
        }
        if self.woken.is_some() || self.values.len() >= capacity {
            return None;
        }
        let p = self.putters.pop_front()?;
        self.woken = Some(p.0);
        Some(p)
    }
}

/// A waiting task, and the number of its wait: with several workers, a
/// wake-up can come just before the task starts waiting, and must still
/// end that wait (`wake_wait`).
type Waiter = (TaskId, u64);

impl ChanObj {
    fn lock(&self) -> Held<'_, ChanState> {
        self.state.lock()
    }
}

/// This task, as a waiter about to wait.
fn waiter(many: bool) -> Waiter {
    let me = crate::rt_tasks::me();
    let wait = if many {
        crate::rt_tasks::scheduler().next_wait(me)
    } else {
        0
    };
    (me, wait)
}

/// Ends the wait of `w`. With one worker only the task holding the turn
/// runs, so the waiter is waiting already.
fn wake(many: bool, (task, wait): Waiter) {
    if many {
        crate::rt_tasks::scheduler().wake_wait(task, wait);
    } else {
        crate::rt_tasks::unpark(task);
    }
}

/// A value going into a channel: with several workers, the task taking it
/// may run on another thread, so it gets a copy that shares no object
/// with the sender (`send.rs`).
fn sent(value: i64, item: Type, many: bool, line: i32) -> u64 {
    if !many || !item.is_ref() {
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
    if op == RtOp::ChanEnded {
        return ENDED.get() as i64;
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
        RtOp::ChanTryPut => i64::from(try_put(a, b, line)),
        RtOp::ChanTakeFor => take(a, code, crate::rt_tasks::limit(b, line), line),
        RtOp::ChanTakeValue => {
            let v = next(a, None, line);
            ENDED.set(v.is_none());
            v.unwrap_or(0) as i64
        }
        RtOp::ChanClose => {
            let waiting: Vec<Waiter> = with(a, |c| {
                let mut st = c.lock();
                st.closed = true;
                let takers = std::mem::take(&mut st.takers);
                let putters = std::mem::take(&mut st.putters);
                takers.into_iter().chain(putters).collect()
            });
            let many = many_workers();
            waiting.into_iter().for_each(|w| wake(many, w));
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
            state: Spin::new(ChanState {
                values: VecDeque::new(),
                closed: false,
                takers: VecDeque::new(),
                putters: VecDeque::new(),
                woken: None,
            }),
            capacity: capacity as usize,
        })),
    );
    crate::heap::retain(chan);
    alloc_slots(tuple, 0, vec![chan as u64, chan as u64])
}

/// `put` (the value owned): waits while the channel is full.
/// `send.try_put(v)` (spec 9.4): `value` goes in only when the channel is
/// open and has room; never waits. Whether it went in.
fn try_put(chan: i64, value: i64, line: i32) -> bool {
    let item = item_type(chan);
    let many = many_workers();
    let value = sent(value, item, many, line);
    let (done, taker, putter) = with(chan, |c| {
        let mut st = c.lock();
        if st.closed || st.values.len() >= c.capacity {
            return (false, None, None);
        }
        st.values.push_back(value);
        let next = if many {
            st.putter_to_wake(c.capacity, many)
        } else {
            None
        };
        (true, st.takers.pop_front(), next)
    });
    taker.into_iter().chain(putter).for_each(|w| wake(many, w));
    if !done {
        release_slot(value as i64, item);
    }
    done
}

fn put(chan: i64, value: i64, limit: crate::rt_tasks::Limit, line: i32) {
    let item = item_type(chan);
    let many = many_workers();
    let value = sent(value, item, many, line);
    loop {
        let me = waiter(many);
        // Room: the value goes in, and the first taker waiting wakes.
        // No room: this task waits, noted under the same lock, so a
        // taker that makes room finds it.
        let (done, taker, putter) = with(chan, |c| {
            let mut st = c.lock();
            if st.closed {
                drop(st);
                panic_with("`put` into a closed channel", line);
            }
            if st.woken == Some(me.0) {
                st.woken = None;
            }
            if st.values.len() < c.capacity {
                st.values.push_back(value);
                let next = if many {
                    st.putter_to_wake(c.capacity, many)
                } else {
                    None
                };
                (true, st.takers.pop_front(), next)
            } else {
                st.putters.push_back(me);
                (false, None, None)
            }
        });
        taker.into_iter().chain(putter).for_each(|w| wake(many, w));
        if done {
            return;
        }
        if !crate::rt_tasks::park(limit, line) {
            // Cancelled, or out of time: the value is let go of, and a
            // wake-up meant for this task goes to the next.
            let next = with(chan, |c| {
                let mut st = c.lock();
                st.putters.retain(|w| w.0 != me.0);
                if st.woken != Some(me.0) {
                    return None;
                }
                st.woken = None;
                st.putter_to_wake(c.capacity, many)
            });
            next.into_iter().for_each(|w| wake(many, w));
            release_slot(value as i64, item);
            return;
        }
    }
}

thread_local! {
    /// Whether the last `ChanTakeValue` found the channel ended.
    static ENDED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The next value as an optional of type `code`, or `none` once the
/// channel is closed and empty.
fn take(chan: i64, code: i64, limit: crate::rt_tasks::Limit, line: i32) -> i64 {
    next(chan, limit, line).map_or(0, |v| crate::rt_data::optional(Some(v as i64), code))
}

/// The next value, or `None` once the channel is closed and empty, or the
/// wait was cancelled or ran out of time.
fn next(chan: i64, limit: crate::rt_tasks::Limit, line: i32) -> Option<u64> {
    let many = many_workers();
    loop {
        let me = waiter(many);
        let (value, putter, closed) = with(chan, |c| {
            let mut st = c.lock();
            let v = st.values.pop_front();
            let p = v.and_then(|_| st.putter_to_wake(c.capacity, many));
            if v.is_none() && !st.closed {
                st.takers.push_back(me);
            }
            (v, p, st.closed)
        });
        if let Some(t) = putter {
            wake(many, t);
        }
        if value.is_some() || closed {
            return value;
        }
        if !crate::rt_tasks::park(limit, line) {
            with(chan, |c| c.lock().takers.retain(|w| w.0 != me.0));
            return None;
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
