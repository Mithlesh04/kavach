// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `Display` in native code (spec 1.11), as in the interpreter
//! (`compiler/interp/src/shown.rs`): the program registers each type's
//! `display` at its start (`DisplayRegister`), and `display.rs` shows the
//! type's values as it gives. A program without `Display` types never
//! fills the table.

use crate::heap::{release, retain, slots, str_of};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Each type's closure around its `display`, kept for the whole program.
static TABLE: OnceLock<Mutex<HashMap<u32, i64>>> = OnceLock::new();

/// Whether any type registered: the common case reads only this.
static ANY: AtomicBool = AtomicBool::new(false);

fn table() -> std::sync::MutexGuard<'static, HashMap<u32, i64>> {
    TABLE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// `DisplayRegister`: values of type number `ty` show as closure
/// `display` (owned) gives.
pub fn register(ty: i64, display: i64) -> i64 {
    if let Some(old) = table().insert(ty as u32, display) {
        release(old);
    }
    ANY.store(true, Ordering::Release);
    static AT_EXIT: std::sync::Once = std::sync::Once::new();
    AT_EXIT.call_once(|| crate::rt::at_exit(forget_all));
    0
}

/// Lets go of the closures at the end, for the leak check.
fn forget_all() {
    ANY.store(false, Ordering::Release);
    for (_, closure) in table().drain() {
        release(closure);
    }
}

/// The text `display()` gives for the value `p` of type number `ty`, when
/// its type implements `Display`.
pub fn own_form(p: i64, ty: u32) -> Option<String> {
    if !ANY.load(Ordering::Acquire) || p == 0 {
        return None;
    }
    let closure = table().get(&ty).copied()?;
    let f = slots(closure)[0] as usize;
    // SAFETY: slot 0 of a closure is its function's address; the code
    // generator compiled a `display` closure as `fn(closure, value) ->
    // String`, borrowing the closure and taking the value (held once more
    // here), and the code lives as long as the program.
    #[allow(unsafe_code)]
    let f: extern "C-unwind" fn(i64, i64) -> i64 = unsafe { std::mem::transmute(f) };
    retain(p);
    let text = f(closure, p);
    let shown = str_of(text);
    release(text);
    Some(shown)
}
