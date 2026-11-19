// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The task panics printed so far (spec 9.1): a panic is printed when it
//! happens, and `.wait()` gives it as the error `panicked: … at …`. When
//! that error ends `main`, the last line says the panic above ended the
//! program instead of printing it again. An `Error` a program makes with
//! the same words isn't one of these, and prints in full.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

fn shown() -> std::sync::MutexGuard<'static, HashSet<String>> {
    static SHOWN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SHOWN
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Notes that the panic `.wait()` gives as `why` was printed.
pub fn panic_shown(why: &str) {
    shown().insert(why.to_string());
}

/// The line that ends a program whose `main` failed with error `text`.
pub fn main_error_line(text: &str) -> String {
    if shown().contains(text) {
        "error: the panic above ended the program".to_string()
    } else {
        format!("error: {text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_printed_once_is_not_printed_again() {
        let why = "panicked: index 3 is out of range at t.kav:3 (shown-test)";
        assert_eq!(main_error_line(why), format!("error: {why}"));
        panic_shown(why);
        assert_eq!(
            main_error_line(why),
            "error: the panic above ended the program"
        );
    }
}
