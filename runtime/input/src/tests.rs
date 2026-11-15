//! The standard layout and the state kept from events, with events made
//! up here: no gamepad needs to be connected.

use crate::{BUTTONS, Change, Pad, apply, button, change_of, stick, trigger};
use gilrs::{Axis, Button, EventType};
use std::collections::BTreeMap;

#[test]
fn gilrs_events_are_named_by_place_in_the_standard_layout() {
    assert_eq!(button(Button::South), Some(0));
    assert_eq!(button(Button::LeftTrigger), Some(4), "the left bumper");
    assert_eq!(button(Button::LeftTrigger2), Some(6), "the left trigger");
    assert_eq!(button(Button::DPadRight), Some(16));
    assert_eq!(button(Button::Unknown), None);
    for b in [
        Button::South,
        Button::Start,
        Button::RightThumb,
        Button::DPadUp,
    ] {
        assert!(BUTTONS[button(b).unwrap_or(0)].len() > 2);
    }
    assert_eq!(
        trigger(Button::RightTrigger2, 0.5),
        Some(Change::Axis("RightTrigger", 0.5))
    );
    assert_eq!(trigger(Button::South, 0.5), None);
    assert_eq!(
        stick(Axis::LeftStickY, -1.0),
        Some(Change::Axis("LeftY", -1.0))
    );
    assert_eq!(stick(Axis::LeftZ, 0.3), None);
    assert_eq!(
        change_of(&EventType::Connected, "Pad"),
        Some(Change::Connected("Pad".into()))
    );
    assert_eq!(
        change_of(&EventType::Disconnected, "Pad"),
        Some(Change::Gone)
    );
    assert_eq!(change_of(&EventType::Dropped, "Pad"), None);
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
        }
    );
    let gone = apply(&mut pads, 3, Change::Gone);
    assert_eq!(gone.kind, 11);
    assert!(pads.is_empty());
}
