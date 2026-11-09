//! The clipboard in native code (spec 14.17, decision 0066), from
//! `kavach_runtime_core::clipboard`. A module and entry of its own, so
//! only a program that uses the clipboard links `arboard` (decision 0024).

use crate::heap::{release, str_of};
use crate::rt::type_of;
use crate::rt_errors::set_error;
use kavach_kir::{RtOp, Shape};
use kavach_runtime_core::clipboard::{self, ClipError};

/// Leaves `e` as a `ClipboardError` for `kv_take_error`.
fn fail(e: ClipError) -> i64 {
    let Some(ty) = crate::shapes::find_union("ClipboardError", "Empty") else {
        return 0;
    };
    let error = match e {
        ClipError::Empty => crate::rt_json::new_variant(ty, "Empty", &[]),
        ClipError::Unavailable(why) => {
            let text = crate::shapes::new_text(&why, crate::heap::first_of(&Shape::Str));
            crate::rt_json::new_variant(ty, "Unavailable", &[text as u64])
        }
    };
    set_error(error);
    0
}

crate::rt_data::entry!(kv_rt_clipboard, |op, a, _b, _c, code, _line| {
    match op {
        RtOp::ClipReadText => match clipboard::read_text() {
            Ok(text) => crate::heap::new_str(text, type_of(code)),
            Err(e) => fail(e),
        },
        // `a` is the text (owned).
        RtOp::ClipWriteText => {
            let text = str_of(a);
            release(a);
            clipboard::write_text(&text).map_or_else(fail, |()| 0)
        }
        _ => unreachable!("not a clipboard op: {op:?}"),
    }
});
