//! Events a thread of the runtime's own hears (a child program's lines,
//! its end), kept for the task that waits for them. The task notes the
//! count, looks for an event, and if none came asks to be woken at the
//! next one; the count closes the gap between looking and asking.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Wakes the task waiting for the next event, from the thread that heard it.
pub type Waker = Box<dyn FnOnce() + Send>;

struct Heard<E> {
    events: VecDeque<E>,
    /// How many events came: a task notes it before it looks.
    count: u64,
    waiter: Option<Waker>,
}

/// Events of type `E`, shared between the thread that hears them and the
/// task that takes them.
pub struct Inbox<E> {
    heard: Arc<Mutex<Heard<E>>>,
}

impl<E> Clone for Inbox<E> {
    fn clone(&self) -> Self {
        Inbox {
            heard: Arc::clone(&self.heard),
        }
    }
}

impl<E> Default for Inbox<E> {
    fn default() -> Self {
        Inbox {
            heard: Arc::new(Mutex::new(Heard {
                events: VecDeque::new(),
                count: 0,
                waiter: None,
            })),
        }
    }
}

impl<E> Inbox<E> {
    fn lock(&self) -> std::sync::MutexGuard<'_, Heard<E>> {
        self.heard.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Adds an event, waking the task waiting for one.
    pub fn tell(&self, event: E) {
        let waiter = {
            let mut h = self.lock();
            h.events.push_back(event);
            h.count += 1;
            h.waiter.take()
        };
        if let Some(wake) = waiter {
            wake();
        }
    }

    /// How many events came so far: noted before looking for one.
    pub fn count(&self) -> u64 {
        self.lock().count
    }

    /// The next event, if one came.
    pub fn next(&self) -> Option<E> {
        self.lock().events.pop_front()
    }

    /// Calls `wake` at the next event after the count `seen`; `false`
    /// when one came already (then the caller looks again now).
    pub fn on_event(&self, seen: u64, wake: Waker) -> bool {
        let mut h = self.lock();
        if h.count != seen {
            return false;
        }
        h.waiter = Some(wake);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn events_come_in_order_and_wake_the_waiter() {
        let inbox: Inbox<i32> = Inbox::default();
        let woken = Arc::new(AtomicBool::new(false));
        let seen = inbox.count();
        assert!(inbox.next().is_none());
        let flag = Arc::clone(&woken);
        assert!(inbox.on_event(seen, Box::new(move || flag.store(true, Ordering::SeqCst))));
        let teller = inbox.clone();
        std::thread::spawn(move || {
            teller.tell(1);
            teller.tell(2);
        })
        .join()
        .unwrap();
        assert!(woken.load(Ordering::SeqCst));
        assert_eq!(
            (inbox.next(), inbox.next(), inbox.next()),
            (Some(1), Some(2), None)
        );
        // An event came after the count was noted: look again at once.
        assert!(!inbox.on_event(seen, Box::new(|| {})));
    }
}
