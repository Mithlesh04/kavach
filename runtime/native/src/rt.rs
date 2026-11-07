//! Run-time functions that native code calls: printing, panics and the
//! stack limit. Their output must match the interpreter exactly (decision 0014):
//! the conformance cross-check compares the two.

use kavach_kir::Type;
use std::io::Write;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The program's files: (first line in the program, path), by first line.
static FILES: OnceLock<Vec<(i32, String)>> = OnceLock::new();

/// The lowest stack address generated code may reach: below it, a call
/// is a stack overflow, a panic (spec 3.8). Each function compares the
/// stack pointer with it on entry and calls `kv_stack_overflow` below it.
#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static kv_stack_limit: AtomicUsize = AtomicUsize::new(0);

/// 1 when `main` must free its objects before it returns: only when
/// leaks are checked (`KAVACH_CHECK_LEAKS`). Otherwise the operating
/// system takes back all memory at once when the program ends, so
/// freeing millions of objects one by one would be wasted work.
#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static kv_free_at_exit: AtomicUsize = AtomicUsize::new(0);

/// In a program built by `kavach build`, tasks run on several threads at
/// once (decision 0020), so each thread has its own stack limit: a
/// thread-local the generated module defines, whose address this
/// function gives, for the thread that calls it. The generated `main`
/// passes it to `kv_main`. Without it (`kavach run --native`), tasks take
/// turns and the global above is enough.
static LIMIT_SLOT: OnceLock<extern "C-unwind" fn() -> *mut usize> = OnceLock::new();

/// Whether tasks may run on several threads at once (`LIMIT_SLOT`).
static PARALLEL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_limit_slot(slot: extern "C-unwind" fn() -> *mut usize) {
    let _ = LIMIT_SLOT.set(slot);
    PARALLEL.store(true, Ordering::Relaxed);
}

/// Whether tasks may run on several threads at once: then the pool and
/// the stack limit are per thread.
pub fn parallel() -> bool {
    PARALLEL.load(Ordering::Relaxed)
}

/// Room left below the limit for the runtime's own calls.
const STACK_RESERVE: usize = 8 * 1024 * 1024;

/// Sets the stack limit for the current thread, which has a stack of
/// `size` bytes and is about to run generated code.
pub fn set_stack_limit(size: usize) {
    set_stack_use(size - STACK_RESERVE);
}

/// Puts back a task's stack limit, kept while another task ran on this
/// thread (`rt_coroutines.rs`). `shared`: also the one limit all threads
/// read when tasks take turns (`kavach run --native`), which belongs to
/// the task whose turn it is, maybe on another thread.
pub fn restore_stack_limit(limit: usize, shared: bool) {
    if shared {
        kv_stack_limit.store(limit, Ordering::Relaxed);
    }
    if let Some(slot) = LIMIT_SLOT.get() {
        // SAFETY: as in `set_stack_use`.
        #[allow(unsafe_code)]
        unsafe {
            *slot() = limit;
        }
    }
}

/// Sets the stack limit for the current thread: generated code may use
/// `usable` bytes below here, and the rest of the thread's stack is left
/// for the runtime.
pub fn set_stack_use(usable: usize) {
    let here = 0u8;
    let top = std::ptr::addr_of!(here) as usize;
    let limit = top.saturating_sub(usable);
    kv_stack_limit.store(limit, Ordering::Relaxed);
    if let Some(slot) = LIMIT_SLOT.get() {
        // SAFETY: the generated module's function gives the address of
        // this thread's own limit, a thread-local `i64` that lives as long
        // as the thread.
        #[allow(unsafe_code)]
        unsafe {
            *slot() = limit;
        }
    }
    // Each task's thread has its own stack (`rt_tasks.rs`).
    crate::rt_tasks::LIMIT.set(limit);
}

/// Sets the file table: `first line<TAB>path` per file (KIR's
/// `Program::path`), or a plain path for a single file.
pub fn set_path(table: &str) {
    let files = table
        .lines()
        .map(|l| match l.split_once('\t') {
            Some((n, p)) => (n.parse().unwrap_or(1), p.to_string()),
            None => (1, l.to_string()),
        })
        .collect();
    let _ = FILES.set(files);
}

/// The file and the line in it of line `line` of the program.
pub(crate) fn location(line: i32) -> (&'static str, i32) {
    let files = FILES.get().map_or(&[][..], Vec::as_slice);
    match files.iter().rev().find(|(first, _)| *first <= line) {
        Some((first, path)) => (path, line - first + 1),
        None => ("?", line),
    }
}

