// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Each window's inbox: the events that came to it, waiting for the
//! program to take them, and who to wake when one comes (spec 14.22).
//! A task waiting in `events()` lets other tasks run meanwhile (spec
//! 9.7): it leaves a waker here, which the next event calls.

use crate::events::{Event, Press, REDRAW, WOKEN, plain};
use std::collections::{HashMap, VecDeque};
use std::sync::{Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

/// What wakes a task that waits for a window's events, from any thread.
pub type Waker = Box<dyn FnOnce() + Send>;

/// What a window has that its program reads.
#[derive(Default)]
pub(crate) struct Inbox {
    pub(crate) events: VecDeque<Event>,
    /// Whether keyboard, mouse and text events come (`listen_input`).
    pub(crate) input: bool,
    pub(crate) size: (u32, u32),
    pub(crate) scale: f64,
    /// The modifiers held now, as `events::SHIFT` and the rest.
    pub(crate) modifiers: u8,
    /// Where the pointer is, and the last press, to count clicks.
    pub(crate) pointer: (f64, f64),
    pub(crate) press: Press,
    /// Tasks waiting for the next event.
    pub(crate) wakers: Vec<Waker>,
    /// How many frames were shown in the window (spec 14.24).
    pub(crate) shown: u64,
}

impl Inbox {
    /// Adds `e`; a `Redraw` or `Woken` already waiting to be taken
    /// stands for another (spec 14.22).
    pub(crate) fn push(&mut self, e: Event) {
        let once = matches!(e.kind, REDRAW | WOKEN);
        if !(once && self.events.iter().any(|old| old.kind == e.kind)) {
            self.events.push_back(e);
        }
    }

    /// The wakers left, to be called once the lock is let go.
    fn woken(&mut self) -> Vec<Waker> {
        std::mem::take(&mut self.wakers)
    }
}

/// Every window's inbox, and a signal when one gets an event or a frame.
pub(crate) struct Hub {
    inboxes: Mutex<HashMap<i64, Inbox>>,
    ready: Condvar,
}

pub(crate) fn hub() -> &'static Hub {
    static HUB: OnceLock<Hub> = OnceLock::new();
    HUB.get_or_init(|| Hub {
        inboxes: Mutex::new(HashMap::new()),
        ready: Condvar::new(),
    })
}

impl Hub {
    pub(crate) fn lock(&self) -> MutexGuard<'_, HashMap<i64, Inbox>> {
        self.inboxes.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Changes window `n`'s inbox with `f`, then wakes whoever waits for
    /// it when `f` gives `true`: nothing when the window is closed.
    pub(crate) fn change(&self, n: i64, f: impl FnOnce(&mut Inbox) -> bool) {
        let mut all = self.lock();
        let Some(inbox) = all.get_mut(&n) else { return };
        if !f(inbox) {
            return;
        }
        let wakers = inbox.woken();
        drop(all);
        self.wake(wakers);
    }

    /// Wakes the threads and tasks waiting.
    pub(crate) fn wake(&self, wakers: Vec<Waker>) {
        self.ready.notify_all();
        for w in wakers {
            w();
        }
    }

    /// Adds window `n`'s inbox.
    pub(crate) fn add(&self, n: i64, inbox: Inbox) {
        self.lock().insert(n, inbox);
    }

    /// Takes window `n`'s inbox away, waking whoever waits for it.
    pub(crate) fn remove(&self, n: i64) -> bool {
        let gone = self.lock().remove(&n);
        let found = gone.is_some();
        self.wake(gone.map(|mut i| i.woken()).unwrap_or_default());
        found
    }
}

/// The events window `n` has had since the last `take`; with `wait`,
/// waiting for at least one first, holding this thread. None once the
/// window is closed.
pub fn take(n: i64, wait: bool) -> Vec<Event> {
    let mut all = hub().lock();
    loop {
        let Some(inbox) = all.get_mut(&n) else {
            return Vec::new();
        };
        if !inbox.events.is_empty() || !wait {
            return inbox.events.drain(..).collect();
        }
        all = (hub().ready)
            .wait_timeout(all, Duration::from_millis(250))
            .unwrap_or_else(|e| e.into_inner())
            .0;
    }
}

