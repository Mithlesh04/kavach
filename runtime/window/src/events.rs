// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What a window event means to a program (spec 14.22): `winit`'s events
//! turned into `std.window`'s, numbered as its `Event` variants are.

use crate::inbox::Inbox;
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent};
use winit::keyboard::PhysicalKey;

/// The kinds of event, numbered as `std.window`'s `Event` variants are.
pub const CLOSED: i64 = 0;
pub const RESIZED: i64 = 1;
pub const SCALED: i64 = 2;
pub const FOCUSED: i64 = 3;
pub const KEY: i64 = 4;
pub const TEXT: i64 = 5;
pub const MOUSE_MOVED: i64 = 6;
pub const MOUSE_BUTTON: i64 = 7;
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

/// One event: its kind, two numbers, and the text of a key or of typing.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub kind: i64,
    pub a: f64,
    pub b: f64,
    pub text: String,
}

impl Event {
    /// The numbers `std.window` reads for it, in order.
    pub fn numbers(&self) -> [f64; 3] {
        [self.kind as f64, self.a, self.b]
    }
}

pub(crate) fn plain(kind: i64, a: f64, b: f64) -> Event {
    Event {
        kind,
        a,
        b,
        text: String::new(),
    }
}

/// What a window event means to a program, keeping the inbox's size and
/// scale up to date; input events only once it listens.
pub(crate) fn translate(event: WindowEvent, inbox: &mut Inbox) -> Vec<Event> {
    let input = inbox.input;
    let flag = |on: bool| if on { 1.0 } else { 0.0 };
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
        WindowEvent::KeyboardInput { event, .. } if input => {
            let down = event.state == ElementState::Pressed;
            let mut out = Vec::new();
            if let (PhysicalKey::Code(code), false) = (event.physical_key, event.repeat) {
                let mut e = plain(KEY, flag(down), 0.0);
                e.text = format!("{code:?}");
                out.push(e);
            }
            if let (Some(text), true) = (event.text, down) {
                let mut e = plain(TEXT, 0.0, 0.0);
                e.text = text.to_string();
                out.push(e);
            }
            out
        }
        WindowEvent::Ime(Ime::Commit(text)) if input => {
            let mut e = plain(TEXT, 0.0, 0.0);
            e.text = text;
            vec![e]
        }
        WindowEvent::CursorMoved { position, .. } if input => {
            vec![plain(MOUSE_MOVED, position.x, position.y)]
        }
        WindowEvent::MouseInput { state, button, .. } if input => {
            let number = match button {
                MouseButton::Left => 0.0,
                MouseButton::Right => 1.0,
                MouseButton::Middle => 2.0,
                MouseButton::Back => 3.0,
                MouseButton::Forward => 4.0,
                MouseButton::Other(n) => f64::from(n),
            };
            vec![plain(
                MOUSE_BUTTON,
                number,
                flag(state == ElementState::Pressed),
            )]
        }
        WindowEvent::MouseWheel { delta, .. } if input => {
            let (x, y) = match delta {
                MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y)),
                // About 20 pixels make a line, as browsers count them.
                MouseScrollDelta::PixelDelta(p) => (p.x / 20.0, p.y / 20.0),
            };
            vec![plain(WHEEL, x, y)]
        }
        WindowEvent::Touch(t) if input => {
            let phase = match t.phase {
                TouchPhase::Started => "Began",
                TouchPhase::Moved => "Moved",
                TouchPhase::Ended => "Ended",
                TouchPhase::Cancelled => "Cancelled",
            };
            let pressure = t.force.map_or(1.0, |f| f.normalized());
            let mut e = plain(TOUCH, t.location.x, t.location.y);
            e.text = format!("{phase} {} {pressure}", t.id);
            vec![e]
        }
        _ => Vec::new(),
    }
}
