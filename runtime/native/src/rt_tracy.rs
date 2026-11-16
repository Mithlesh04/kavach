//! `kavach build --tracy` (spec 14.12, decision 0123): the program's
//! `main` calls this first, so only a program built so links Tracy's
//! client (decision 0024).

/// Starts Tracy's client and sends it every span from now on.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C-unwind" fn kv_tracy_start() {
    kavach_runtime_tracy::start();
}
