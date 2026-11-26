// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `Shared`'s lock (decision 0052): `update` takes it and lets go of it
//! with no cell left on a thread's list, and a panic lets go only of the
//! locks its own task still holds, whatever other cores do meanwhile.

use super::{SharedCell, UPDATING, let_go_after_panic, let_go_for_update, take_for_update};
use crate::rt_tasks::ME;
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn cell() -> Arc<SharedCell> {
    Arc::new(SharedCell {
        value: Cell::new(0),
        owner: AtomicUsize::new(0),
        depth: Cell::new(0),
    })
}

/// How many cells this thread's list holds.
fn listed() -> usize {
    UPDATING.with(|u| u.borrow().len())
}

/// A cell left on this thread's list whose lock another task holds now:
/// a panic here leaves it locked for that task.
#[test]
fn a_panic_lets_go_only_of_the_locks_its_task_holds() {
    ME.set(7);
    let theirs = cell();
    theirs.owner.store(9 + 1, Ordering::Release);
    theirs.depth.set(1);
    UPDATING.with(|u| u.borrow_mut().push(Arc::clone(&theirs)));
    let mine = cell();
    take_for_update(&mine);
    let_go_after_panic();
    assert_eq!(theirs.owner.load(Ordering::Acquire), 10, "theirs let go");
    assert_eq!(theirs.depth.get(), 1);
    assert_eq!(mine.owner.load(Ordering::Acquire), 0, "mine kept");
    assert_eq!(mine.depth.get(), 0);
    assert_eq!(listed(), 0);
}

/// Many cores updating one value: each takes the cell off its list when
/// it lets go of the lock, so no list keeps it.
#[test]
fn updates_on_many_cores_leave_no_cell_on_a_list() {
    let c = cell();
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let c = Arc::clone(&c);
            std::thread::spawn(move || {
                ME.set(100 + i);
                for _ in 0..50_000 {
                    take_for_update(&c);
                    c.value.set(c.value.get() + 1);
                    let_go_for_update(&c);
                }
                listed()
            })
        })
        .collect();
    let left: Vec<usize> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(left, vec![0; 8]);
    assert_eq!(c.value.get(), 400_000);
}
