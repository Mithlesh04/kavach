// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Values shared by every way of running Kavach, so the interpreter and
//! native code behave exactly alike: exact decimals (spec 2.1), how
//! floats print (spec 1.11), JSON text (spec 2.12) and the bookkeeping of
//! agents' guards (spec 8.3).

pub mod agents;
pub mod app;
pub mod base64;
mod big_decimal;
pub mod chrome_trace;
pub mod clipboard;
pub mod compress;
mod decimal;
pub mod env_file;
pub mod file_info;
pub mod files;
pub mod fine_clock;
pub mod fs_scope;
pub mod inbox;
pub mod input;
pub mod journal;
pub mod json;
pub mod log;
pub mod math;
pub mod normalize;
pub mod processes;
pub mod profiler;
pub mod regex;
pub mod seeded;
pub mod serial;
pub mod stop_holds;
pub mod tasks;
pub mod text_methods;
pub mod trace;
pub mod writers;
pub mod zones;

pub use big_decimal::{AnyDecimal, BigDecimal};
pub use decimal::Decimal;
/// Whole numbers of any size (spec 2.1, decision 0041).
pub use num_bigint::BigInt;

/// The longest text `repeat` makes: 1 GB.
pub const MAX_REPEAT: usize = 1 << 30;

/// `s.repeat(n)` (spec 14.2), or the panic message when the text would be
/// longer than `MAX_REPEAT` or the memory can't be had (spec 5.6): asking
/// for that much text is a bug in the program, not a crash of Kavach.
pub fn repeat(s: &str, n: usize) -> Result<String, String> {
    let too_big = || {
        format!(
            "out of memory: `repeat` asked for {n} copies of {} bytes",
            s.len()
        )
    };
    let total = s
        .len()
        .checked_mul(n)
        .filter(|t| *t <= MAX_REPEAT)
        .ok_or_else(too_big)?;
    // Empty text repeated is empty, however many times: no loop of `n`
    // rounds (a trillion took forever).
    if s.is_empty() {
        return Ok(String::new());
    }
    let mut out = String::new();
    out.try_reserve_exact(total).map_err(|_| too_big())?;
    for _ in 0..n {
        out.push_str(s);
    }
    Ok(out)
}

/// Shortest form that reads back exactly, always with a `.` or an exponent
/// (spec 1.11): `7.0`, `0.1`, `1e21`.
pub fn format_float(f: f64) -> String {
    let mut out = String::new();
    push_float(&mut out, f);
    out
}

/// Appends `format_float(f)` to `out`.
pub fn push_float(out: &mut String, f: f64) {
    use std::fmt::Write;
    let a = f.abs();
    let start = out.len();
    let written = if f.is_finite() && a != 0.0 && !(1e-5..1e16).contains(&a) {
        write!(out, "{f:e}")
    } else {
        write!(out, "{f}")
    };
    written.expect("writing to a String never fails");
    if f.is_finite() && (a == 0.0 || (1e-5..1e16).contains(&a)) && !out[start..].contains('.') {
        out.push_str(".0");
    }
}

/// `x ** y` for whole numbers, `y` not negative (spec 1.8): `None` when
/// the result needs more than 128 bits. Bases 0, 1 and -1 give their
/// value for any `y`, however large.
pub fn int_pow(x: i128, y: i128) -> Option<i128> {
    match x {
        0 => Some(i128::from(y == 0)),
        1 => Some(1),
        -1 => Some(if y % 2 == 0 { 1 } else { -1 }),
        _ => x.checked_pow(u32::try_from(y).ok()?),
    }
}

/// `a ** b` for `BigInt`s (spec 1.8), or the panic message: a negative
/// power, or one too big to work out.
pub fn big_pow(a: &BigInt, b: &BigInt) -> Result<BigInt, String> {
    if b.sign() == num_bigint::Sign::Minus {
        return Err(format!(
            "{a} ** {b}: a whole number to a negative power isn't whole; use floats"
        ));
    }
    let one = BigInt::from(1);
    if *a == one || a.sign() == num_bigint::Sign::NoSign {
        return Ok(if b.sign() == num_bigint::Sign::NoSign {
            one
        } else {
            a.clone()
        });
    }
    match u32::try_from(b) {
        Ok(e) => Ok(a.pow(e)),
        Err(_) if *a == -&one => Ok(if b.bit(0) { a.clone() } else { one }),
        Err(_) => Err(format!("{a} ** {b}: the power is too big to work out")),
    }
}

/// `a.near(b)` (spec 14.2): whether two floats are equal but for
/// rounding: the same, within one billionth of the larger, or both within
/// 1e-12 of each other near zero. `NaN` is near nothing.
pub fn float_near(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    // An infinity is near only itself: the tolerance of the larger would
    // be infinite too (14.2).
    if a.is_infinite() || b.is_infinite() {
        return false;
    }
    let gap = (a - b).abs();
    gap <= 1e-12 || gap <= 1e-9 * a.abs().max(b.abs())
}

