//! Standard library modules in native code (spec 14.3–14.6): files,
//! clocks, math, the terminal and the command line. Messages must match
//! the interpreter (`compiler/interp/src/std_lib.rs`).

use crate::heap::{alloc_slots, new_str, release, shape, str_of};
use crate::rt::{flush, type_of};
use crate::rt_data::optional;
use crate::rt_errors::set_error;
use kavach_kir::{RtOp, Shape, Type};
use std::io::IsTerminal;
use std::sync::OnceLock;
use std::time::Instant;

static ARGS: OnceLock<Vec<String>> = OnceLock::new();

/// The program's arguments, for `cli.args` (set once, before it runs).
pub fn set_args(args: Vec<String>) {
    let _ = ARGS.set(args);
}

fn text(p: i64) -> String {
    let s = str_of(p);
    release(p);
    s
}

/// The first text type in the program: errors are text.
fn str_type() -> Type {
    crate::heap::first_of(&Shape::Str)
}

/// Leaves the error of a failed file operation for `kv_take_error`: an
/// `FsError` from `std.fs` (spec 14.3).
pub(crate) fn fail(_what: &str, path: &str, e: std::io::Error) -> i64 {
    let Some(ty) = crate::shapes::find_union("FsError", "NotFound") else {
        return 0;
    };
    let text = |s: &str| crate::shapes::new_text(s, str_type()) as u64;
    let error = match e.kind() {
        std::io::ErrorKind::NotFound => crate::rt_json::new_variant(ty, "NotFound", &[text(path)]),
        std::io::ErrorKind::PermissionDenied => {
            crate::rt_json::new_variant(ty, "Denied", &[text(path)])
        }
        _ => crate::rt_json::new_variant(ty, "Failed", &[text(path), text(&e.to_string())]),
    };
    set_error(error);
    0
}

/// Writes through a temporary file, then renames it (spec 14.3).
fn write_atomically(path: &str, text: impl AsRef<[u8]>) -> std::io::Result<()> {
    kavach_runtime_core::files::write_atomically(path, text.as_ref())
}

fn float(slot: i64) -> f64 {
    f64::from_bits(slot as u64)
}

fn bits(f: f64) -> i64 {
    f.to_bits() as i64
}

/// The item type of a list type `code`.
fn item_of(code: i64) -> Type {
    match shape(type_of(code)) {
        Shape::List(t) | Shape::Optional(t) => *t,
        _ => Type::I64,
    }
}

static START: OnceLock<Instant> = OnceLock::new();

/// Waits until monotonic time `ms` (or not at all if it has passed), and
/// returns the time then: a round of `time.every_ms` (spec 14.4).
pub fn wait_until(ms: i64, line: i32) -> i64 {
    let start = *START.get_or_init(Instant::now);
    let at = start + std::time::Duration::from_millis(ms.max(0) as u64);
    // A `sleep` until then, so in a durable handler an `every` round
    // carries on across a restart.
    sleep_until(at, line);
    start.elapsed().as_millis() as i64
}

/// Waits until `at`. In a durable handler the wait's end is journaled, so
/// after a restart only what is left is waited for, never more (spec 8.5,
/// decision 0062).
fn sleep_until(at: Instant, line: i32) {
    let now = Instant::now();
    // Whole milliseconds, rounded up, so a wait never ends early.
    let ms = at
        .saturating_duration_since(now)
        .as_micros()
        .div_ceil(1_000) as i64;
    let left = journaled_wait("time.sleep", ms, line);
    let until = match left == ms as u64 {
        true => at,
        false => now + std::time::Duration::from_millis(left),
    };
    crate::rt_tasks::sleep_until(until, line);
}

/// How long of a wait of `ms` is left: all of it, or in a durable handler
/// what is left of the one the journal has for `call`, never more.
pub(crate) fn journaled_wait(call: &str, ms: i64, line: i32) -> u64 {
    let (ms, now) = (ms.max(0), wall_ms());
    let due = journaled(call, now.saturating_add(ms), line);
    due.saturating_sub(now).clamp(0, ms) as u64
}

/// Milliseconds since 1970-01-01 UTC, by the wall clock.
fn wall_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// Set by the journal when the first durable agent starts
/// (`rt_journal.rs`), so programs without one don't link it (decision
/// 0024): what a durable handler's clock read or wait is.
pub(crate) static JOURNALED_TIME: std::sync::OnceLock<fn(&str, i64, i32) -> i64> =
    std::sync::OnceLock::new();

/// `now`, or in a durable handler the moment its journal has for `call`.
fn journaled(call: &str, now: i64, line: i32) -> i64 {
    JOURNALED_TIME.get().map_or(now, |f| f(call, now, line))
}

