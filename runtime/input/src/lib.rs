//! Gamepads for `std.input` (spec 14.26, decision 0113), through `gilrs`
//! on a thread of the runtime's own. It keeps each connected pad's state,
//! which `pads` copies, and gives each change to every window that
//! listens to input, as an event (`kavach_runtime_window::deliver_input`).
//! Buttons and axes are named by their place in the standard layout.
//!
//! A machine where `gilrs` can't start (no udev, as in a container) has
//! no pads: that is not an error.

use gilrs::{Axis, Button, EventType, Gilrs};
use kavach_runtime_window::{self as win, Event};
use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, OnceLock, mpsc};
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

/// The buttons of the standard layout, by place; button `k` of a pad is
/// bit `k` of `Pad::buttons`.
pub const BUTTONS: [&str; 17] = [
    "South",
    "East",
    "North",
    "West",
    "LeftBumper",
    "RightBumper",
    "LeftTrigger",
    "RightTrigger",
    "Select",
    "Start",
    "Mode",
    "LeftStick",
    "RightStick",
    "DPadUp",
    "DPadDown",
    "DPadLeft",
    "DPadRight",
];

/// A connected pad now: its number and name, the buttons held (bits, as
/// `BUTTONS`), its sticks (x right, y up, each -1 to 1) and its triggers
/// (0 to 1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pad {
    pub id: i64,
    pub name: String,
    pub buttons: u32,
    pub left: (f64, f64),
    pub right: (f64, f64),
    pub triggers: (f64, f64),
}

/// What happened to a pad, in the standard layout's terms.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Change {
    Connected(String),
    Gone,
    /// Button `BUTTONS[k]` down or up.
    Button(usize, bool),
    /// An axis by name (`LeftX`, `LeftY`, `RightX`, `RightY`,
    /// `LeftTrigger`, `RightTrigger`) and where it is now.
    Axis(&'static str, f64),
}

/// A button's place in `BUTTONS`.
pub(crate) fn button(b: Button) -> Option<usize> {
    Some(match b {
        Button::South => 0,
        Button::East => 1,
        Button::North => 2,
        Button::West => 3,
        Button::LeftTrigger => 4,
        Button::RightTrigger => 5,
        Button::LeftTrigger2 => 6,
        Button::RightTrigger2 => 7,
        Button::Select => 8,
        Button::Start => 9,
        Button::Mode => 10,
        Button::LeftThumb => 11,
        Button::RightThumb => 12,
        Button::DPadUp => 13,
        Button::DPadDown => 14,
        Button::DPadLeft => 15,
        Button::DPadRight => 16,
        _ => return None,
    })
}

/// What a `gilrs` event says, in the standard layout; `None` for what
/// isn't in it (another axis, an unknown button).
pub(crate) fn change_of(event: &EventType, name: &str) -> Option<Change> {
    match *event {
        EventType::Connected => Some(Change::Connected(name.to_string())),
        EventType::Disconnected => Some(Change::Gone),
        EventType::ButtonPressed(b, _) => Some(Change::Button(button(b)?, true)),
        EventType::ButtonReleased(b, _) => Some(Change::Button(button(b)?, false)),
        EventType::ButtonChanged(b, v, _) => trigger(b, v),
        EventType::AxisChanged(a, v, _) => stick(a, v),
        _ => None,
    }
}

/// How far a trigger is pressed: a trigger is a button and an axis.
pub(crate) fn trigger(b: Button, v: f32) -> Option<Change> {
    match b {
        Button::LeftTrigger2 => Some(Change::Axis("LeftTrigger", v.into())),
        Button::RightTrigger2 => Some(Change::Axis("RightTrigger", v.into())),
        _ => None,
    }
}

/// Where a stick is, on one of its axes.
pub(crate) fn stick(a: Axis, v: f32) -> Option<Change> {
    let name = match a {
        Axis::LeftStickX => "LeftX",
        Axis::LeftStickY => "LeftY",
        Axis::RightStickX => "RightX",
        Axis::RightStickY => "RightY",
        _ => return None,
    };
    Some(Change::Axis(name, v.into()))
}

/// Applies `change` of pad `id` to `pads`, and gives the event it makes.
pub(crate) fn apply(pads: &mut BTreeMap<i64, Pad>, id: i64, change: Change) -> Event {
    let event = |kind, b, text: &str| Event {
        kind,
        a: id as f64,
        b,
        text: text.to_string(),
    };
    match change {
        Change::Connected(name) => {
            let e = event(win::PAD_CONNECTED, 0.0, &name);
            pads.insert(
                id,
                Pad {
                    id,
                    name,
                    ..Pad::default()
                },
            );
            e
        }
        Change::Gone => {
            pads.remove(&id);
            event(win::PAD_GONE, 0.0, "")
        }
        Change::Button(k, down) => {
            let pad = pads.entry(id).or_insert_with(|| Pad {
                id,
                ..Pad::default()
            });
            if down {
                pad.buttons |= 1 << k;
            } else {
                pad.buttons &= !(1 << k);
            }
            event(win::PAD_BUTTON, f64::from(u8::from(down)), BUTTONS[k])
        }
        Change::Axis(name, v) => {
            let pad = pads.entry(id).or_insert_with(|| Pad {
                id,
                ..Pad::default()
            });
            let v = v.clamp(-1.0, 1.0);
            match name {
                "LeftX" => pad.left.0 = v,
                "LeftY" => pad.left.1 = v,
                "RightX" => pad.right.0 = v,
                "RightY" => pad.right.1 = v,
                "LeftTrigger" => pad.triggers.0 = v.max(0.0),
                _ => pad.triggers.1 = v.max(0.0),
            }
            event(win::PAD_AXIS, v, name)
        }
    }
}

