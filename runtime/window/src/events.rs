// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What a window event means to a program (spec 14.22): `winit`'s events
//! turned into `std.window`'s, numbered as its `Event` variants are.

use crate::inbox::Inbox;
use std::time::{Duration, Instant};
use winit::event::{
    ElementState, Ime, Modifiers, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent,
};
use winit::keyboard::{Key, NamedKey, PhysicalKey};

/// The kinds of event, numbered as `std.window`'s `Event` variants are.
pub const CLOSED: i64 = 0;
pub const RESIZED: i64 = 1;
pub const SCALED: i64 = 2;
pub const FOCUSED: i64 = 3;
/// A key: down (`a`), repeated (`b`), the modifiers (`c`); its text is
/// the key's place and its name in the layout, a tab between.
pub const KEY: i64 = 4;
pub const TEXT: i64 = 5;
/// The pointer at `a`, `b`, with the modifiers (`c`).
pub const MOUSE_MOVED: i64 = 6;
/// A button (`a`) down (`b`), its clicks (`c`) and the modifiers (`d`).
pub const MOUSE_BUTTON: i64 = 7;
/// Scrolled by `a`, `b`, in pixels when `c` is 1, with the modifiers
/// (`d`); its text is the phase.
pub const WHEEL: i64 = 8;
pub const MOTION: i64 = 9;
/// Gamepads' events, from `kavach_runtime_input` (spec 14.26): a pad
/// connected (its number, its name), gone, a button, and a stick or
/// trigger moved.
pub const PAD_CONNECTED: i64 = 10;
pub const PAD_GONE: i64 = 11;
pub const PAD_BUTTON: i64 = 12;
pub const PAD_AXIS: i64 = 13;
/// A touch: where (`a`, `b`), and in its text its phase, its id and its
/// pressure, as `Began 3 0.5`.
pub const TOUCH: i64 = 14;
/// The system asks for the window to be drawn again.
pub const REDRAW: i64 = 15;
/// `wake()` was called.
pub const WOKEN: i64 = 16;
/// The pointer came into the window, or left it.
pub const MOUSE_ENTERED: i64 = 17;
pub const MOUSE_LEFT: i64 = 18;
/// Text being composed by an input method; its selected part from
/// character `a` to `b`, or -1 for both when it shows none.
pub const COMPOSING: i64 = 19;

/// The bits of the modifiers held, as `std.window` reads them.
pub const SHIFT: u8 = 1;
pub const CTRL: u8 = 2;
pub const ALT: u8 = 4;
pub const META: u8 = 8;

/// Presses this close in time and place make one more click: the
/// defaults of Windows, macOS and GNOME (spec 14.22).
const CLICK_TIME: Duration = Duration::from_millis(500);
const CLICK_PLACE: f64 = 4.0;

/// One event: its kind, four numbers, and the text of a key or of typing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Event {
    pub kind: i64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub text: String,
}

impl Event {
    /// The numbers `std.window` reads for it, in order.
    pub fn numbers(&self) -> [f64; 5] {
        [self.kind as f64, self.a, self.b, self.c, self.d]
    }
}

pub(crate) fn plain(kind: i64, a: f64, b: f64) -> Event {
    Event {
        kind,
        a,
        b,
        ..Event::default()
    }
}

/// An event with all four numbers and a text.
fn full(kind: i64, numbers: [f64; 4], text: String) -> Event {
    let [a, b, c, d] = numbers;
    Event {
        kind,
        a,
        b,
        c,
        d,
        text,
    }
}

/// The last press, to count clicks (spec 14.22).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Press {
    button: f64,
    at: Option<Instant>,
    place: (f64, f64),
    clicks: u32,
}

impl Press {
    /// How many clicks a press of `button` at `place`, at `now`, makes.
    pub(crate) fn count(&mut self, button: f64, place: (f64, f64), now: Instant) -> u32 {
        let near = (place.0 - self.place.0).abs() <= CLICK_PLACE
            && (place.1 - self.place.1).abs() <= CLICK_PLACE;
        let soon = self.at.is_some_and(|t| now.duration_since(t) <= CLICK_TIME);
        self.clicks = if near && soon && button == self.button {
            self.clicks + 1
        } else {
            1
        };
        (self.button, self.at, self.place) = (button, Some(now), place);
        self.clicks
    }

    /// The clicks of the last press, which its release gives too.
    pub(crate) fn clicks(&self) -> u32 {
        self.clicks.max(1)
    }
}

/// The modifiers' bits.
pub(crate) fn modifier_bits(m: &Modifiers) -> u8 {
    let s = m.state();
    let bit = |on: bool, b: u8| if on { b } else { 0 };
    bit(s.shift_key(), SHIFT)
        | bit(s.control_key(), CTRL)
        | bit(s.alt_key(), ALT)
        | bit(s.super_key(), META)
}

