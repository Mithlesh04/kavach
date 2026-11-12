//! The run-time library of native Kavach programs (decision 0016): heap
//! values with reference counting, printing, panics and errors.
//!
//! It is used two ways: linked into the compiler for `kavach run --native`
//! (the JIT looks functions up by address), and as a static library
//! linked into executables by `kavach build` (by symbol name, so every
//! function native code calls is `#[unsafe(no_mangle)] extern
//! "C-unwind"`, so a panic can unwind through it: decision 0065).
//!
//! `unsafe` code is allowed in this crate only. Exporting unmangled
//! symbols is `unsafe` in Rust 2024; each other use has a `SAFETY:` comment.

#![allow(unsafe_code)]

pub mod display;
pub mod flat;
pub mod hashing;
pub mod heap;
mod lib_calls;
pub mod lists;
mod live;
mod opaque;
pub mod pool;
pub mod radix;
mod rc;
pub mod rt;
mod rt_agents;
mod rt_ai;
mod rt_approvals;
mod rt_base64;
pub mod rt_big_decimal;
pub mod rt_bigint;
mod rt_bytes;
mod rt_channels;
pub mod rt_cli;
mod rt_clipboard;
mod rt_coroutines;
mod rt_crypto;
pub mod rt_data;
mod rt_drops;
pub mod rt_errors;
mod rt_events;
mod rt_file_info;
mod rt_files;
mod rt_gpu;
mod rt_journal;
pub mod rt_json;
pub mod rt_json_build;
mod rt_json_checks;
pub mod rt_json_decode;
mod rt_json_pull;
mod rt_json_read;
pub mod rt_json_tape;
mod rt_json_write;
mod rt_log;
pub mod rt_maps;
mod rt_memory;
mod rt_net;
pub mod rt_numbers;
mod rt_otlp;
pub mod rt_par;
mod rt_process;
mod rt_ptr;
mod rt_random;
pub mod rt_regex;
mod rt_replays;
mod rt_serial;
pub mod rt_std;
pub mod rt_stop;
mod rt_task_errors;
mod rt_tasks;
mod rt_under_way;
mod rt_waits;
mod rt_workers;
mod rt_zone;
mod small_blocks;
// The entries of the feature groups, for code the JIT compiles.
pub use rt_agents::kv_rt_agents;
pub use rt_ai::kv_rt_ai;
pub use rt_base64::kv_rt_base64;
pub use rt_clipboard::kv_rt_clipboard;
pub use rt_crypto::kv_rt_crypto;
pub use rt_drops::kv_rt_drops;
pub use rt_file_info::kv_rt_file_info;
pub use rt_gpu::kv_rt_gpu;
pub use rt_journal::kv_rt_journal;
pub use rt_log::kv_rt_log;
pub use rt_net::kv_rt_net;
pub use rt_otlp::kv_rt_otlp;
pub use rt_process::kv_rt_process;
pub use rt_random::kv_rt_random;
pub use rt_serial::kv_rt_serial;
pub use rt_std::kv_rt_version;
pub use rt_tasks::kv_rt_tasks;
pub use rt_zone::kv_rt_zone;
pub use send::kv_rt_weak;
pub mod rt_text;
mod send;
pub mod shapes;
pub mod table;
pub mod text;
pub mod text_items;
pub mod unwind_tables;

/// Native code runs on a thread with a large stack for deep recursion.
pub const STACK_SIZE: usize = 512 * 1024 * 1024;

/// Sets what the runtime must know before a program runs: its file path
/// (for panic messages) and the layout of its heap types.
pub fn start(path: &str, shapes: &[kavach_kir::Shape]) {
    rt::set_path(path);
    heap::set_shapes(shapes);
}