pub fn std_op(op: RtOp, a: i64, b: i64, code: i64, line: i32) -> i64 {
    // Files, the terminal and leaving are effects other tasks could see.
    let effect = !matches!(
        op,
        RtOp::TimeNowMs
            | RtOp::TimeMonotonicMs
            | RtOp::TimeSleepMs
            | RtOp::MathSqrt
            | RtOp::MathPow
            | RtOp::MathFloor
            | RtOp::MathCeil
            | RtOp::MathRound
            | RtOp::CliArgs
            | RtOp::IoIsTerminal
            | RtOp::IoColors
    );
    if effect {
        crate::rt_tasks::turn();
    }
    // Outside the folders of `uses Fs("…")` (spec 8.1).
    let file_op = matches!(
        op,
        RtOp::FsReadText
            | RtOp::FsWriteText
            | RtOp::FsAppendText
            | RtOp::FsExists
            | RtOp::FsList
            | RtOp::FsRemove
            | RtOp::FsCreateDir
            | RtOp::FsReadBytes
            | RtOp::FsWriteBytes
            | RtOp::FsWriteDurable
    );
    if file_op {
        let path = str_of(a);
        if !crate::rt_files::allowed(&path) {
            release(a);
            if matches!(
                op,
                RtOp::FsWriteText | RtOp::FsAppendText | RtOp::FsWriteBytes | RtOp::FsWriteDurable
            ) {
                release(b);
            }
            if op == RtOp::FsExists {
                return 0;
            }
            return crate::rt_files::denied(&path);
        }
    }
    match op {
        RtOp::FsReadText => {
            let path = text(a);
            match std::fs::read_to_string(&path) {
                Ok(s) => new_str(s, type_of(code)),
                Err(e) => fail("read", &path, e),
            }
        }
        RtOp::FsReadBytes => {
            let path = text(a);
            match std::fs::read(&path) {
                Ok(b) => crate::rt_bytes::new_bytes(b, type_of(code)),
                Err(e) => fail("read", &path, e),
            }
        }
        RtOp::FsWriteBytes | RtOp::FsWriteDurable => {
            let path = text(a);
            let body = crate::rt_bytes::with_bytes(b, <[u8]>::to_vec);
            crate::heap::release(b);
            let written = if op == RtOp::FsWriteDurable {
                kavach_runtime_core::files::write_durably(&path, &body)
            } else {
                write_atomically(&path, &body)
            };
            match written {
                Ok(()) => 0,
                Err(e) => fail("write", &path, e),
            }
        }
        RtOp::FsWriteText | RtOp::FsAppendText => {
            let (path, body) = (text(a), text(b));
            let done = if op == RtOp::FsWriteText {
                write_atomically(&path, &body).map_err(|e| ("write", e))
            } else {
                use std::io::Write;
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .map_err(|e| ("open", e))
                    .and_then(|mut f| f.write_all(body.as_bytes()).map_err(|e| ("write", e)))
            };
            match done {
                Ok(()) => 0,
                Err((what, e)) => fail(what, &path, e),
            }
        }
        RtOp::FsExists => std::path::Path::new(&text(a)).exists() as i64,
        RtOp::FsList => {
            let path = text(a);
            match std::fs::read_dir(&path) {
                Ok(entries) => {
                    let mut names: Vec<String> = entries
                        .flatten()
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect();
                    names.sort();
                    let item = item_of(code);
                    let slots = names.into_iter().map(|n| new_str(n, item) as u64).collect();
                    alloc_slots(type_of(code), 0, slots)
                }
                Err(e) => fail("list", &path, e),
            }
        }
        RtOp::FsRemove => {
            let path = text(a);
            let p = std::path::Path::new(&path);
            let done = if p.is_dir() {
                std::fs::remove_dir(p)
            } else {
                std::fs::remove_file(p)
            };
            done.map_or_else(|e| fail("remove", &path, e), |_| 0)
        }
        RtOp::FsCreateDir => {
            let path = text(a);
            std::fs::create_dir_all(&path).map_or_else(|e| fail("create", &path, e), |_| 0)
        }
        // In a durable handler, the time its journal has (spec 8.5).
        RtOp::TimeNowMs => journaled("time.now_ms", wall_ms(), line),
        RtOp::TimeMonotonicMs => START.get_or_init(Instant::now).elapsed().as_millis() as i64,
        RtOp::TimeSleepMs => {
            sleep_until(
                Instant::now() + std::time::Duration::from_millis(a.max(0) as u64),
                line,
            );
            0
        }
        RtOp::MathSqrt => bits(float(a).sqrt()),
        RtOp::MathPow => bits(float(a).powf(float(b))),
        RtOp::MathFloor => bits(float(a).floor()),
        RtOp::MathCeil => bits(float(a).ceil()),
        RtOp::MathRound => bits(float(a).round()),
        RtOp::IoWrite => {
            let text = str_of(a);
            release(a);
            crate::rt::out(&text);
            let _ = std::io::Write::flush(&mut std::io::stdout());
            0
        }
        RtOp::IoReadHidden => {
            let _ = std::io::Write::flush(&mut std::io::stdout());
            let line = kavach_runtime_core::input::read_hidden();
            optional(line.map(|l| new_str(l, item_of(code))), code)
        }
        RtOp::IoReadLine => {
            // What was printed (a question) shows before the program waits.
            let _ = std::io::Write::flush(&mut std::io::stdout());
            let mut line = String::new();
            match std::io::stdin().read_line(&mut line) {
                Ok(0) | Err(_) => 0,
                Ok(_) => optional(
                    Some(new_str(
                        line.trim_end_matches(['\n', '\r']).to_string(),
                        item_of(code),
                    )),
                    code,
                ),
            }
        }
        RtOp::IoIsTerminal => std::io::stdout().is_terminal() as i64,
        RtOp::IoColors => kavach_runtime_core::log::colors() as i64,
        RtOp::CliArgs => {
            let item = item_of(code);
            let args = ARGS.get().cloned().unwrap_or_default();
            let slots = args.into_iter().map(|s| new_str(s, item) as u64).collect();
            alloc_slots(type_of(code), 0, slots)
        }
        // `a` is the name (owned); the value is a secret (spec 10.13).
        RtOp::EnvGet => {
            if let Err(e) = kavach_runtime_core::env_file::load_once() {
                flush();
                eprintln!("error: {e}");
                std::process::exit(2);
            }
            let name = text(a);
            match kavach_runtime_core::env_file::get(&name) {
                Some(v) => new_str(v, type_of(code)),
                None => {
                    let message = format!("environment variable `{name}` is not set");
                    set_error(crate::shapes::new_text(&message, str_type()));
                    0
                }
            }
        }
        RtOp::CliParse => {
            crate::rt_cli::cli_parse(a, b, code, ARGS.get().map_or(&[], Vec::as_slice))
        }
        _ => {
            // CliExit: ends the program at once (spec 14.6).
            flush();
            crate::rt::run_at_exit();
            std::process::exit(a as i32)
        }
    }
}