/// The type codes are the compiler's (`kavach_kir::TYPE_CODES`), so the
/// two sides can never drift apart.
pub use kavach_kir::type_code;

pub fn type_of(code: i64) -> Type {
    if code >= 100 {
        return Type::Ref((code - 100) as u32);
    }
    kavach_kir::TYPE_CODES
        .get(code as usize)
        .copied()
        .unwrap_or(Type::I64)
}

pub fn out(text: &str) {
    crate::rt_tasks::turn();
    let _ = std::io::stdout().lock().write_all(text.as_bytes());
}

thread_local! {
    /// The line the running task's `print` is making, with tasks on all
    /// cores: it is written whole at its end, so tasks on other cores
    /// never print into the middle of it (decision 0049). Moved with the
    /// task when coroutines switch (`rt_coroutines.rs`).
    static LINE: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Puts `s` in place of this thread's line being printed, giving that one.
pub(crate) fn swap_line(s: String) -> String {
    LINE.replace(s)
}

/// A piece of what `print` prints: on one core written at once, as the
/// turn orders it; on all cores kept for the line's end.
pub(crate) fn print_piece(text: &str) {
    piece(text)
}

fn piece(text: &str) {
    if kavach_runtime_core::tasks::many_workers() {
        crate::rt_tasks::turn();
        LINE.with(|l| l.borrow_mut().push_str(text));
    } else {
        out(text);
    }
}

/// What runs when the program ends, however it ends: a feature that
/// must finish something (sending the last log events) adds itself.
static AT_EXIT: std::sync::Mutex<Vec<fn()>> = std::sync::Mutex::new(Vec::new());

/// Runs `f` when the program ends.
pub fn at_exit(f: fn()) {
    AT_EXIT.lock().unwrap_or_else(|e| e.into_inner()).push(f);
}

/// Runs what `at_exit` gathered, once, in order. Each is taken before
/// it runs, so one that panics leaves the rest to the panic's own run.
pub fn run_at_exit() {
    loop {
        let next = {
            let mut hooks = AT_EXIT.lock().unwrap_or_else(|e| e.into_inner());
            (!hooks.is_empty()).then(|| hooks.remove(0))
        };
        match next {
            Some(f) => f(),
            None => break,
        }
    }
}

/// A panic unwinding to the start of its task (decision 0065): the error
/// `.wait()` on the task gives.
pub(crate) struct TaskPanic(pub(crate) String);

/// Prints `panic: …`. In a spawned task, the panic stops only that task
/// (spec 9.1): it unwinds to the task's start, which gives `.wait()` the
/// error `panicked: … at …`. Anywhere else it ends the program with exit
/// code 101 (spec 5.8).
pub fn panic_with(message: &str, line: i32) -> ! {
    crate::rt_tasks::turn();
    let _ = std::io::stdout().flush();
    let (path, line) = location(line);
    if crate::rt_task_errors::in_spawned_task() {
        eprintln!("panic: {message}\n  at {path}:{line}");
        let why = format!("panicked: {message} at {path}:{line}");
        std::panic::resume_unwind(Box::new(TaskPanic(why)));
    }
    run_at_exit();
    eprintln!("panic: {message}\n  at {path}:{line}");
    // A child of `spawn process` (spec 9.8): its parent's task fails with
    // the panic, as any task's does.
    if kavach_runtime_core::processes::site().is_some() {
        let why = format!("panicked: {message} at {path}:{line}");
        let outcome = kavach_runtime_core::json::Tree::Object(vec![(
            "panicked".into(),
            kavach_runtime_core::json::Tree::Text(why),
        )]);
        let json = kavach_runtime_core::json::write(&outcome, 0);
        let _ = std::io::stdout()
            .write_all(kavach_runtime_core::processes::outcome_line(&json).as_bytes());
        let _ = std::io::stdout().flush();
    }
    std::process::exit(101)
}

/// Shows `bits` as a value of integer type `t`.
pub fn show_int(bits: i64, t: Type) -> String {
    if t.is_signed() {
        bits.to_string()
    } else {
        (bits as u64).to_string()
    }
}

pub use kavach_runtime_core::format_float;

/// Text from native code: a pointer to UTF-8 bytes and their length.
#[allow(unsafe_code)]
pub(crate) fn text<'a>(ptr: *const u8, len: usize) -> &'a str {
    // SAFETY: native code only passes pointers to string constants that the
    // code generator leaked for the life of the program, with their length;
    // it copies them from Rust strings, so they are valid UTF-8.
    unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) }
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_print_str(ptr: *const u8, len: usize) {
    piece(text(ptr, len));
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_print_i64(v: i64) {
    piece(&v.to_string());
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_print_u64(v: u64) {
    piece(&v.to_string());
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_print_f64(v: f64) {
    piece(&format_float(v));
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_print_bool(v: i8) {
    piece(if v != 0 { "true" } else { "false" });
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_print_end() {
    if kavach_runtime_core::tasks::many_workers() {
        let mut line = LINE.take();
        line.push('\n');
        out(&line);
    } else {
        out("\n");
    }
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_panic(ptr: *const u8, len: usize, line: i32) {
    panic_with(text(ptr, len), line)
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_overflow(op: i32, a: i64, b: i64, ty: i32, line: i32) {
    let t = type_of(ty as i64);
    // Shifts come as their first character.
    let op = match char::from_u32(op as u32).unwrap_or('?') {
        '<' => "<<".to_string(),
        '>' => ">>".to_string(),
        c => c.to_string(),
    };
    panic_with(
        &format!(
            "integer overflow: {} {op} {} doesn't fit in {}",
            show_int(a, t),
            show_int(b, t),
            t.kavach_name()
        ),
        line,
    )
}

/// `a ** b` in whole numbers of type `t` (spec 1.8), panicking as `*`
/// does when the result doesn't fit.
pub(crate) fn int_pow(a: i64, b: i64, t: Type, line: i32) -> i64 {
    let wide = |v: i64| {
        if t.is_signed() {
            v as i128
        } else {
            v as u64 as i128
        }
    };
    let (x, y) = (wide(a), wide(b));
    if y < 0 {
        panic_with(
            &format!("{x} ** {y}: a whole number to a negative power isn't whole; use floats"),
            line,
        )
    }
    let bits = t.bits();
    let (lo, hi) = if t.is_signed() {
        (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
    } else {
        (0, (1i128 << bits) - 1)
    };
    match kavach_runtime_core::int_pow(x, y) {
        Some(n) if (lo..=hi).contains(&n) => n as i64,
        _ => panic_with(
            &format!(
                "integer overflow: {x} ** {y} doesn't fit in {}",
                t.kavach_name()
            ),
            line,
        ),
    }
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_neg_overflow(a: i64, ty: i32, line: i32) {
    let t = type_of(ty as i64);
    panic_with(
        &format!(
            "integer overflow: -({}) doesn't fit in {}",
            show_int(a, t),
            t.kavach_name()
        ),
        line,
    )
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_div_zero(line: i32) {
    panic_with("division by zero", line)
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_fmod(a: f64, b: f64) -> f64 {
    a % b
}

/// Checks that integer `bits` (of type `from`) fits in type `to`.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_check_int(bits: i64, from: i32, to: i32, line: i32) -> i64 {
    let (from, to) = (type_of(from as i64), type_of(to as i64));
    let value: i128 = if from.is_signed() {
        bits as i128
    } else {
        bits as u64 as i128
    };
    let (lo, hi) = range(to);
    if value < lo || value > hi {
        panic_with(
            &format!(
                "{} doesn't fit in {}",
                show_int(bits, from),
                to.kavach_name()
            ),
            line,
        )
    }
    value as i64
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_float_to_int(f: f64, to: i32, line: i32) -> i64 {
    let to = type_of(to as i64);
    let (lo, hi) = range(to);
    let t = f.trunc();
    if !t.is_finite() || t < lo as f64 || t > hi as f64 {
        panic_with(
            &format!("{} doesn't fit in {}", format_float(f), to.kavach_name()),
            line,
        )
    }
    t as i128 as i64
}

pub fn range(t: Type) -> (i128, i128) {
    match t {
        Type::I8 => (i8::MIN as i128, i8::MAX as i128),
        Type::I16 => (i16::MIN as i128, i16::MAX as i128),
        Type::I32 => (i32::MIN as i128, i32::MAX as i128),
        Type::U8 => (0, u8::MAX as i128),
        Type::U16 => (0, u16::MAX as i128),
        Type::U32 => (0, u32::MAX as i128),
        Type::U64 => (0, u64::MAX as i128),
        _ => (i64::MIN as i128, i64::MAX as i128),
    }
}

/// A call went below the stack limit.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_stack_overflow(line: i32) {
    panic_with(
        "stack overflow: calls are nested too deeply (spec 3.8)",
        line,
    )
}

pub fn flush() {
    let _ = std::io::stdout().flush();
}
