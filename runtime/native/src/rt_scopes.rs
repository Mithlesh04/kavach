//! What a running task's calls set up, in native code, as the
//! interpreter keeps it (`fs_scopes`): the folders of `uses Fs("…")`
//! (spec 8.1), the program of `uses Process("…")` (14.23), and the spans
//! and seeds of blocks given to a call (4.8, decision 0107). One stack a
//! task, swapped as tasks take turns on a thread (`rt_coroutines.rs`),
//! kept by the tasks it spawns, seeds left out.

use crate::heap::release;
use crate::shapes::{str_of, with_str};
use kavach_kir::RtOp;
use kavach_runtime_core::fs_scope::{self as scope, Scope};
use kavach_runtime_core::trace::{Finished, SpanId, parse_traceparent, traceparent};
use std::sync::{Arc, Mutex, OnceLock};

thread_local! {
    /// This task's stack: one entry for each such call or block running.
    static SCOPES: std::cell::RefCell<Vec<Scope>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Where spans that end go: set by `rt_otlp.rs` when `[log.otlp]` names a
/// collector, so a program without one links no network code.
pub(crate) static SPAN_SINK: OnceLock<fn(Vec<Finished>)> = OnceLock::new();

/// Puts `scopes` in place of this thread's, giving those: tasks switching
/// on one thread keep their own (`rt_coroutines.rs`).
pub(crate) fn swap_scopes(scopes: Vec<Scope>) -> Vec<Scope> {
    SCOPES.with(|s| std::mem::replace(&mut *s.borrow_mut(), scopes))
}

/// What a task spawned now keeps.
pub(crate) fn inherited() -> Vec<Scope> {
    SCOPES.with(|s| scope::inherited(&s.borrow()))
}

pub(crate) fn scopes() -> Vec<Scope> {
    SCOPES.with(|s| s.borrow().clone())
}

pub(crate) fn set_scopes(scopes: Vec<Scope>) {
    SCOPES.with(|s| *s.borrow_mut() = scopes);
}

/// Whether `path` is inside this task's folders.
pub(crate) fn allowed(path: &str) -> bool {
    SCOPES.with(|s| scope::inside(&s.borrow(), path))
}

/// The span the task runs in.
pub(crate) fn current_span() -> Option<SpanId> {
    SCOPES.with(|s| scope::current_span(&s.borrow()))
}

/// 64 bits from the innermost `random.seeded` block, if any.
pub(crate) fn seeded_bits() -> Option<u64> {
    SCOPES.with(|s| scope::seeded_bits(&s.borrow()))
}

fn push(entry: Scope) -> i64 {
    SCOPES.with(|s| {
        let mut s = s.borrow_mut();
        s.push(entry);
        s.len() as i64 - 1
    })
}

// Spans and seeds: an entry of their own, so a program without them
// doesn't link their code (decision 0024).
crate::rt_data::entry!(kv_rt_scopes, |op, a, b, _c, code, _line| {
    // From the first span or seed on, leaving ends spans too.
    let _ = LEAVE.set(leave_ending_spans);
    match op {
        RtOp::ScopeSpan => {
            let name = str_of(a);
            let remote = (b != 0).then(|| with_str(b, parse_traceparent)).flatten();
            release(a);
            if b != 0 {
                release(b);
            }
            let entry = SCOPES.with(|s| scope::span(&s.borrow(), &name, remote));
            push(entry)
        }
        RtOp::ScopeSeeded => {
            let g = kavach_runtime_core::seeded::Seeded::new(a as u64);
            push(Scope::Seeded(Arc::new(Mutex::new(g))))
        }
        RtOp::ScopeMark => SCOPES.with(|s| s.borrow().len() as i64),
        _ => {
            // ScopeTraceId, ScopeTraceparent.
            let text = match (current_span(), op) {
                (Some(id), RtOp::ScopeTraceId) => format!("{:032x}", id.trace),
                (Some(id), _) => traceparent(id),
                (None, _) => String::new(),
            };
            crate::shapes::new_text(&text, crate::rt::type_of(code))
        }
    }
});

/// How `FsScopeLeave` goes back to a depth: a plain cut, until a span or
/// seed has been made, and from then on `leave_ending_spans`.
static LEAVE: OnceLock<fn(usize, bool)> = OnceLock::new();

/// Goes back to `depth`, ending the spans above it (marked as errors when
/// `failed`), sent where `SPAN_SINK` says.
fn leave_ending_spans(depth: usize, failed: bool) {
    let failed = failed.then_some("failed");
    let ended = SCOPES.with(|s| scope::leave(&mut s.borrow_mut(), depth, failed));
    if let (false, Some(sink)) = (ended.is_empty(), SPAN_SINK.get()) {
        sink(ended);
    }
}

/// The folder and program scope ops (text operands owned): each `enter`
/// gives the depth that `FsScopeLeave` (with `b` set when the block
/// failed) goes back to.
pub(crate) fn scope_op(op: RtOp, a: i64, b: i64) -> i64 {
    match op {
        RtOp::FsScopeEnter | RtOp::ScopeProgram => {
            let text = str_of(a);
            release(a);
            push(match op {
                RtOp::FsScopeEnter => Scope::Folder(scope::resolve(&text)),
                _ => Scope::Program(text),
            })
        }
        _ => {
            // FsScopeLeave.
            let depth = a.max(0) as usize;
            match LEAVE.get() {
                Some(leave) => leave(depth, b != 0),
                None => SCOPES.with(|s| s.borrow_mut().truncate(depth)),
            }
            0
        }
    }
}
