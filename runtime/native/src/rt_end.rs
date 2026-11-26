// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What runs when a program ends, in two lists.
//!
//! - **Finishing hooks** (`at_exit`) finish what the program started:
//!   send the last log events and traces, report a GPU error, leave the
//!   waiting coroutines alone. They run however the program ends: when
//!   `main` returns, on `cli.exit`, on a panic, and on a stop signal
//!   (spec 5.8).
//! - **Freeing hooks** (`free_at_end`) let go of what the runtime kept for
//!   the whole program (the closures of `Drop`, `Display` and JSON checks,
//!   lists kept on the GPU, known keys), so the leak check
//!   (`KAVACH_CHECK_LEAKS`) finds every object freed. They run only when
//!   `main` returns (`run_entry`), after it waited for every task it
//!   spawned (spec 9.2), so no other thread runs Kavach code. On a stop,
//!   `cli.exit` or a panic, other threads may still be using what they
//!   would free: a use after free, and an object given back to a pool
//!   that generated code takes from with no lock. The program ends right
//!   after those, so nothing is lost by keeping it all.

use std::sync::Mutex;

static AT_EXIT: Mutex<Vec<fn()>> = Mutex::new(Vec::new());
static FREE_AT_END: Mutex<Vec<fn()>> = Mutex::new(Vec::new());

/// Runs `f` when the program ends, however it ends: for a feature that
/// must finish something (sending the last log events). `f` must free no
/// object other threads may hold.
pub fn at_exit(f: fn()) {
    AT_EXIT.lock().unwrap_or_else(|e| e.into_inner()).push(f);
}

/// Runs `f` only when `main` returns, to let go of what a feature kept
/// for the whole program, for the leak check.
pub(crate) fn free_at_end(f: fn()) {
    FREE_AT_END
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(f);
}

/// Runs the finishing hooks, once, in order.
pub fn run_at_exit() {
    run(&AT_EXIT);
}

/// Runs the freeing hooks, once, in order: only from `run_entry`, after
/// the finishing hooks.
pub(crate) fn run_free_at_end() {
    run(&FREE_AT_END);
}

/// Ends the program at once with `code`: on a panic, `cli.exit` or a stop
/// signal, after the finishing hooks. Output is flushed first: Rust's,
/// then the C library's (a C function the program called may have
/// printed). Then the process ends and nothing else runs.
/// `std::process::exit` would run this thread's thread-local destructors
/// and the C library's exit handlers. Some of those let go of objects
/// (lists kept on the GPU) after the pools they go back to were
/// destroyed, a crash, and while other threads may still run (unsafe
/// audit (a) 3).
pub fn end_now(code: i32) -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    // SAFETY: `fflush(NULL)` flushes every C output stream; it touches
    // no Rust memory.
    unsafe {
        c::fflush(std::ptr::null_mut());
    }
    quit(code)
}

/// Ends the process with `code` now, flushing nothing and running no
/// destructors or exit handlers: for a second stop signal, which must
/// not wait for a lock another thread may hold.
pub fn quit(code: i32) -> ! {
    #[cfg(unix)]
    // SAFETY: `_exit` ends the process; it runs no handler and touches no
    // memory of ours.
    unsafe {
        libc::_exit(code)
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
        // SAFETY: `GetCurrentProcess` gives this process's pseudo-handle,
        // with every right; ending this process doesn't return.
        unsafe {
            TerminateProcess(GetCurrentProcess(), code as u32);
        }
        // Not reached: the call above ends this thread with the rest.
        std::process::exit(code)
    }
    #[cfg(not(any(unix, windows)))]
    std::process::exit(code)
}

/// The C library's `fflush`: `libc` names it on Unix; on Windows it is
/// the C runtime's, which every Rust program links.
mod c {
    #[cfg(unix)]
    pub(super) use libc::fflush;

    #[cfg(not(unix))]
    unsafe extern "C" {
        pub(super) fn fflush(stream: *mut std::ffi::c_void) -> i32;
    }
}

/// Each hook is taken off `list` before it runs, so one that panics
/// leaves the rest to the panic's own run.
fn run(list: &Mutex<Vec<fn()>>) {
    loop {
        let next = {
            let mut hooks = list.lock().unwrap_or_else(|e| e.into_inner());
            (!hooks.is_empty()).then(|| hooks.remove(0))
        };
        match next {
            Some(f) => f(),
            None => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::heap::{alloc_fixed, release, slots_mut};
    use kavach_kir::Type;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static FINISHED: AtomicUsize = AtomicUsize::new(0);
    static FREED: AtomicUsize = AtomicUsize::new(0);

    /// A `display` closure's function: shows any value as `shown`.
    extern "C-unwind" fn shown(_closure: i64, value: i64) -> i64 {
        release(value);
        crate::heap::new_text("shown", Type::Ref(0))
    }

    /// A stop signal's hooks (`end_hooks`, which `rt_stop` runs) finish,
    /// but free nothing: other threads may still be showing values with
    /// the `Display` closures. `main`'s return frees them. One test, as
    /// the lists are the program's own.
    #[test]
    fn a_stop_finishes_but_frees_nothing() {
        crate::send::tests::set_shapes();
        // The test types' `Pair`, a record of two words: one is enough.
        let pair = Type::Ref(6);
        let closure = alloc_fixed(pair, 0, 2);
        let f: extern "C-unwind" fn(i64, i64) -> i64 = shown;
        slots_mut(closure).copy_from_slice(&[f as usize as u64, 0]);
        crate::rt_display::register(6, closure);
        let value = alloc_fixed(pair, 0, 2);
        super::at_exit(|| {
            FINISHED.fetch_add(1, Ordering::SeqCst);
        });
        super::free_at_end(|| {
            FREED.fetch_add(1, Ordering::SeqCst);
        });
        crate::end_hooks();
        assert_eq!(FINISHED.load(Ordering::SeqCst), 1);
        assert_eq!(FREED.load(Ordering::SeqCst), 0);
        let form = crate::rt_display::own_form(value, 6);
        assert_eq!(form.as_deref(), Some("shown"), "the closure was let go");
        super::run_free_at_end();
        assert_eq!(FREED.load(Ordering::SeqCst), 1);
        assert_eq!(crate::rt_display::own_form(value, 6), None);
        release(value);
    }

    /// Says when it is dropped: thread-locals like it (lists kept on the
    /// GPU) let go of objects, after the pools they go back to are gone.
    struct Kept;

    impl Drop for Kept {
        fn drop(&mut self) {
            eprintln!("a thread-local was dropped");
        }
    }

    thread_local! {
        static KEPT: Kept = const { Kept };
    }

    /// A panic ends the program with 101, its output flushed, and runs no
    /// thread-local destructors: in a child run of this same test, as the
    /// panic ends the process.
    #[test]
    fn a_panic_ends_without_running_thread_local_destructors() {
        const CHILD: &str = "KAVACH_TEST_END_CHILD";
        if std::env::var_os(CHILD).is_some() {
            KEPT.with(|_| ());
            // No newline: still in the buffer when the panic comes.
            print!("printed before");
            crate::rt::panic_with("boom", 0);
        }
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "rt_end::tests::a_panic_ends_without_running_thread_local_destructors",
            ])
            .args(["--nocapture", "--test-threads=1"])
            .env(CHILD, "1")
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(101), "{said}");
        assert!(said.contains("panic: boom"), "{said}");
        assert!(!said.contains("dropped"), "{said}");
        assert!(String::from_utf8_lossy(&out.stdout).contains("printed before"));
    }
}