/// A key's name in the keyboard's layout, as the W3C's
/// `KeyboardEvent.key` gives it: `z`, `Z`, `Enter`, `ArrowLeft`.
pub(crate) fn key_name(key: &Key) -> String {
    match key {
        Key::Character(s) => s.to_string(),
        Key::Named(NamedKey::Space) => " ".into(),
        Key::Named(n) => format!("{n:?}"),
        Key::Dead(_) => "Dead".into(),
        Key::Unidentified(_) => "Unidentified".into(),
    }
}

/// A phase's name, as `Touch` and `Wheel` give it.
fn phase_name(p: TouchPhase) -> &'static str {
    match p {
        TouchPhase::Started => "Began",
        TouchPhase::Moved => "Moved",
        TouchPhase::Ended => "Ended",
        TouchPhase::Cancelled => "Cancelled",
    }
}

/// The place of byte `at` of `text`, in characters.
fn chars_to(text: &str, at: usize) -> f64 {
    text.get(..at).map_or(-1.0, |t| t.chars().count() as f64)
}

/// What a window event means to a program, keeping the inbox's size,
/// scale, modifiers and pointer up to date; input events only once it
/// listens.
pub(crate) fn translate(event: WindowEvent, inbox: &mut Inbox) -> Vec<Event> {
    let input = inbox.input;
    let flag = |on: bool| if on { 1.0 } else { 0.0 };
    let mods = f64::from(inbox.modifiers);
    match event {
        WindowEvent::CloseRequested => vec![plain(CLOSED, 0.0, 0.0)],
        WindowEvent::RedrawRequested => vec![plain(REDRAW, 0.0, 0.0)],
        WindowEvent::Resized(size) => {
            inbox.size = (size.width, size.height);
            vec![plain(
                RESIZED,
                f64::from(size.width),
                f64::from(size.height),
            )]
        }
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            inbox.scale = scale_factor;
            vec![plain(SCALED, scale_factor, 0.0)]
        }
        WindowEvent::Focused(on) => vec![plain(FOCUSED, flag(on), 0.0)],
        WindowEvent::ModifiersChanged(m) => {
            inbox.modifiers = modifier_bits(&m);
            Vec::new()
        }
        WindowEvent::KeyboardInput { event, .. } if input => {
            let down = event.state == ElementState::Pressed;
            let mut out = Vec::new();
            if let PhysicalKey::Code(code) = event.physical_key {
                let text = format!("{code:?}\t{}", key_name(&event.logical_key));
                out.push(full(KEY, [flag(down), flag(event.repeat), mods, 0.0], text));
            }
            if let (Some(text), true) = (event.text, down) {
                out.push(full(TEXT, [0.0; 4], text.to_string()));
            }
            out
        }
        WindowEvent::Ime(Ime::Commit(text)) if input => vec![full(TEXT, [0.0; 4], text)],
        WindowEvent::Ime(Ime::Preedit(text, at)) if input => {
            let (from, to) = at.map_or((-1.0, -1.0), |(a, b)| {
                (chars_to(&text, a), chars_to(&text, b))
            });
            vec![full(COMPOSING, [from, to, 0.0, 0.0], text)]
        }
        WindowEvent::CursorMoved { position, .. } => {
            inbox.pointer = (position.x, position.y);
            if !input {
                return Vec::new();
            }
            vec![full(
                MOUSE_MOVED,
                [position.x, position.y, mods, 0.0],
                String::new(),
            )]
        }
        WindowEvent::CursorEntered { .. } if input => vec![plain(MOUSE_ENTERED, 0.0, 0.0)],
        WindowEvent::CursorLeft { .. } if input => vec![plain(MOUSE_LEFT, 0.0, 0.0)],
        WindowEvent::MouseInput { state, button, .. } if input => {
            let number = match button {
                MouseButton::Left => 0.0,
                MouseButton::Right => 1.0,
                MouseButton::Middle => 2.0,
                MouseButton::Back => 3.0,
                MouseButton::Forward => 4.0,
                MouseButton::Other(n) => f64::from(n),
            };
            let down = state == ElementState::Pressed;
            let clicks = if down {
                inbox.press.count(number, inbox.pointer, Instant::now())
            } else {
                inbox.press.clicks()
            };
            let numbers = [number, flag(down), f64::from(clicks), mods];
            vec![full(MOUSE_BUTTON, numbers, String::new())]
        }
        WindowEvent::MouseWheel { delta, phase, .. } if input => {
            let (x, y, pixels) = match delta {
                MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y), 0.0),
                MouseScrollDelta::PixelDelta(p) => (p.x, p.y, 1.0),
            };
            let text = phase_name(phase).to_string();
            vec![full(WHEEL, [x, y, pixels, mods], text)]
        }
        WindowEvent::Touch(t) if input => {
            let pressure = t.force.map_or(1.0, |f| f.normalized());
            let mut e = plain(TOUCH, t.location.x, t.location.y);
            e.text = format!("{} {} {pressure}", phase_name(t.phase), t.id);
            vec![e]
        }
        _ => Vec::new(),
    }
}
