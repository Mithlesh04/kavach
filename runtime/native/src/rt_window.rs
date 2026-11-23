// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.window` in native code (spec 14.22, decision 0104), from
//! `kavach_runtime_window`, by the window's number, as the interpreter
//! does. An entry of its own, so only a program that opens windows links
//! `winit` (decision 0024).

use crate::heap::{alloc_slots, new_str, release, shape, str_of};
use crate::rt::type_of;
use crate::rt_errors::set_error;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_window as win;

/// Leaves `why` as a `WindowError.Unavailable` for `kv_take_error`.
fn fail(why: &str) -> i64 {
    let Some(ty) = crate::shapes::find_union("WindowError", "Unavailable") else {
        return 0;
    };
    let text = crate::shapes::new_text(why, crate::heap::first_of(&Shape::Str));
    set_error(crate::rt_json::new_variant(
        ty,
        "Unavailable",
        &[text as u64],
    ));
    0
}

/// The item type of the list type `code`.
fn item(code: i64) -> Type {
    match shape(type_of(code)) {
        Shape::List(t) => *t,
        _ => Type::I64,
    }
}

// `a` is the window's number, or the title (owned) for `WindowOpen`;
// `b` and `c` are the width and height, a flag, or a title (owned).
/// Waits until window `n` has an event or is closed, while other tasks
/// run (spec 14.22); without tasks, `take` waits alone.
fn wait_for_events(n: i64, line: i32) {
    while !win::ready(n) {
        let me = crate::rt_tasks::me();
        let tasks = crate::rt_tasks::wait_outside(None, line, |wake| {
            win::on_event(
                n,
                Box::new(move || {
                    wake();
                    kavach_runtime_core::tasks::interrupt_queue_of(me);
                }),
            )
        });
        if !tasks || crate::rt_tasks::cancelling() {
            return;
        }
    }
}

crate::rt_data::entry!(kv_rt_window, |op, a, b, c, code, line| {
    match op {
        RtOp::WindowOpen => {
            let title = str_of(a);
            release(a);
            win::open(&title, b, c).unwrap_or_else(|why| fail(&why))
        }
        // Other tasks run while it waits (spec 14.22).
        RtOp::WindowEvents => {
            if b != 0 {
                wait_for_events(a, line);
            }
            let numbers = win::take_numbers(a, b != 0);
            alloc_slots(
                type_of(code),
                0,
                numbers.iter().map(|f| f.to_bits()).collect(),
            )
        }
        RtOp::WindowTexts => {
            let t = item(code);
            let texts = win::texts(a)
                .into_iter()
                .map(|s| new_str(s, t) as u64)
                .collect();
            alloc_slots(type_of(code), 0, texts)
        }
        RtOp::WindowSize => {
            let (w, h) = win::size(a);
            alloc_slots(type_of(code), 0, vec![w as u64, h as u64])
        }
        RtOp::WindowScale => win::scale(a).to_bits() as i64,
        RtOp::WindowListen => {
            win::listen_input(a);
            0
        }
        RtOp::WindowTitle => {
            let title = str_of(b);
            release(b);
            win::set_title(a, &title);
            0
        }
        RtOp::WindowFullscreen => {
            win::set_fullscreen(a, b != 0);
            0
        }
        RtOp::WindowLockCursor => {
            win::lock_cursor(a, b != 0);
            0
        }
        RtOp::WindowWake => {
            win::wake(a);
            0
        }
        RtOp::WindowTextInput => {
            win::set_text_input(a, b != 0);
            0
        }
        // `b` is the list (owned) of `x`, `y`, width and height.
        RtOp::WindowCaret => {
            let all: Vec<f64> = (crate::heap::slots(b).iter())
                .map(|s| f64::from_bits(*s))
                .collect();
            release(b);
            let part = |k: usize| all.get(k).copied().unwrap_or(0.0);
            win::set_caret(a, [part(0), part(1), part(2), part(3)]);
            0
        }
        _ => {
            win::close(a);
            0
        }
    }
});