// Apart from `kv_rt`, which every other op of this file goes through, so
// a program links only this of it (decision 0024).
crate::rt_data::entry!(kv_rt_version, |_op, a, b, c, _code, _line| {
    if let Some(why) = other_operations(c) {
        eprintln!("error: {why}");
        std::process::exit(2)
    }
    program_version(a, b)
});

/// Why a program compiled for the operations whose fingerprint is
/// `compiled` can't run with this runtime library, if it can't: the two
/// come from different Kavach versions (spec 10.5).
fn other_operations(compiled: i64) -> Option<String> {
    (compiled != kavach_kir::RtOp::FINGERPRINT).then(|| {
        "this program was built by a Kavach compiler that doesn't match the runtime library it was linked with, so it would call the wrong operations; build both from the same Kavach (`cargo build -p kavach-cli -p kavach-runtime-native`), or unset KAVACH_RUNTIME_LIB (spec 10.5)".to_string()
    })
}

/// What built the program, between its marks (spec 10.5): with `[build]
/// version_flag` (`answers`), the one argument `--version` prints it and
/// ends the program before `main` runs.
pub fn program_version(marked: i64, answers: i64) -> i64 {
    let marked = text(marked);
    let asked = ARGS
        .get()
        .is_some_and(|x| x.len() == 1 && x[0] == "--version");
    if answers != 0 && asked {
        let inner = marked.split_once('>').map_or("", |(_, rest)| rest);
        let inner = inner.rsplit_once('<').map_or(inner, |(t, _)| t);
        println!("{inner}");
        flush();
        std::process::exit(0)
    }
    0
}

#[cfg(test)]
mod tests {
    use super::other_operations;

    #[test]
    fn a_program_for_other_operations_is_stopped_with_a_reason() {
        assert_eq!(other_operations(kavach_kir::RtOp::FINGERPRINT), None);
        let why = other_operations(kavach_kir::RtOp::FINGERPRINT ^ 1).unwrap();
        assert!(why.contains("doesn't match the runtime library"), "{why}");
    }
}