/// What the program asks of the gamepad thread: rumble pad `id`.
struct Rumble {
    id: i64,
    strong: f64,
    weak: f64,
    ms: u32,
}

struct Shared {
    pads: Mutex<BTreeMap<i64, Pad>>,
    rumble: Mutex<Option<mpsc::Sender<Rumble>>>,
    /// The names of the pads of the last `pad_numbers`.
    names: Mutex<Vec<String>>,
}

fn shared() -> &'static Shared {
    static SHARED: OnceLock<Shared> = OnceLock::new();
    SHARED.get_or_init(|| Shared {
        pads: Mutex::new(BTreeMap::new()),
        rumble: Mutex::new(None),
        names: Mutex::new(Vec::new()),
    })
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Starts watching gamepads, once: from `input.pads()`, and when a window
/// listens to input, so pad events come to it.
pub fn start() {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        *lock(&shared().rumble) = Some(tx);
        // The first pads are known before `start` gives them.
        let (ready, wait) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("kavach-gamepads".into())
            .spawn(move || watch(rx, ready));
        if spawned.is_ok() {
            let _ = wait.recv_timeout(Duration::from_secs(2));
        }
    });
}

/// The gamepad thread: `gilrs`'s events into the shared state and the
/// windows, and rumbles asked for.
fn watch(rumbles: mpsc::Receiver<Rumble>, ready: mpsc::Sender<()>) {
    let mut gilrs = match Gilrs::new() {
        Ok(g) => g,
        Err(gilrs::Error::NotImplemented(g)) => g,
        Err(_) => {
            let _ = ready.send(());
            return;
        }
    };
    let first: Vec<(i64, String)> = (gilrs.gamepads())
        .map(|(id, g)| (usize::from(id) as i64, g.name().to_string()))
        .collect();
    for (id, name) in first {
        let e = apply(&mut lock(&shared().pads), id, Change::Connected(name));
        win::deliver_input(e);
    }
    let _ = ready.send(());
    let mut playing: Vec<(gilrs::ff::Effect, Instant)> = Vec::new();
    loop {
        while let Ok(r) = rumbles.try_recv() {
            if let Some(effect) = rumble(&mut gilrs, &r) {
                let end = Instant::now() + Duration::from_millis(u64::from(r.ms));
                playing.push((effect, end));
            }
        }
        playing.retain(|(_, end)| *end > Instant::now());
        let Some(ev) = gilrs.next_event_blocking(Some(Duration::from_millis(20))) else {
            continue;
        };
        let name = gilrs.gamepad(ev.id).name().to_string();
        if let Some(change) = change_of(&ev.event, &name) {
            let id = usize::from(ev.id) as i64;
            let e = apply(&mut lock(&shared().pads), id, change);
            win::deliver_input(e);
        }
    }
}

/// Plays rumble `r` on its pad, if it is connected and can rumble.
fn rumble(gilrs: &mut Gilrs, r: &Rumble) -> Option<gilrs::ff::Effect> {
    use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Replay, Ticks};
    let (id, pad) = gilrs
        .gamepads()
        .find(|(id, _)| usize::from(*id) as i64 == r.id)?;
    if !pad.is_ff_supported() {
        return None;
    }
    let scheduling = Replay {
        play_for: Ticks::from_ms(r.ms),
        ..Replay::default()
    };
    let magnitude = |f: f64| (f.clamp(0.0, 1.0) * f64::from(u16::MAX)) as u16;
    let effect = EffectBuilder::new()
        .add_effect(BaseEffect {
            kind: BaseEffectType::Strong {
                magnitude: magnitude(r.strong),
            },
            scheduling,
            ..BaseEffect::default()
        })
        .add_effect(BaseEffect {
            kind: BaseEffectType::Weak {
                magnitude: magnitude(r.weak),
            },
            scheduling,
            ..BaseEffect::default()
        })
        .gamepads(&[id])
        .finish(gilrs)
        .ok()?;
    effect.play().ok()?;
    Some(effect)
}

/// The connected pads now, by number.
pub fn pads() -> Vec<Pad> {
    start();
    lock(&shared().pads).values().cloned().collect()
}

/// `pads`, as `std.input` reads it: eight numbers a pad (its number, the
/// buttons' bits, the sticks' x and y, the triggers), with the names kept
/// for `names`.
pub fn pad_numbers() -> Vec<f64> {
    let all = pads();
    *lock(&shared().names) = all.iter().map(|p| p.name.clone()).collect();
    (all.iter())
        .flat_map(|p| {
            [
                p.id as f64,
                f64::from(p.buttons),
                p.left.0,
                p.left.1,
                p.right.0,
                p.right.1,
                p.triggers.0,
                p.triggers.1,
            ]
        })
        .collect()
}

/// The names of the pads of the last `pad_numbers`, in its order.
pub fn names() -> Vec<String> {
    std::mem::take(&mut *lock(&shared().names))
}

/// Makes pad `id` vibrate for `ms` milliseconds: its strong (low) motor
/// at `strong` and its weak (high) one at `weak`, each 0 to 1. Nothing
/// happens for a pad that is gone or can't rumble.
pub fn rumble_pad(id: i64, strong: f64, weak: f64, ms: i64) {
    start();
    let ms = u32::try_from(ms.clamp(0, 60_000)).unwrap_or(0);
    if let Some(tx) = lock(&shared().rumble).as_ref() {
        let _ = tx.send(Rumble {
            id,
            strong,
            weak,
            ms,
        });
    }
}
