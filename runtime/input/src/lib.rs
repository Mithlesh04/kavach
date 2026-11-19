// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Gamepads for `std.input` (spec 14.26, decision 0124), through SDL3 on
//! a thread of the runtime's own. It keeps each connected pad's state,
//! gyroscope included, which `pads` copies, and gives each change to every
//! window that listens to input, as an event
//! (`kavach_runtime_window::deliver_input`). Buttons and axes are named
//! by their place in the standard layout.
//!
//! A machine where SDL can't start has no pads: that is not an error.

use kavach_runtime_window::{self as win, Event};
use sdl3::gamepad::{Axis, Button, Gamepad};
use sdl3::sensor::SensorType;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, MutexGuard, OnceLock, mpsc};
use std::time::Duration;

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
    /// How fast it turns about x, y and z, in radians a second; `None`
    /// for a pad with no gyroscope.
    pub gyro: Option<(f64, f64, f64)>,
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

/// A button's place in `BUTTONS`; the triggers are axes in SDL.
pub(crate) fn button(b: Button) -> Option<usize> {
    Some(match b {
        Button::South => 0,
        Button::East => 1,
        Button::North => 2,
        Button::West => 3,
        Button::LeftShoulder => 4,
        Button::RightShoulder => 5,
        Button::Back => 8,
        Button::Start => 9,
        Button::Guide => 10,
        Button::LeftStick => 11,
        Button::RightStick => 12,
        Button::DPadUp => 13,
        Button::DPadDown => 14,
        Button::DPadLeft => 15,
        Button::DPadRight => 16,
        _ => return None,
    })
}

/// An axis by name, and where it is: sticks from -1 to 1 with y up, as
/// spec 14.26 has them (SDL's y is down), triggers from 0 to 1.
pub(crate) fn axis(a: Axis, raw: i16) -> Change {
    let v = (f64::from(raw) / 32_767.0).clamp(-1.0, 1.0);
    match a {
        Axis::LeftX => Change::Axis("LeftX", v),
        Axis::LeftY => Change::Axis("LeftY", -v),
        Axis::RightX => Change::Axis("RightX", v),
        Axis::RightY => Change::Axis("RightY", -v),
        Axis::TriggerLeft => Change::Axis("LeftTrigger", v.max(0.0)),
        Axis::TriggerRight => Change::Axis("RightTrigger", v.max(0.0)),
    }
}

/// How far a trigger must be pressed to count as held.
const HELD: f64 = 0.5;