/// A `Float32` as Kavach prints it: the shortest text that reads back as
/// the same `Float32`, in the style of [`format_float`].
pub fn format_float32(f: f32) -> String {
    if !f.is_finite() {
        return f.to_string();
    }
    let a = f.abs();
    if a != 0.0 && !(1e-5..1e16).contains(&a) {
        return format!("{f:e}");
    }
    let s = f.to_string();
    if s.contains('.') { s } else { format!("{s}.0") }
}

/// `a.near(b)` for `Float32`s (spec 14.2): as [`float_near`], with room
/// for single precision: within one hundred-thousandth of the larger, or
/// within 1e-6 near zero.
pub fn float32_near(a: f32, b: f32) -> bool {
    if a == b {
        return true;
    }
    if a.is_infinite() || b.is_infinite() {
        return false;
    }
    let gap = (a - b).abs();
    gap <= 1e-6 || gap <= 1e-5 * a.abs().max(b.abs())
}

/// The line for `n` `Shared` values made at `place` (`main.kav:9`) that
/// a cycle kept alive to the end of the program (spec 6.2).
pub fn leak_message(n: usize, place: &str) -> String {
    match n {
        1 => format!("leak: a `Shared` value made at {place} was never freed: a cycle keeps it"),
        n => format!(
            "leak: {n} `Shared` values made at {place} were never freed: a cycle keeps them"
        ),
    }
}

/// Printed after the leaks, once.
pub const LEAK_HINT: &str = "  make the link back a `Weak`, so no cycle forms (spec 6.2)";

/// The most entries `capacity(n)` may ask for (spec 2.2): a limit is kept
/// in 32 bits.
pub const MAX_CAPACITY: i64 = 4_294_967_294;

/// The panic of adding to a full `what` ("list", "map", "set") made by
/// `capacity(n)` (spec 2.2).
pub fn full_panic(what: &str, n: usize) -> String {
    format!("the {what} is full (capacity {n})")
}

/// The panic of `capacity(n)` given more first entries than `n`, or a
/// number above `MAX_CAPACITY` (spec 2.2).
pub fn capacity_panic(kind: &str, n: i64, first: usize) -> String {
    if n > MAX_CAPACITY {
        return format!("capacity {n} is more than {MAX_CAPACITY}");
    }
    format!("`{kind}.capacity({n})` was given {first} first entries")
}

/// The panic of index `i` past the end of a list of `n` items.
pub fn list_index_panic(i: i128, n: usize) -> String {
    let items = if n == 1 { "item" } else { "items" };
    format!("index {i} is out of range: the list has {n} {items}")
}

/// The panic of index `i` past the end of `n` bytes.
pub fn bytes_index_panic(i: i128, n: usize) -> String {
    if n == 1 {
        return format!("index {i} is out of range: there is 1 byte");
    }
    format!("index {i} is out of range: there are {n} bytes")
}

