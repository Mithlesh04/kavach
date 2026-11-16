//! A live profiler, when one is on (spec 14.12, decision 0123): spans and
//! the GPU's timed work go to it as they happen. The profiler's crate
//! (`runtime/tracy`) sets the hooks, so only a program that asks for one
//! links it (decision 0024).

use std::sync::OnceLock;

/// What a profiler is told: a span `id` named so begins, a span ends, and
/// the GPU did work `name` between two of its clock's ticks, each `period`
/// nanoseconds.
pub struct Profiler {
    pub begin: fn(u64, &str),
    pub end: fn(u64),
    pub gpu: fn(&str, u64, u64, f32),
}

static PROFILER: OnceLock<Profiler> = OnceLock::new();

/// Turns the profiler `p` on, for the rest of the program.
pub fn set(p: Profiler) {
    let _ = PROFILER.set(p);
}

/// Whether a profiler is on.
pub fn on() -> bool {
    PROFILER.get().is_some()
}

pub(crate) fn begin(id: u64, name: &str) {
    if let Some(p) = PROFILER.get() {
        (p.begin)(id, name);
    }
}

pub(crate) fn end(id: u64) {
    if let Some(p) = PROFILER.get() {
        (p.end)(id);
    }
}

/// The GPU's work `name`, from tick `start` to tick `end`.
pub fn gpu(name: &str, start: u64, end: u64, period: f32) {
    if let Some(p) = PROFILER.get() {
        (p.gpu)(name, start, end, period);
    }
}
