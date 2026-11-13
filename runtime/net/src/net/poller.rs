//! The I/O event loop (decision 0046): the system's readiness queue
//! (epoll on Linux, kqueue on macOS, IOCP on Windows, through `mio`)
//! wakes the tasks waiting for a socket.
//!
//! There is no thread of its own. A thread with nothing else to do waits
//! on the queue: the scheduler's, when no task is ready but some wait for
//! a socket (`tasks::set_io_idle`), or a program's only thread while it
//! waits. It first looks at the queue for a moment (`LOOK`), while that
//! finds something. Its events wake the waiting tasks directly, so a
//! wake-up costs one switch between threads, as a blocking read does in C.
//!
//! With tasks on all cores (decision 0049), each worker has a queue of
//! its own, for the sockets its tasks made, and waits on it when it has
//! nothing to run. A socket's number says whose queue it is on.
//!
//! Each registered socket counts the events it has had. A task notes the
//! count before it tries an operation; when the operation would block,
//! it waits only if no event came since (`wait` gives `false` otherwise,
//! and the task tries again at once). So an event that arrives between
//! the try and the wait is never lost. Readiness is edge-triggered: an
//! event comes when a socket changes, which is why a task waits only
//! after an operation said it would block.

use mio::event::Source;
use mio::{Events, Interest, Poll, Registry, Token};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// What wakes a waiting task: called once, by the thread that sees its
/// event.
pub type Waker = Box<dyn FnOnce() + Send>;

/// Something a task can wait for: a socket on the queue, or a connect
/// finishing on its helper thread (`connect.rs`).
pub trait Ready {
    /// How many events it has had: noted before each try.
    fn events(&self) -> u64;
    /// Calls `wake` at the next event after the count `seen`; `false`
    /// when one came already: then the caller tries again now.
    fn on_event(&self, seen: u64, wake: Waker) -> bool;
    fn waits(&self) -> Waits;
}

/// How a waiting task learns of the event.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Waits {
    /// It can't: the caller sleeps a little and tries again (QUIC).
    No,
    /// From the readiness queue, which a thread waiting alone polls.
    OnQueue,
    /// From another thread, which wakes it.
    OnThread,
}

/// How long a thread with nothing to run keeps looking at its queue
/// before it sleeps on it. Measured on 100 clients and on a mostly idle
/// server (`docs/architecture/tasks-on-all-cores.md`): longer gains little
/// and costs an idle server's CPU.
const LOOK: Duration = Duration::from_micros(100);

/// A look that keeps finding nothing costs CPU and saves nothing: replies
/// come later than it lasts. After this many in a row, the worker sleeps
/// at once, and looks again only at every `LOOK_AGAIN`th sleep.
const LOOK_MISSES: u32 = 8;
const LOOK_AGAIN: u32 = 16;

/// Whether looking has found anything lately.
#[derive(Default)]
struct Looks {
    misses: u32,
    sleeps: u32,
}

impl Looks {
    /// This sleep starts with a look.
    fn now(&mut self) -> bool {
        self.sleeps = self.sleeps.wrapping_add(1);
        self.misses < LOOK_MISSES || self.sleeps.is_multiple_of(LOOK_AGAIN)
    }

    fn found(&mut self, found: bool) {
        self.misses = if found {
            0
        } else {
            self.misses.saturating_add(1)
        };
    }
}

/// The token of `Poller::interrupt`: sockets count from 1.
const INTERRUPT: Token = Token(0);

struct Poller {
    queue: Mutex<(Poll, Events, Looks)>,
    /// Ends a wait on the queue early: for an event from another thread.
    interrupt: mio::Waker,
    registry: Registry,
    next: AtomicUsize,
    sockets: Mutex<HashMap<usize, Entry>>,
}

#[derive(Default)]
struct Entry {
    /// How many events the socket has had.
    events: u64,
    waiter: Option<Waker>,
}

/// Each worker's poller, made at the first use of any; `None` when the
/// system gave no readiness queue (then callers try again after a short
/// sleep).
static POLLERS: OnceLock<Option<Vec<Poller>>> = OnceLock::new();

/// The low bits of a socket's number: the worker whose queue it is on.
const WORKER_BITS: usize = 8;
const WORKER_MASK: usize = (1 << WORKER_BITS) - 1;

