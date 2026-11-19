// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.fs`'s `Watcher` in native code (spec 14.3, decision 0116), from
//! `kavach_runtime_watch`, by the watcher's number, as the interpreter
//! does (`compiler/interp/src/std_watch.rs`). A path is checked against
//! `uses Fs("…")` when its watcher starts. An entry of its own, so only a
//! program that watches files links `notify`.

use crate::heap::{alloc_slots, new_str, release, shape};
use crate::rt::type_of;
use crate::shapes::str_of;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_watch as watch;

/// A change as `std.fs` reads it: its kind's digit, a space, the path.
fn row((kind, path): (u8, String)) -> String {
    format!("{kind} {path}")
}

// `a` is the path (owned) for `FsWatchOpen`, else the watcher's number.
crate::rt_data::entry!(kv_rt_watch, |op, a, _b, _c, code, _line| {
    match op {
        RtOp::FsWatchOpen => {
            let path = str_of(a);
            release(a);
            if !crate::rt_scopes::allowed(&path) {
                return crate::rt_files::denied(&path);
            }
            watch::watch(&path).unwrap_or_else(|e| crate::rt_std::fail("watch", &path, e))
        }
        // Waiting holds the task's turn, as a window's events do.
        RtOp::FsWatchNext => {
            let text = watch::next(a).map(row).unwrap_or_default();
            new_str(text, type_of(code))
        }
        RtOp::FsWatchPoll => {
            let item = match shape(type_of(code)) {
                Shape::List(t) => *t,
                _ => Type::I64,
            };
            let rows = (watch::poll(a).into_iter())
                .map(|c| new_str(row(c), item) as u64)
                .collect();
            alloc_slots(type_of(code), 0, rows)
        }
        _ => {
            watch::close(a);
            0
        }
    }
});