/// `text` between `quote`s, as a string or character inside a collection
/// prints (spec 1.11): as written, combining marks too, with only the
/// quote, `\`, line breaks, tabs and characters that are invisible or
/// control the terminal escaped.
pub fn quoted(text: &str, quote: char) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push(quote);
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() || invisible(c) => {
                out.push_str(&format!("\\u{{{:x}}}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// Whether `c` is a format character that shows nothing: zero-width
/// spaces and joiners, direction marks, the byte order mark, soft hyphen.
fn invisible(c: char) -> bool {
    matches!(
        c as u32,
        0xAD | 0x61C | 0x180E | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064
            | 0x2066..=0x206F | 0xFEFF | 0xFFF9..=0xFFFB | 0xE0001 | 0xE0020..=0xE007F
    )
}

/// How many cores this program may use now (spec 4.9, decision 0126):
/// `program_cores`, capped by an `on cores(n)` around the value being
/// worked out.
pub fn cores() -> usize {
    let all = program_cores();
    match place().0 {
        0 => all,
        n => n.min(all),
    }
}

/// The panic of `on cores(n) x` with a count under 1 (spec 4.9).
pub fn place_count_panic(n: i64) -> String {
    format!("work runs on at least one core, but the count is {n} (spec 4.9)")
}

/// Where the value being worked out runs (`on cores(n) x`, `on cpu x`,
/// spec 4.9): its most cores (0: no cap) and whether its GPU maps run on
/// the CPU. The native runtime keeps it per task and sets this hook.
static PLACE: std::sync::OnceLock<fn() -> (usize, bool)> = std::sync::OnceLock::new();

/// Sets the hook `cores` and `gpu_mode` ask for the place.
pub fn set_place_hook(f: fn() -> (usize, bool)) {
    let _ = PLACE.set(f);
}

fn place() -> (usize, bool) {
    PLACE.get().map_or((0, false), |f| f())
}

/// How many cores this program may use (spec 4.9, decision 0126): the
/// machine's, as a container or CPU affinity allows, capped by
/// `KAVACH_CORES` (`[run] cores` in `kavach.toml`); `KAVACH_SERIAL` is
/// `KAVACH_CORES=1`. At least 1.
pub fn program_cores() -> usize {
    static CORES: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *CORES.get_or_init(|| {
        let machine = std::thread::available_parallelism().map_or(1, |n| n.get());
        let asked = std::env::var("KAVACH_CORES")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok());
        let asked = asked.or_else(|| std::env::var_os("KAVACH_SERIAL").map(|_| 1));
        let file = FILE_CORES.load(std::sync::atomic::Ordering::Relaxed);
        let asked = asked.or((file > 0).then_some(file));
        if let Some(note) = asked.and_then(|n| cap_note(n, machine))
            && std::env::var_os("KAVACH_PAR_LOG").is_some()
        {
            eprintln!("{note}");
        }
        asked.map_or(machine, |n| n.clamp(1, machine))
    })
}

/// What `KAVACH_PAR_LOG` prints when more cores are asked for than the
/// machine has (decision 0126), or `None`.
pub fn cap_note(asked: usize, machine: usize) -> Option<String> {
    (asked > machine).then(|| {
        format!("kavach: {asked} cores asked for, and this machine has {machine}: using {machine}")
    })
}

/// `[run] cores` in `kavach.toml` (spec 4.9), 0 when not set: the
/// program's start gives it, before anything asks `cores()`.
static FILE_CORES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// `[run] gpu = "cpu"` in `kavach.toml` (spec 15.2), set at the start.
static FILE_GPU_CPU: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Sets `[run] gpu = "cpu"`; `KAVACH_GPU` at launch still wins.
pub fn set_file_gpu_cpu() {
    FILE_GPU_CPU.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Where a `map` with a `gpu fn` runs (spec 15.2): `KAVACH_GPU` (`cpu`
/// is `off`), else `off` with `[run] gpu = "cpu"`, else `""`, the usual.
pub fn gpu_mode() -> String {
    // `on cpu x` (spec 4.9) keeps `x`'s maps on the CPU.
    if place().1 {
        return "off".into();
    }
    match std::env::var("KAVACH_GPU") {
        Ok(m) if m == "cpu" => "off".into(),
        Ok(m) => m,
        Err(_) if FILE_GPU_CPU.load(std::sync::atomic::Ordering::Relaxed) => "off".into(),
        Err(_) => String::new(),
    }
}

/// The `on threads` rounds running now in all of a program's loops, and
/// the most `[run] threads` lets run at once (spec 4.9), 256 when not
/// set. A native program has one (`PROGRAM_ROUNDS`); the interpreter one
/// a run, as tests run many programs in one process.
pub struct RoundCount {
    most: std::sync::atomic::AtomicUsize,
    running: std::sync::atomic::AtomicUsize,
}

impl Default for RoundCount {
    fn default() -> Self {
        RoundCount::new()
    }
}

impl RoundCount {
    pub const fn new() -> Self {
        RoundCount {
            most: std::sync::atomic::AtomicUsize::new(256),
            running: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Sets `[run] threads`.
    pub fn set_most(&self, n: usize) {
        self.most
            .store(n.max(1), std::sync::atomic::Ordering::Relaxed);
    }

    /// Whether as many rounds run as `[run] threads` allows.
    pub fn full(&self) -> bool {
        use std::sync::atomic::Ordering::Relaxed;
        self.running.load(Relaxed) >= self.most.load(Relaxed)
    }

    /// A round started (`1`) or was waited for (`-1`).
    pub fn add(&self, delta: i64) {
        use std::sync::atomic::Ordering::Relaxed;
        match delta {
            1 => self.running.fetch_add(1, Relaxed),
            _ => self.running.fetch_sub(1, Relaxed),
        };
    }
}

/// A native program's rounds.
pub static PROGRAM_ROUNDS: RoundCount = RoundCount::new();

/// Sets `[run] cores`; `KAVACH_CORES` at launch still wins.
pub fn set_file_cores(n: usize) {
    FILE_CORES.store(n, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    /// More cores asked for than the machine has gives a note; as many
    /// or fewer, none (decision 0126).
    #[test]
    fn a_cap_above_the_machine_is_noted() {
        assert_eq!(
            super::cap_note(11, 10).as_deref(),
            Some("kavach: 11 cores asked for, and this machine has 10: using 10")
        );
        assert_eq!(super::cap_note(10, 10), None);
        assert_eq!(super::cap_note(1, 10), None);
    }
}