/// The pollers, made at their first use.
fn pollers() -> Option<&'static [Poller]> {
    let made = POLLERS.get_or_init(|| {
        let workers = kavach_runtime_core::tasks::worker_count();
        let all = (0..workers).map(|_| make()).collect::<Option<Vec<_>>>()?;
        kavach_runtime_core::tasks::set_io_idle(idle_on, interrupt_on);
        Some(all)
    });
    made.as_deref()
}

fn make() -> Option<Poller> {
    let poll = Poll::new().ok()?;
    let registry = poll.registry().try_clone().ok()?;
    let interrupt = mio::Waker::new(poll.registry(), INTERRUPT).ok()?;
    Some(Poller {
        queue: Mutex::new((poll, Events::with_capacity(256), Looks::default())),
        interrupt,
        registry,
        next: AtomicUsize::new(1),
        sockets: Mutex::new(HashMap::new()),
    })
}

/// The poller of this thread's worker.
fn poller() -> Option<&'static Poller> {
    pollers()?.get(kavach_runtime_core::tasks::this_worker())
}

/// The poller of worker `w`, once they are made.
fn poller_on(w: usize) -> Option<&'static Poller> {
    POLLERS.get()?.as_deref()?.get(w)
}

/// The poller socket `n` is on.
fn poller_of(n: usize) -> Option<&'static Poller> {
    poller_on(n & WORKER_MASK)
}

/// A new socket number on the queue of worker `worker`, poller `p`.
fn number(p: &Poller, worker: usize) -> usize {
    p.next.fetch_add(1, Ordering::Relaxed) << WORKER_BITS | worker
}

fn sockets(p: &Poller) -> std::sync::MutexGuard<'_, HashMap<usize, Entry>> {
    p.sockets.lock().unwrap_or_else(|e| e.into_inner())
}

/// Waits up to `timeout` (`None`: as long as it takes) for sockets of
/// this thread's worker to be ready, counts their events and wakes their
/// tasks.
pub fn idle(timeout: Option<Duration>) {
    idle_on(kavach_runtime_core::tasks::this_worker(), timeout);
}

/// `idle` on worker `w`'s queue.
fn idle_on(w: usize, timeout: Option<Duration>) {
    let Some(p) = poller_on(w) else {
        return;
    };
    // Wakers run after the locks are let go: waking takes the
    // scheduler's lock, and a task registering holds that first.
    let woken: Vec<Waker> = {
        let mut queue = p.queue.lock().unwrap_or_else(|e| e.into_inner());
        let (poll, events, looks) = &mut *queue;
        // A reply often comes within the look, and going to sleep and
        // waking cost more than the look does.
        let look = match timeout {
            Some(t) if t.is_zero() => Duration::ZERO,
            _ if looks.now() => timeout.map_or(LOOK, |t| t.min(LOOK)),
            _ => Duration::ZERO,
        };
        let mut sleep = true;
        if !look.is_zero() {
            let until = Instant::now() + look;
            loop {
                if poll.poll(events, Some(Duration::ZERO)).is_err() {
                    return;
                }
                sleep = events.is_empty();
                if !sleep || Instant::now() >= until {
                    break;
                }
                std::hint::spin_loop();
            }
            looks.found(!sleep);
        }
        let left = timeout.map(|t| t.saturating_sub(look));
        if sleep && poll.poll(events, left).is_err() {
            return;
        }
        let mut all = sockets(p);
        events
            .iter()
            .filter_map(|e| {
                let entry = all.get_mut(&e.token().0)?;
                entry.events += 1;
                entry.waiter.take()
            })
            .collect()
    };
    for wake in woken {
        wake();
    }
}

/// Registers `source` for reading and writing; gives its number, or
/// `None` without a poller.
pub fn register(source: &mut impl Source) -> Option<usize> {
    let p = poller()?;
    let n = number(p, kavach_runtime_core::tasks::this_worker());
    sockets(p).insert(n, Entry::default());
    let interest = Interest::READABLE | Interest::WRITABLE;
    if p.registry.register(source, Token(n), interest).is_err() {
        sockets(p).remove(&n);
        return None;
    }
    Some(n)
}

/// Registers `source` under socket `n`'s number, so its events count as
/// `n`'s: a TLS listener's handshakes in progress wake its `accept`.
pub fn register_as(source: &mut impl Source, n: usize) -> bool {
    let interest = Interest::READABLE | Interest::WRITABLE;
    poller_of(n).is_some_and(|p| p.registry.register(source, Token(n), interest).is_ok())
}

