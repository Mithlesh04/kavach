// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Events translated, waits woken, and a window opened or refused, on
//! any machine. Inboxes of made-up windows, numbered below 0, stand for
//! windows: no test needs a screen but the last.

use super::events::{plain, translate};
use super::inbox::{Inbox, hub, shown, wait_for_frame};
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceId, TouchPhase, WindowEvent};

#[test]
fn input_events_come_only_after_listening() {
    let mut inbox = Inbox::default();
    let moved = || WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: winit::dpi::PhysicalPosition::new(3.0, 4.0),
    };
    assert!(translate(moved(), &mut inbox).is_empty());
    assert_eq!(inbox.pointer, (3.0, 4.0), "kept for counting clicks");
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

/// A made-up window numbered `n`, with nothing in its inbox.
fn fake(n: i64) {
    hub().add(n, Inbox::default());
}

#[test]
fn redraws_and_wakes_come_once_until_taken() {
    fake(-1);
    let redraw = translate(WindowEvent::RedrawRequested, &mut Inbox::default());
    assert_eq!(redraw, vec![plain(REDRAW, 0.0, 0.0)]);
    wake(-1);
    wake(-1);
    hub().change(-1, |inbox| {
        inbox.push(plain(REDRAW, 0.0, 0.0));
        inbox.push(plain(RESIZED, 8.0, 6.0));
        inbox.push(plain(REDRAW, 0.0, 0.0));
        true
    });
    let kinds: Vec<i64> = take(-1, false).iter().map(|e| e.kind).collect();
    assert_eq!(kinds, [WOKEN, REDRAW, RESIZED]);
    wake(-1);
    assert_eq!(take(-1, true), vec![plain(WOKEN, 0.0, 0.0)]);
    hub().remove(-1);
    wake(-1);
    assert!(take(-1, true).is_empty(), "a closed window has no events");
}

#[test]
fn a_waiting_task_is_woken_by_an_event_or_the_close() {
    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let counter = || {
        let c = std::sync::Arc::clone(&calls);
        Box::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
        }) as Waker
    };
    fake(-2);
    assert!(on_event(-2, counter()), "nothing came yet: wait");
    assert!(on_event(-2, counter()));
    wake(-2);
    assert_eq!(calls.load(Ordering::SeqCst), 2, "both waiters woken");
    assert!(!on_event(-2, counter()), "an event is there: no wait");
    take(-2, false);
    assert!(on_event(-2, counter()));
    hub().remove(-2);
    assert_eq!(calls.load(Ordering::SeqCst), 3, "closing wakes the waiter");
    assert!(!on_event(-2, counter()), "a closed window: look again now");
    // From another thread, as a task spawned on another core calls it.
    fake(-3);
    let waiter = std::thread::spawn(|| take(-3, true));
    std::thread::sleep(Duration::from_millis(20));
    std::thread::spawn(|| wake(-3)).join().unwrap();
    assert_eq!(waiter.join().unwrap(), vec![plain(WOKEN, 0.0, 0.0)]);
    hub().remove(-3);
}

#[test]
fn the_event_loop_waits_a_little_for_a_frame() {
    fake(-4);
    assert_eq!(shown(-4), Some(0));
    let quick = Duration::from_millis(5);
    assert!(!wait_for_frame(-4, 0, quick), "no frame came");
    let drawer = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(10));
        frame_shown(-4);
    });
    assert!(wait_for_frame(-4, 0, Duration::from_secs(10)));
    drawer.join().unwrap();
    assert_eq!(shown(-4), Some(1));
    hub().remove(-4);
    assert!(!wait_for_frame(-4, 1, Duration::from_secs(10)), "closed");
    assert_eq!(shown(-4), None);
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

#[test]
fn text_field_events_carry_what_a_text_box_needs() {
    use super::events::{Press, key_name, modifier_bits};
    use winit::event::{Ime, Modifiers, MouseScrollDelta};
    use winit::keyboard::{Key, ModifiersState, NamedKey};
    let mut inbox = Inbox {
        input: true,
        ..Inbox::default()
    };
    // Names as the W3C's `KeyboardEvent.key` gives them.
    assert_eq!(key_name(&Key::Character("z".into())), "z");
    assert_eq!(key_name(&Key::Named(NamedKey::ArrowLeft)), "ArrowLeft");
    assert_eq!(key_name(&Key::Named(NamedKey::Space)), " ");
    let held = Modifiers::from(ModifiersState::CONTROL | ModifiersState::SUPER);
    assert_eq!(modifier_bits(&held), CTRL | META);
    assert!(translate(WindowEvent::ModifiersChanged(held), &mut inbox).is_empty());
    assert_eq!(inbox.modifiers, CTRL | META);
    // Composing text: its selected part in characters, not bytes.
    let preedit = WindowEvent::Ime(Ime::Preedit("日本語".into(), Some((3, 6))));
    let composing = translate(preedit, &mut inbox);
    assert_eq!(
        composing[0].numbers(),
        [COMPOSING as f64, 1.0, 2.0, 0.0, 0.0]
    );
    assert_eq!(composing[0].text, "日本語");
    let hidden = WindowEvent::Ime(Ime::Preedit(String::new(), None));
    assert_eq!(translate(hidden, &mut inbox)[0].a, -1.0);
    // A touchpad scrolls in pixels, with its phase and the modifiers.
    let wheel = WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta: MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(0.0, -12.5)),
        phase: TouchPhase::Started,
    };
    let scrolled = translate(wheel, &mut inbox);
    assert_eq!(scrolled[0].numbers(), [WHEEL as f64, 0.0, -12.5, 1.0, 10.0]);
    assert_eq!(scrolled[0].text, "Began");
    let entered = WindowEvent::CursorEntered {
        device_id: DeviceId::dummy(),
    };
    assert_eq!(translate(entered, &mut inbox)[0].kind, MOUSE_ENTERED);
    // Clicks: quick presses in one place count up; another button, a
    // move or a pause starts again.
    let mut p = Press::default();
    let t = std::time::Instant::now();
    let ms = |n| t + Duration::from_millis(n);
    assert_eq!(p.count(0.0, (10.0, 10.0), t), 1);
    assert_eq!(p.count(0.0, (12.0, 9.0), ms(300)), 2);
    assert_eq!(p.count(0.0, (12.0, 9.0), ms(600)), 3);
    assert_eq!(p.clicks(), 3, "the release gives the press's count");
    assert_eq!(p.count(1.0, (12.0, 9.0), ms(700)), 1, "another button");
    assert_eq!(p.count(1.0, (30.0, 9.0), ms(800)), 1, "another place");
    assert_eq!(p.count(1.0, (30.0, 9.0), ms(1400)), 1, "too slow");
}
