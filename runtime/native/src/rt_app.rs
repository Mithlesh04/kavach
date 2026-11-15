//! `std.app` in native code (spec 14.30, decision 0118): the program's
//! folders and crash reports from `kavach_runtime_core::app`, and the
//! dialogs from `kavach_runtime_dialog`, as the interpreter does. The
//! dialogs have an entry of their own, so only a program that shows one
//! links `rfd` (decision 0024).

use crate::heap::{new_str, release};
use crate::rt::type_of;
use crate::rt_errors::set_error;
use crate::shapes::str_of;
use kavach_kir::{RtOp, Shape};
use kavach_runtime_core::app::{self, Kind};

/// Leaves `why` as an `AppError.Unavailable` for `kv_take_error`.
fn fail(why: &str) -> i64 {
    if let Some(ty) = crate::shapes::find_union("AppError", "Unavailable") {
        let text = crate::shapes::new_text(why, crate::heap::first_of(&Shape::Str));
        set_error(crate::rt_json::new_variant(
            ty,
            "Unavailable",
            &[text as u64],
        ));
    }
    0
}

/// The text `p` (owned).
fn take(p: i64) -> String {
    let t = str_of(p);
    release(p);
    t
}

// `AppDir`: `a` the kind, `b` the name (owned). `AppCrashReports`: `a`
// the folder (owned).
crate::rt_data::entry!(kv_rt_app, |op, a, b, _c, code, _line| {
    match op {
        RtOp::AppDir => {
            let kind = [Kind::Data, Kind::Config, Kind::Cache][a.clamp(0, 2) as usize];
            match app::folder(kind, &take(b)) {
                Ok(path) => new_str(path, type_of(code)),
                Err(why) => fail(&why),
            }
        }
        _ => match app::crash_reports(&take(a)) {
            Ok(()) => 0,
            Err(why) => fail(&why),
        },
    }
});

// `a` is which dialog, `b` its title and `c` its text (both owned).
crate::rt_data::entry!(kv_rt_dialog, |_op, a, b, c, code, _line| {
    let (title, text) = (take(b), take(c));
    new_str(kavach_runtime_dialog::show(a, &title, &text), type_of(code))
});
