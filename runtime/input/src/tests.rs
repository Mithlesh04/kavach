//! The standard layout and the state kept from events, with events made
//! up here: no gamepad needs to be connected.

use crate::{BUTTONS, Change, Pad, apply, axis, button, trigger_button};
use sdl3::gamepad::{Axis, Button};
use std::collections::BTreeMap;

#[test]
fn sdl_events_are_named_by_place_in_the_standard_layout() {
    assert_eq!(button(Button::South), Some(0));
    assert_eq!(button(Button::LeftShoulder), Some(4), "the left bumper");
    assert_eq!(button(Button::Back), Some(8), "select");
    assert_eq!(button(Button::DPadRight), Some(16));
    assert_eq!(button(Button::Misc1), None);
    for b in [
        Button::South,
        Button::Start,
        Button::RightStick,
        Button::DPadUp,
    ] {
        assert!(BUTTONS[button(b).unwrap_or(0)].len() > 2);
    }
    // SDL's y is down; the standard layout's is up.
    assert_eq!(axis(Axis::LeftY, -32_767), Change::Axis("LeftY", 1.0));
    assert_eq!(
        axis(Axis::RightX, 16_384),
        Change::Axis("RightX", 16_384.0 / 32_767.0)
    );
    assert_eq!(
        axis(Axis::TriggerLeft, 32_767),
        Change::Axis("LeftTrigger", 1.0)
    );
    assert_eq!(axis(Axis::LeftX, i16::MIN), Change::Axis("LeftX", -1.0));
    // A trigger is a button too, held from halfway.
    assert_eq!(
        trigger_button("LeftTrigger", 0.2, 0.7),
        Some(Change::Button(6, true))
    );
    assert_eq!(
        trigger_button("RightTrigger", 0.9, 0.1),
        Some(Change::Button(7, false))
    );
    assert_eq!(trigger_button("RightTrigger", 0.6, 0.9), None);
    assert_eq!(trigger_button("LeftX", 0.0, 1.0), None);
}

#[test]
fn a_pads_state_follows_its_events() {
    let mut pads = BTreeMap::new();
    let e = apply(&mut pads, 3, Change::Connected("Xbox Controller".into()));
    assert_eq!((e.kind, e.a, e.text.as_str()), (10, 3.0, "Xbox Controller"));
    apply(&mut pads, 3, Change::Button(0, true));
    apply(&mut pads, 3, Change::Button(9, true));
    let up = apply(&mut pads, 3, Change::Button(0, false));
    assert_eq!((up.kind, up.b, up.text.as_str()), (12, 0.0, "South"));
    apply(&mut pads, 3, Change::Axis("LeftX", 0.25));
    apply(&mut pads, 3, Change::Axis("RightY", -2.0));
    let t = apply(&mut pads, 3, Change::Axis("LeftTrigger", 0.75));
    assert_eq!((t.kind, t.b, t.text.as_str()), (13, 0.75, "LeftTrigger"));
    assert_eq!(
        pads[&3],
        Pad {
            id: 3,
            name: "Xbox Controller".into(),
            buttons: 1 << 9,
            left: (0.25, 0.0),
            right: (0.0, -1.0),
            triggers: (0.75, 0.0),
            gyro: None,
        }
    );
    let gone = apply(&mut pads, 3, Change::Gone);
    assert_eq!(gone.kind, 11);
    assert!(pads.is_empty());
}
