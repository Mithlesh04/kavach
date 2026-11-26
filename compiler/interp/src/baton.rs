// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! How values cross between task threads (spec 9, decision 0020).
//!
//! Values hold `Rc`s, whose counts aren't atomic, so Rust lets no value
//! go to another thread. Tasks take turns: only the task holding the
//! scheduler's turn runs, and the turn passes through a mutex, which
//! orders memory between the threads. A `Baton` carries values from one
//! task's thread to another's, and the rule that makes that sound is:
//! **a thread touches a value only while its task holds the turn**:
//! makes it, clones it, reads it, or lets it go.
//!
//! This is the interpreter's one `unsafe`, outside the places AGENTS.md
//! section 8 allows; it waits for a decision record (unsafe audit,
//! `docs/architecture/unsafe-audit.md`, (a) 5).

use crate::interp::Interp;
use crate::tasks::{MadeShared, TaskHandle, Work};
use crate::value::{Eval, Flow};
use kavach_runtime_core::tasks::{Scheduler, TaskId};
use std::sync::Arc;

/// Values moved to, or read from, another task's thread: only kinds that
/// are [`Crosses`].
pub(crate) struct Baton<T>(pub(crate) T);

/// What a `Baton` may carry. Each kind says where it is made, read and
/// let go of; every one of those places holds the turn.
pub(crate) trait Crosses {}

/// `Tasks::ended`, what ended the program: set by a task holding the turn
/// (`run_task`), copied by waiters holding it (`task_waits.rs`), and let
/// go of with the last `Tasks`, which each task drops before `end_turn`
/// (`run_task`) and the main task before its own (`on_big_stack`).
impl Crosses for Flow<'_> {}

/// `Tasks::made`, each `Shared` made: pushed by a task holding the turn
/// (`made_shared`). It holds a `Weak`, whose count isn't atomic either.
/// The list outlives every task thread: `on_big_stack` reads it and lets
/// it go after the thread scope has joined them all.
impl Crosses for MadeShared<'_> {}

/// `TaskHandle::result`: set by its task holding the turn (`run_task`),
/// copied by waiters holding it (`task_waits.rs`), and let go of with the
/// last `Arc<TaskHandle>`. Its holders are `Value::Task`s, frames'
/// `spawned` lists, and `run_task`'s own, which it drops before
/// `end_turn`. `Drop for TaskHandle` checks this in debug builds.
impl Crosses for Eval<'_> {}

/// A new task (`spawn_work`): made by the spawner holding the turn, then
/// sent through a channel, which orders memory. The loop in
/// `on_big_stack` only moves it into a new thread. There it is opened
/// before `begin` (a move, which changes no count), and `run_task` lets
/// all of it go before `end_turn`. If the send fails, the spawner drops
/// it, still holding the turn; if no thread can be started, the scope's
/// thread drops it, but the run panics then anyway.
impl<'a> Crosses for (Interp<'a>, Arc<TaskHandle<'a>>, Work<'a>) {}

// SAFETY: a `Baton`'s contents are only made, read and let go of by the
// thread holding the turn, which passed to it through a mutex; each kind
// it may carry says where (the `Crosses` impls above).
#[allow(unsafe_code)]
unsafe impl<T: Crosses> Send for Baton<T> {}

impl<T> Baton<T> {
    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}

thread_local! {
    /// This thread's task has given the turn up for good (`end_turn`).
    static GIVEN_UP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Gives task `id`'s turn up for good: this thread runs no Kavach code
/// and touches no value after it.
pub(crate) fn end_turn(sched: &Scheduler, id: TaskId) {
    GIVEN_UP.with(|g| g.set(true));
    sched.end(id);
}

/// Whether this thread has given its task's turn up for good, so a value
/// it let go of now would race with the task holding the turn.
pub(crate) fn turn_given_up() -> bool {
    GIVEN_UP.with(|g| g.get())
}
