//! Run-time functions for errors as values (spec 5): `context`, and the
//! error a failed built-in call (`parse`) leaves behind.

use crate::display::display;
use crate::heap::{Data, alloc, release};
use crate::rt::type_of;
use kavach_kir::Type;
use std::cell::Cell;

thread_local! {
    /// The error of the last built-in call that failed, until taken.
    static LAST_ERROR: Cell<i64> = const { Cell::new(0) };
}

/// Puts `p` in place of this thread's last error, giving that one: tasks
/// switching on one thread keep their own (`rt_coroutines.rs`).
pub(crate) fn swap_error(p: i64) -> i64 {
    LAST_ERROR.replace(p)
}

pub fn set_error(p: i64) {
    release(LAST_ERROR.replace(p));
}

/// The error of the built-in call just made, or 0 when it succeeded.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_take_error() -> i64 {
    LAST_ERROR.replace(0)
}

/// The error `error` holds, past any `context`, when it is of type
/// `want`, as a new reference; 0 (`none`) otherwise: a `match` on an
/// `Error` (spec 5.5). `error` is borrowed.
pub fn error_as(error: i64, want: Type) -> i64 {
    let mut p = error;
    while p != 0 {
        let inner = match &*crate::heap::data(p).borrow() {
            Data::Context(c) => c.1 as i64,
            _ => break,
        };
        p = inner;
    }
    if p == 0 || crate::heap::obj(p).ty() != want {
        return 0;
    }
    crate::rc::retain(p);
    p
}

/// `error` with `text` added in front (`try … context "…"`, spec 5.7).
/// Both references move into the new error.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_context(text: i64, error: i64, code: i64) -> i64 {
    let message = display(text, crate::heap::obj(text).ty(), true);
    release(text);
    alloc(
        type_of(code),
        Data::Context(Box::new((message, error as u64))),
    )
}
