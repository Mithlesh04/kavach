//! Events translated, and a window opened or refused, on any machine.

use super::*;

#[test]
fn input_events_come_only_after_listening() {
    let mut inbox = Inbox::default();
    let moved = || WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: winit::dpi::PhysicalPosition::new(3.0, 4.0),
    };
    assert!(translate(moved(), &mut inbox).is_empty());
    inbox.input = true;
    assert_eq!(
        translate(moved(), &mut inbox),
        vec![plain(MOUSE_MOVED, 3.0, 4.0)]
    );
    let resized = WindowEvent::Resized(PhysicalSize::new(640, 480));
    assert_eq!(
        translate(resized, &mut inbox),
        vec![plain(RESIZED, 640.0, 480.0)]
    );
    assert_eq!(inbox.size, (640, 480));
    let touched = WindowEvent::Touch(winit::event::Touch {
        device_id: DeviceId::dummy(),
        phase: TouchPhase::Started,
        location: winit::dpi::PhysicalPosition::new(10.0, 20.0),
        force: Some(winit::event::Force::Normalized(0.5)),
        id: 3,
    });
    let mut began = plain(TOUCH, 10.0, 20.0);
    began.text = "Began 3 0.5".into();
    assert_eq!(translate(touched, &mut inbox), vec![began]);
}

#[test]
fn a_window_opens_or_says_why_not() {
    // A machine with no screen (a server, CI) has no windows: that is
    // an answer too, never a hang or a crash.
    match open("kavach test", 64, 48) {
        Ok(n) => {
            assert!(size(n).0 > 0);
            close(n);
        }
        Err(why) => assert!(!why.is_empty()),
    }
}
