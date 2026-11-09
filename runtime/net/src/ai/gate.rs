//! The gate every `ask` of the program passes (spec 8.4), and the waits
//! between tries. Many `ask`s at once meet the provider's limit on calls
//! at the same time: 300 started together, each sent again after the
//! same 1, 2, 4 and 8 seconds, came back together and were turned away
//! together, and 276 failed (benchmarks/agents). After a call is turned
//! away as busy, no more calls go out at once than the others under way
//! then, which the provider took; each answer lets a little more through.
//! Each wait is a random part of its step, so calls turned away together
//! come back apart.

use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// The most calls under way at once.
pub(super) const MOST: f64 = 256.0;

pub(super) struct Gate {
    state: Mutex<State>,
    free: Condvar,
}

struct State {
    under_way: usize,
    /// How many may be under way: lowered when the provider is busy,
    /// raised by one over as many answers as there are calls allowed.
    limit: f64,
}

/// The program's one gate.
pub(super) static GATE: Gate = Gate::new();

impl Gate {
    pub(super) const fn new() -> Gate {
        Gate {
            state: Mutex::new(State {
                under_way: 0,
                limit: MOST,
            }),
            free: Condvar::new(),
        }
    }

    /// Waits for room, then counts a call as under way.
    pub(super) fn enter(&self) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        while s.under_way as f64 >= s.limit.floor().max(1.0) {
            s = self.free.wait(s).unwrap_or_else(|e| e.into_inner());
        }
        s.under_way += 1;
    }

    /// A call ended: turned away as busy (status 429), answered, or
    /// neither (another failure, which changes nothing).
    pub(super) fn leave(&self, busy: bool, answered: bool) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.under_way = s.under_way.saturating_sub(1);
        if busy {
            // The provider took the others under way, not this one.
            s.limit = s.limit.min(s.under_way.max(1) as f64);
        } else if answered {
            s.limit = (s.limit + 1.0 / s.limit).min(MOST);
        }
        drop(s);
        self.free.notify_all();
    }

    #[cfg(test)]
    pub(super) fn limit(&self) -> f64 {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).limit
    }
}

/// The wait before try `k + 1`: a random time between half and all of
/// 1, 2, 4 and 8 seconds, then 8 again.
pub(super) fn wait_before(k: usize) -> Duration {
    let step = 1000u64 << k.min(3);
    Duration::from_millis(step / 2 + random() % (step / 2 + 1))
}

/// A number that differs from call to call and thread to thread: enough
/// to spread waits, not for anything secret.
fn random() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    h.finish()
}
