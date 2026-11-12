//! Holds on a program's stop (spec 5.8, decision 0088), for the
//! interpreter and native code alike. While a hold is kept, the first
//! stop signal doesn't end the program: the signal's thread marks the
//! stop as asked and waits, until every hold is let go of or the longest
//! grace has passed, before the program ends with the signal's code.

use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// The holds kept: each one's number and grace, in milliseconds.
static HOLDS: Mutex<Vec<(u64, u64)>> = Mutex::new(Vec::new());
static NEXT: AtomicU64 = AtomicU64::new(1);
/// The exit code of the stop asked for; 0 while none has been.
static ASKED: AtomicI32 = AtomicI32::new(0);

fn holds() -> MutexGuard<'static, Vec<(u64, u64)>> {
    HOLDS.lock().unwrap_or_else(|e| e.into_inner())
}

/// A new hold with `grace_ms` to finish in: its number.
pub fn hold(grace_ms: i64) -> i64 {
    let id = NEXT.fetch_add(1, Ordering::SeqCst);
    holds().push((id, grace_ms.max(0) as u64));
    id as i64
}

/// Lets go of hold `id`.
pub fn done(id: i64) {
    holds().retain(|(h, _)| *h != id as u64);
}

/// Whether a stop has been asked for.
pub fn asked() -> bool {
    ASKED.load(Ordering::SeqCst) != 0
}

/// The exit code of the stop asked for, if one was.
pub fn asked_code() -> Option<i32> {
    match ASKED.load(Ordering::SeqCst) {
        0 => None,
        code => Some(code),
    }
}

/// A stop signal with exit `code` came: marks it asked, then waits while
/// any hold is kept, at most the longest grace. Called on the signal's
/// own thread, before the program ends.
pub fn let_holders_finish(code: i32) {
    ASKED.store(code, Ordering::SeqCst);
    let longest = holds().iter().map(|(_, g)| *g).max().unwrap_or(0);
    let until = Instant::now() + Duration::from_millis(longest);
    while !holds().is_empty() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holders_finish_or_their_grace_runs_out() {
        let id = hold(5_000);
        let waiter = std::thread::spawn(|| {
            let start = Instant::now();
            let_holders_finish(143);
            start.elapsed()
        });
        while !asked() {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(asked_code(), Some(143));
        done(id);
        let waited = waiter.join().unwrap();
        assert!(waited < Duration::from_millis(4_000), "{waited:?}");
        // No hold left: nothing to wait for.
        let start = Instant::now();
        let_holders_finish(130);
        assert!(start.elapsed() < Duration::from_millis(500));
        // A hold never let go of waits only its grace.
        let _stuck = hold(50);
        let start = Instant::now();
        let_holders_finish(130);
        assert!(start.elapsed() >= Duration::from_millis(50));
    }
}
