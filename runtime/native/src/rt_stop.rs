//! Ctrl+C, `SIGTERM` and closing the console (spec 5.8, decisions 0062
//! and 0063): the program ends as `cli.exit` does, with its end hooks
//! run and its output flushed, and exit code 130 (`SIGINT`), 143
//! (`SIGTERM`) or 129 (`SIGHUP`, the terminal closed). A second one
//! ends it at once. Holds (`cli.on_stop`, decision 0088) get their grace
//! first (`kavach_runtime_core::stop_holds`).

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// What runs before the program ends: the end hooks of native code, or
/// of the interpreter.
static HOOKS: OnceLock<fn()> = OnceLock::new();

/// Set when the first signal came: a second one doesn't wait for hooks.
static STOPPING: AtomicBool = AtomicBool::new(false);

/// Catches the signals that stop a program, so `hooks` runs before it
/// ends. Call it before any other thread starts: on Unix the signals are
/// blocked in every thread and one waits for them.
pub fn catch_stops(hooks: fn()) {
    if HOOKS.set(hooks).is_err() {
        return;
    }
    os::install();
}

/// A stop signal came, with the exit code it gives: the hooks run, then
/// the program ends; when one came already, it ends at once.
fn stop(code: i32) {
    if STOPPING.swap(true, Ordering::SeqCst) {
        std::process::exit(code);
    }
    kavach_runtime_core::stop_holds::let_holders_finish(code);
    if let Some(hooks) = HOOKS.get() {
        hooks();
    }
    std::process::exit(code);
}

#[cfg(unix)]
mod os {
    /// The signals that stop a program, with their exit codes.
    const STOPS: [(libc::c_int, i32); 3] = [
        (libc::SIGINT, 130),
        (libc::SIGTERM, 143),
        (libc::SIGHUP, 129),
    ];

    fn set() -> libc::sigset_t {
        // SAFETY: `sigemptyset` and `sigaddset` fill in the set they are
        // given, a local zeroed `sigset_t`, with signal numbers libc names.
        unsafe {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            for (s, _) in STOPS {
                libc::sigaddset(&mut set, s);
            }
            set
        }
    }

    pub(super) fn install() {
        let set = set();
        // SAFETY: blocks the stop signals in this thread, and so in every
        // thread started after it; `set` is a valid, filled-in set.
        unsafe {
            libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());
        }
        std::thread::spawn(move || {
            loop {
                let mut got: libc::c_int = 0;
                // SAFETY: waits for one of the blocked signals in `set`,
                // writing its number into the local `got`.
                if unsafe { libc::sigwait(&set, &mut got) } != 0 {
                    continue;
                }
                let code = STOPS
                    .iter()
                    .find(|(s, _)| *s == got)
                    .map_or(130, |(_, c)| *c);
                // On a thread of its own, so a second signal is heard.
                std::thread::spawn(move || super::stop(code));
            }
        });
    }
}

#[cfg(windows)]
mod os {
    use windows_sys::Win32::System::Console::{
        CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, SetConsoleCtrlHandler,
    };

    /// Called by Windows on a thread of its own for each event, so a
    /// second Ctrl+C is heard while the first runs the hooks; Windows ends
    /// the program when this returns from a close, so the hooks run here.
    /// Ctrl+C and Ctrl+Break are `SIGINT`, closing the console is
    /// `SIGHUP`, and logging off and shutting down are `SIGTERM`.
    unsafe extern "system" fn handler(event: u32) -> i32 {
        let code = match event {
            CTRL_C_EVENT | CTRL_BREAK_EVENT => 130,
            CTRL_CLOSE_EVENT => 129,
            _ => 143,
        };
        super::stop(code);
        1
    }

    pub(super) fn install() {
        // SAFETY: `handler` has the signature Windows calls and lives as
        // long as the program.
        unsafe {
            SetConsoleCtrlHandler(Some(handler), 1);
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod os {
    pub(super) fn install() {}
}