/// Gives `source`, registered under socket `under`'s number, a number of
/// its own, on the same queue.
pub fn move_to_own(source: &mut impl Source, under: usize) -> Option<usize> {
    let p = poller_of(under)?;
    let n = number(p, under & WORKER_MASK);
    sockets(p).insert(n, Entry::default());
    let interest = Interest::READABLE | Interest::WRITABLE;
    if p.registry.reregister(source, Token(n), interest).is_err() {
        sockets(p).remove(&n);
        return None;
    }
    Some(n)
}

/// Whether socket `n` is on the queue of this thread's worker.
pub fn is_here(n: usize) -> bool {
    n & WORKER_MASK == kavach_runtime_core::tasks::this_worker()
}

/// Moves `source`, socket `n`, to the queue of this thread's worker;
/// gives its new number, or `None` when it couldn't move (then it stays).
pub fn move_here(source: &mut impl Source, n: usize) -> Option<usize> {
    let (from, to) = (poller_of(n)?, poller()?);
    from.registry.deregister(source).ok()?;
    sockets(from).remove(&n);
    let m = number(to, kavach_runtime_core::tasks::this_worker());
    sockets(to).insert(m, Entry::default());
    let interest = Interest::READABLE | Interest::WRITABLE;
    if to.registry.register(source, Token(m), interest).is_err() {
        sockets(to).remove(&m);
        return None;
    }
    Some(m)
}

/// How many events socket `n` has had: noted before each try.
pub fn events(n: usize) -> u64 {
    poller_of(n)
        .and_then(|p| sockets(p).get(&n).map(|e| e.events))
        .unwrap_or(0)
}

/// Calls `wake` at socket `n`'s next event, unless one came after the
/// count `seen`: then `false`, and the caller tries again now.
pub fn wait(n: usize, seen: u64, wake: Waker) -> bool {
    let Some(p) = poller_of(n) else { return false };
    let mut all = sockets(p);
    match all.get_mut(&n) {
        Some(entry) if entry.events == seen => {
            entry.waiter = Some(wake);
            true
        }
        _ => false,
    }
}

/// Ends a wait on the queue now, so its thread looks again: an event
/// came from another thread. With several workers, the scheduler ends
/// the wait of the worker it wakes (`interrupt_on`).
pub fn interrupt() {
    if !kavach_runtime_core::tasks::many_workers() {
        interrupt_on(0);
    }
}

/// Ends a wait on worker `w`'s queue now.
fn interrupt_on(w: usize) {
    if let Some(p) = poller_on(w) {
        let _ = p.interrupt.wake();
    }
}

/// Without tasks: the program's only thread waits for `r`'s next event
/// after `seen`, or until `at`: on the queue itself, or asleep until the
/// other thread wakes it.
pub fn wait_alone(r: &dyn Ready, seen: u64, at: Option<Instant>) {
    let ready = Arc::new(AtomicBool::new(false));
    let (flag, thread) = (Arc::clone(&ready), std::thread::current());
    let wake = Box::new(move || {
        flag.store(true, Ordering::Release);
        thread.unpark();
    });
    if !r.on_event(seen, wake) {
        return;
    }
    while !ready.load(Ordering::Acquire) {
        let left = at.map(|at| at.saturating_duration_since(Instant::now()));
        match (left, r.waits()) {
            (Some(Duration::ZERO), _) => return,
            (_, Waits::OnQueue) => idle(left),
            (Some(left), _) => std::thread::park_timeout(left),
            (None, _) => std::thread::park(),
        }
    }
}

/// Takes `source`, socket `n`, off its queue: it is about to change
/// (a plain connection that starts TLS) and will be registered again.
pub fn deregister(source: &mut impl Source, n: usize) {
    if let Some(p) = poller_of(n) {
        let _ = p.registry.deregister(source);
        sockets(p).remove(&n);
    }
}

/// Socket `n` is closed: it has no more events.
pub fn forget(n: usize) {
    if let Some(p) = poller_of(n) {
        sockets(p).remove(&n);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_stop_when_they_find_nothing_and_start_again_when_they_do() {
        let mut looks = Looks::default();
        for _ in 0..LOOK_MISSES {
            assert!(looks.now());
            looks.found(false);
        }
        // Given up: only every `LOOK_AGAIN`th sleep still looks.
        let looked = (0..LOOK_AGAIN).filter(|_| looks.now()).count();
        assert_eq!(looked, 1);
        looks.found(true);
        assert!((0..LOOK_AGAIN).all(|_| looks.now()));
    }
}