/// The button a trigger's axis makes, when moving from `before` to `now`
/// crosses `HELD`: a trigger is a button and an axis (spec 14.26).
pub(crate) fn trigger_button(name: &str, before: f64, now: f64) -> Option<Change> {
    let k = match name {
        "LeftTrigger" => 6,
        "RightTrigger" => 7,
        _ => return None,
    };
    ((before >= HELD) != (now >= HELD)).then_some(Change::Button(k, now >= HELD))
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

/// The gamepad thread: SDL's events into the shared state and the
/// windows, each pad's gyroscope read on each round, and rumbles asked
/// for.
fn watch(rumbles: mpsc::Receiver<Rumble>, ready: mpsc::Sender<()>) {
    let started = sdl3::init().and_then(|sdl| Ok((sdl.gamepad()?, sdl.event_pump()?)));
    let Ok((subsystem, mut pump)) = started else {
        let _ = ready.send(());
        return;
    };
    let mut open: HashMap<u32, Gamepad> = HashMap::new();
    for id in subsystem.gamepads().unwrap_or_default() {
        connect(&subsystem, &mut open, id);
    }
    let _ = ready.send(());
    loop {
        while let Ok(r) = rumbles.try_recv() {
            if let Some(pad) = u32::try_from(r.id).ok().and_then(|id| open.get_mut(&id)) {
                let level = |f: f64| (f.clamp(0.0, 1.0) * f64::from(u16::MAX)) as u16;
                let _ = pad.set_rumble(level(r.strong), level(r.weak), r.ms);
            }
        }
        let first = pump.wait_event_timeout(Duration::from_millis(10));
        for event in first
            .into_iter()
            .chain(pump.poll_iter().collect::<Vec<_>>())
        {
            handle(&subsystem, &mut open, event);
        }
        read_gyros(&open);
    }
}

/// Opens pad `id`, turns its gyroscope on if it has one, and tells of it.
fn connect(
    subsystem: &sdl3::GamepadSubsystem,
    open: &mut HashMap<u32, Gamepad>,
    id: sdl3::joystick::JoystickId,
) {
    let Ok(pad) = subsystem.open(id) else {
        return;
    };
    let _ = pad.sensor_set_enabled(SensorType::Gyroscope, true);
    let name = pad.name().unwrap_or_else(|| "Gamepad".into());
    let n = pad.id().map_or(0, |i| i.raw());
    open.insert(n, pad);
    let e = apply(
        &mut lock(&shared().pads),
        i64::from(n),
        Change::Connected(name),
    );
    win::deliver_input(e);
}

/// One SDL event: a pad come or gone, a button, an axis.
fn handle(
    subsystem: &sdl3::GamepadSubsystem,
    open: &mut HashMap<u32, Gamepad>,
    event: sdl3::event::Event,
) {
    use sdl3::event::Event as E;
    let (id, change) = match event {
        E::GamepadAdded { which, .. } => {
            if !open.contains_key(&which.raw()) {
                connect(subsystem, open, which);
            }
            return;
        }
        E::GamepadRemoved { which, .. } => {
            open.remove(&which.raw());
            (which.raw(), Change::Gone)
        }
        E::GamepadButtonDown {
            which, button: b, ..
        } => match button(b) {
            Some(k) => (which.raw(), Change::Button(k, true)),
            None => return,
        },
        E::GamepadButtonUp {
            which, button: b, ..
        } => match button(b) {
            Some(k) => (which.raw(), Change::Button(k, false)),
            None => return,
        },
        E::GamepadAxisMotion {
            which,
            axis: a,
            value,
            ..
        } => (which.raw(), axis(a, value)),
        _ => return,
    };
    let id = i64::from(id);
    let mut pads = lock(&shared().pads);
    if let Change::Axis(name, now) = change {
        let before = pads.get(&id).map_or(0.0, |p| match name {
            "LeftTrigger" => p.triggers.0,
            "RightTrigger" => p.triggers.1,
            _ => 0.0,
        });
        if let Some(held) = trigger_button(name, before, now) {
            win::deliver_input(apply(&mut pads, id, held));
        }
    }
    let e = apply(&mut pads, id, change);
    drop(pads);
    win::deliver_input(e);
}

/// Each open pad's gyroscope now, into its state.
fn read_gyros(open: &HashMap<u32, Gamepad>) {
    let mut pads = lock(&shared().pads);
    for (id, pad) in open {
        let mut data = [0.0f32; 3];
        let gyro = (pad.sensor_enabled(SensorType::Gyroscope)
            && pad
                .sensor_get_data(SensorType::Gyroscope, &mut data)
                .is_ok())
        .then(|| (f64::from(data[0]), f64::from(data[1]), f64::from(data[2])));
        if let Some(p) = pads.get_mut(&i64::from(*id)) {
            p.gyro = gyro;
        }
    }
}

/// The connected pads now, by number.
pub fn pads() -> Vec<Pad> {
    start();
    lock(&shared().pads).values().cloned().collect()
}

/// `pads`, as `std.input` reads it: twelve numbers a pad (its number,
/// the buttons' bits, the sticks' x and y, the triggers, whether it has a
/// gyroscope and its three rates), with the names kept for `names`.
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
                f64::from(u8::from(p.gyro.is_some())),
                p.gyro.map_or(0.0, |g| g.0),
                p.gyro.map_or(0.0, |g| g.1),
                p.gyro.map_or(0.0, |g| g.2),
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
