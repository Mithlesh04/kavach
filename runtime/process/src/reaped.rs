// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! When a program's group may still be signalled (decision 0105). On
//! Linux and macOS a program's tree is a process group named by its first
//! program's id. Until that program is reaped, ended or not, the id is
//! its own, so a signal to the group reaches only the tree. Once it is
//! reaped and the group is empty, the id is free: the next program to get
//! it leads a group of the same number, and a late signal would end that
//! stranger's tree. So the reap and every signal take one lock, and no
//! signal is sent after the reap.

use std::sync::{Mutex, MutexGuard};

/// Whether a program's first process was reaped.
#[derive(Default)]
pub struct Leader {
    reaped: Mutex<bool>,
}

impl Leader {
    fn lock(&self) -> MutexGuard<'_, bool> {
        self.reaped.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Runs `signal` unless the program was reaped; whether it ran.
    pub fn signal(&self, signal: impl FnOnce()) -> bool {
        let reaped = self.lock();
        if *reaped {
            return false;
        }
        signal();
        true
    }

    /// Runs `reap`, which reaps the program (or finds it can't be); no
    /// signal is sent after it.
    pub fn reap<T>(&self, reap: impl FnOnce() -> T) -> T {
        let mut reaped = self.lock();
        let out = reap();
        *reaped = true;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    #[test]
    fn a_group_is_signalled_until_its_program_is_reaped_never_after() {
        let leader = Leader::default();
        let mut sent = 0;
        assert!(leader.signal(|| sent += 1), "while it runs");
        assert!(leader.signal(|| sent += 1), "again: it is not reaped");
        assert_eq!(leader.reap(|| 7), 7);
        assert!(!leader.signal(|| sent += 1), "its id may be a stranger's");
        assert_eq!(sent, 2);
    }

    #[test]
    fn a_signal_waits_for_a_reap_under_way_then_is_not_sent() {
        let leader = Arc::new(Leader::default());
        let reaping = Arc::new(AtomicBool::new(false));
        let (l, r) = (Arc::clone(&leader), Arc::clone(&reaping));
        let reaper = std::thread::spawn(move || {
            l.reap(|| {
                r.store(true, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(100));
            })
        });
        while !reaping.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        // The reap holds the lock: the signal waits, then finds it reaped.
        assert!(!leader.signal(|| panic!("signalled after the reap")));
        reaper.join().unwrap();
    }
}