/// Runs a program's compiled entry function and returns the exit code:
/// 0, or 1 when `main` returns an error (spec 5.5). A panic exits the
/// process directly with 101 (spec 5.8).
///
/// `fails` says whether the entry function returns an error pointer.
pub fn run_entry(entry: *const u8, fails: bool) -> i32 {
    rt::set_stack_limit(STACK_SIZE);
    rt_coroutines::make_host();
    let check_leaks = std::env::var_os("KAVACH_CHECK_LEAKS").is_some();
    rt::kv_free_at_exit.store(check_leaks as usize, std::sync::atomic::Ordering::Relaxed);
    let error = call_entry(entry, fails);
    rt::flush();
    // `main`'s error first: an end hook that panics ends the program,
    // and the error would be lost.
    let exit_code = if error != 0 {
        eprintln!("error: {}", display::show_error(error));
        heap::release(error);
        1
    } else {
        0
    };
    // `main` returned while a stop signal let it finish (decision 0088).
    let exit_code = match kavach_runtime_core::stop_holds::asked_code() {
        Some(code) if exit_code == 0 => code,
        _ => exit_code,
    };
    rt::run_at_exit();
    // With KAVACH_CHECK_LEAKS=1, report `Shared` values a cycle kept
    // alive (spec 6.2), else objects reference counting missed.
    let cycles = if check_leaks {
        send::leaked_shared()
    } else {
        Vec::new()
    };
    for (line, n) in &cycles {
        let (path, line) = rt::location(*line);
        eprintln!(
            "{}",
            kavach_runtime_core::leak_message(*n, &format!("{path}:{line}"))
        );
    }
    // Not counted: key objects that other threads keep to find keys fast
    // until the program ends (`rt_maps/known_keys.rs`).
    let live = || heap::live_objects().saturating_sub(rt_maps::known_keys::held());
    if !cycles.is_empty() {
        eprintln!("{}", kavach_runtime_core::LEAK_HINT);
    } else if check_leaks
        && live() != 0
        // What a panicking task's frames held is never freed (decision 0065).
        && !rt_task_errors::PANICKED.load(std::sync::atomic::Ordering::Relaxed)
    {
        eprintln!("kavach: {} heap objects were not freed", live());
    }
    exit_code
}

fn call_entry(entry: *const u8, fails: bool) -> i64 {
    if !fails {
        // SAFETY: the code generator compiled `entry` with the signature
        // `fn()`, and the code it points to lives as long as the program.
        let f: extern "C-unwind" fn() = unsafe { std::mem::transmute(entry) };
        f();
        return 0;
    }
    // SAFETY: as above, with the signature `fn() -> i64`: an entry that
    // can fail returns `Unit` and its error pointer.
    let f: extern "C-unwind" fn() -> i64 = unsafe { std::mem::transmute(entry) };
    f()
}

/// What runs before a program ends on Ctrl+C or `SIGTERM` (`rt_stop`):
/// its output flushed, then its end hooks, as at a normal end.
pub fn end_hooks() {
    rt::flush();
    rt::run_at_exit();
}

/// The start of an executable made by `kavach build`: its generated C
/// `main` calls this with the entry function, the program's file path and
/// its shape table (as written by `kavach_kir::encode_shapes`).
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_main(
    entry: *const u8,
    fails: i8,
    path: *const u8,
    path_len: usize,
    shapes: *const u8,
    shapes_len: usize,
    limit_slot: Option<extern "C-unwind" fn() -> *mut usize>,
) -> i32 {
    // Only LLVM-built programs give one (`rt::set_limit_slot`).
    if let Some(slot) = limit_slot {
        rt::set_limit_slot(slot);
    }
    rt_stop::catch_stops(end_hooks);
    rt_std::set_args(std::env::args().skip(1).collect());
    // Its first `env` reads the `.env` beside it, or the one
    // `KAVACH_ENV_FILE` names (spec 10.13).
    kavach_runtime_core::env_file::BUILT.store(true, std::sync::atomic::Ordering::Relaxed);
    let path = rt::text(path, path_len).to_string();
    let shapes = kavach_kir::decode_shapes(rt::text(shapes, shapes_len))
        .expect("the compiler wrote a valid shape table");
    start(&path, &shapes);
    // Raw pointers can't cross threads; the address can.
    let entry = entry as usize;
    let run = std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || run_entry(entry as *const u8, fails != 0));
    match run.map(|t| t.join()) {
        Ok(Ok(code)) => code,
        _ => {
            // What the program printed comes out first.
            let _ = std::io::Write::flush(&mut std::io::stdout());
            eprintln!("kavach: the program crashed");
            101
        }
    }
}
