// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Time finer than a millisecond (spec 14.4): a nanosecond clock that
//! never jumps, and the last stretch of a precise wait. Systems wake a
//! sleeping thread only so finely (about a millisecond on Windows), so a
//! precise wait sleeps until a millisecond before and then spins.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// When this program's nanosecond clock started.
fn start() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

/// Nanoseconds since an unspecified start; never goes back.
pub fn monotonic_ns() -> i64 {
    i64::try_from(start().elapsed().as_nanos()).unwrap_or(i64::MAX)
}

/// How long to sleep before spinning out a wait of `ns`, and the moment
/// it ends.
pub fn plan(ns: i64) -> (Duration, Instant) {
    let wait = Duration::from_nanos(ns.max(0) as u64);
    let end = Instant::now() + wait;
    (wait.saturating_sub(Duration::from_millis(1)), end)
}

/// Spins until `end`, giving way to other threads.
pub fn spin_until(end: Instant) {
    while Instant::now() < end {
        std::hint::spin_loop();
        std::thread::yield_now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_precise_wait_ends_on_time() {
        let before = monotonic_ns();
        let (sleep, end) = plan(3_000_000);
        assert!(sleep <= Duration::from_millis(2));
        std::thread::sleep(sleep);
        spin_until(end);
        let waited = monotonic_ns() - before;
        assert!((3_000_000..30_000_000).contains(&waited), "{waited} ns");
        assert!(monotonic_ns() >= before);
    }
}
