// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Work in the GPU's driver on threads of their own: opening a device
//! for later, compiling kernels ahead. The program waits for it before it
//! ends (`settle`). A process that ends while a thread is inside the
//! driver can hang in the driver's own clean-up, for good: an open of a
//! laptop's second GPU, cut off by a short program's end, did so about
//! once in twenty runs.

use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// How many such threads haven't finished.
static UNDER_WAY: Mutex<usize> = Mutex::new(0);
static FINISHED: Condvar = Condvar::new();

fn under_way() -> MutexGuard<'static, usize> {
    UNDER_WAY.lock().unwrap_or_else(|e| e.into_inner())
}

/// Counts one thread down when it ends, even by a panic, or when it
/// couldn't start.
struct Counted;

impl Drop for Counted {
    fn drop(&mut self) {
        *under_way() -= 1;
        FINISHED.notify_all();
    }
}

/// Runs `f` on a thread of its own, counted until it ends.
pub fn in_background(f: impl FnOnce() + Send + 'static) {
    *under_way() += 1;
    let counted = Counted;
    // A thread that can't start drops `f` and `counted` with it.
    let _ = std::thread::Builder::new().spawn(move || {
        let _counted = counted;
        f();
    });
}

/// Waits until no such thread runs, at most `limit`: whether none does.
/// For the program's end, so it never ends with one inside the driver.
pub fn settle(limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    let mut n = under_way();
    while *n > 0 {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return false;
        }
        n = FINISHED
            .wait_timeout(n, left)
            .unwrap_or_else(|e| e.into_inner())
            .0;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One test: the count is the program's own. Other tests may open a
    /// device in the background too, so it is never checked to be 0.
    #[test]
    fn settle_waits_for_threads_under_way_up_to_its_limit() {
        let (go, wait) = std::sync::mpsc::channel::<()>();
        in_background(move || {
            let _ = wait.recv();
        });
        assert!(
            !settle(Duration::from_millis(50)),
            "the thread is still blocked"
        );
        in_background(|| std::thread::sleep(Duration::from_millis(100)));
        go.send(()).unwrap();
        let started = Instant::now();
        assert!(settle(Duration::from_secs(10)));
        assert!(started.elapsed() >= Duration::from_millis(50));
        in_background(|| panic!("a driver call that panics"));
        assert!(settle(Duration::from_secs(10)), "a panic counts down too");
    }
}
