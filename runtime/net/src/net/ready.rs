//! A socket as something a task waits for (`poller::Ready`): its events
//! on its worker's queue, counted for what a wait wants, reading or
//! writing, so a read isn't woken over and over because the socket can
//! be written to.

use super::{Shared, Socket, poller};

impl poller::Ready for Shared {
    fn events(&self) -> u64 {
        Shared::events(self)
    }

    fn on_event(&self, seen: u64, wake: poller::Waker) -> bool {
        // A socket that moved has a new count: `seen` is the old one's, so
        // the caller tries again and notes the new count first.
        self.token()
            .is_some_and(|n| poller::wait(self.here(n), seen, wake))
    }

    fn waits(&self) -> poller::Waits {
        match self.token() {
            Some(_) => poller::Waits::OnQueue,
            None => poller::Waits::No,
        }
    }

    fn events_for(&self, want: poller::Want) -> u64 {
        self.token().map_or(0, |n| poller::events_for(n, want))
    }

    fn on_event_for(&self, seen: u64, want: poller::Want, wake: poller::Waker) -> bool {
        self.token()
            .is_some_and(|n| poller::wait_for(self.here(n), seen, want, wake))
    }
}

impl Shared {
    /// What a read (or an accept) on it waits for: a plain socket only
    /// for something to read; TLS for either, as reading may need to
    /// write.
    pub fn wants_read(&self) -> poller::Want {
        self.with(|s| match s {
            Socket::Stream(_) | Socket::Listener(_) | Socket::Udp(_) => poller::Want::Read,
            _ => poller::Want::Any,
        })
    }

    /// What a write on it waits for: a plain socket only for room.
    pub fn wants_write(&self) -> poller::Want {
        self.with(|s| match s {
            Socket::Stream(_) | Socket::Udp(_) => poller::Want::Write,
            _ => poller::Want::Any,
        })
    }
}