/// Calls `wake` at window `n`'s next event, or when it closes, so a task
/// waits for it while others run; `false` when there is no need to wait
/// (an event came already, or the window is closed): look again now.
pub fn on_event(n: i64, wake: Waker) -> bool {
    let mut all = hub().lock();
    match all.get_mut(&n) {
        Some(inbox) if inbox.events.is_empty() => {
            inbox.wakers.push(wake);
            true
        }
        _ => false,
    }
}

/// Whether `take` of window `n` gives something now, or nothing ever
/// again: it has events, or it is closed.
pub fn ready(n: i64) -> bool {
    hub().lock().get(&n).is_none_or(|i| !i.events.is_empty())
}

/// `take`, as `std.window` reads it: five numbers an event (its kind and
/// four numbers), with the texts kept for `texts`, one an event.
pub fn take_numbers(n: i64, wait: bool) -> Vec<f64> {
    let got = take(n, wait);
    let texts = got.iter().map(|e| e.text.clone()).collect();
    last_texts()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(n, texts);
    got.iter().flat_map(Event::numbers).collect()
}

/// The texts of window `n`'s last `take_numbers`: a key's name, or text
/// typed; empty for other events.
pub fn texts(n: i64) -> Vec<String> {
    let mut all = last_texts().lock().unwrap_or_else(|e| e.into_inner());
    all.remove(&n).unwrap_or_default()
}

fn last_texts() -> &'static Mutex<HashMap<i64, Vec<String>>> {
    static LAST: OnceLock<Mutex<HashMap<i64, Vec<String>>>> = OnceLock::new();
    LAST.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Gives `event`, which no window had (a gamepad's), to every window
/// that listens to input, waking any that waits.
pub fn deliver_input(event: Event) {
    let mut all = hub().lock();
    let mut wakers = Vec::new();
    for inbox in all.values_mut().filter(|i| i.input) {
        inbox.push(event.clone());
        wakers.extend(inbox.woken());
    }
    drop(all);
    hub().wake(wakers);
}

/// Makes window `n`'s waiting `events()` give a `Woken` event, from any
/// task or thread; nothing once it is closed (spec 14.22).
pub fn wake(n: i64) {
    hub().change(n, |inbox| {
        inbox.push(plain(WOKEN, 0.0, 0.0));
        true
    });
}

/// From now on, keyboard, mouse and text events come to window `n` too.
pub fn listen_input(n: i64) {
    hub().change(n, |inbox| {
        inbox.input = true;
        false
    });
}

/// The inside of window `n`, in pixels; `(0, 0)` once it is closed.
pub fn size(n: i64) -> (i64, i64) {
    let all = hub().lock();
    all.get(&n)
        .map_or((0, 0), |i| (i64::from(i.size.0), i64::from(i.size.1)))
}

/// How many pixels make a point on window `n`'s screen.
pub fn scale(n: i64) -> f64 {
    hub().lock().get(&n).map_or(1.0, |i| i.scale)
}

/// A frame was shown in window `n` (spec 14.24): a `Redraw` the event
/// loop waits on is answered.
pub fn frame_shown(n: i64) {
    let mut all = hub().lock();
    if let Some(inbox) = all.get_mut(&n) {
        inbox.shown += 1;
        drop(all);
        hub().ready.notify_all();
    }
}

/// How many frames window `n` has shown, or `None` once it is closed.
pub(crate) fn shown(n: i64) -> Option<u64> {
    hub().lock().get(&n).map(|i| i.shown)
}

/// Waits until window `n` shows a frame after its `seen`th, or until
/// `limit` passes, or it closes: whether a frame came. The event loop
/// waits so while the system runs its own loop of a resize (spec 14.22),
/// so the window is drawn at each new size.
pub(crate) fn wait_for_frame(n: i64, seen: u64, limit: Duration) -> bool {
    let end = Instant::now() + limit;
    let mut all = hub().lock();
    loop {
        match all.get(&n) {
            Some(i) if i.shown > seen => return true,
            None => return false,
            Some(_) => {}
        }
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return false;
        }
        all = (hub().ready)
            .wait_timeout(all, left)
            .unwrap_or_else(|e| e.into_inner())
            .0;
    }
}
