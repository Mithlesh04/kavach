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
crate::rt_data::entry!(kv_rt_window, |op, a, b, c, code, _line| {
    match op {
        RtOp::WindowOpen => {
            let title = str_of(a);
            release(a);
            win::open(&title, b, c).unwrap_or_else(|why| fail(&why))
        }
        // Waiting holds the task's turn (spec 14.22).
        RtOp::WindowEvents => {
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
        _ => {
            win::close(a);
            0
        }
    }
});
