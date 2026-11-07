//! Durable agents in native code (spec 8.5, decision 0025): the journal
//! (`kavach_runtime_core::journal`, the interpreter's too) of each started
//! agent that has one, the events it left unfinished, and values to and
//! from JSON. The code that calls these ops is made by
//! `compiler/kir/src/lower_agents.rs` and `lower_guard.rs`, only for
//! agents with a `journal`, so other programs don't link this.

use crate::heap::{obj, release, slots};
use crate::rt::{panic_with, type_of};
use crate::shapes::{shape, with_str};
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_core::journal::{Journal, Outcome, Unfinished};
use kavach_runtime_core::json::Tree;
use std::collections::VecDeque;
use std::sync::Mutex;

/// A started agent's journal; the events left to run again; the one
/// starting again now; the calls being made, each with its event (calls
/// of several events can be under way at once); and the outcome found
/// for the last call.
struct Durable {
    handle: i64,
    journal: Journal,
    unfinished: VecDeque<Unfinished>,
    again: Option<Unfinished>,
    calls: Vec<(i64, String)>,
    found: Option<Outcome>,
}

/// The running handler's event (`rt_agents::event`), or 0 outside one.
fn event() -> i64 {
    crate::rt_agents::event().unwrap_or(0)
}

static DURABLE: Mutex<Vec<Durable>> = Mutex::new(Vec::new());

fn with<R>(handle: i64, f: impl FnOnce(&mut Durable) -> R) -> R {
    let mut all = DURABLE.lock().unwrap_or_else(|e| e.into_inner());
    let d = all.iter_mut().find(|d| d.handle == handle);
    f(d.expect("an agent with a journal"))
}

fn text(p: i64) -> String {
    let s = with_str(p, str::to_string);
    release(p);
    s
}

/// The items of tuple `t` (owned) as JSON.
fn items(t: i64) -> Vec<Tree> {
    if t == 0 {
        return Vec::new();
    }
    let types = match shape(obj(t).ty()) {
        Shape::Tuple(ts) => ts.clone(),
        _ => Vec::new(),
    };
    let out = types
        .iter()
        .zip(slots(t))
        .map(|(ty, s)| crate::rt_json::encode(*s as i64, *ty))
        .collect();
    release(t);
    out
}

/// `t` as a value of type `ty`, with the decoding `templates` (owned).
fn value(t: &Tree, ty: Type, templates: i64, line: i32) -> i64 {
    let decoded = crate::rt_json_decode::decode(t, ty, "", templates);
    release(templates);
    match decoded {
        Ok(v) => v as i64,
        Err(_) => panic_with("the journal holds a value of another type", line),
    }
}

/// The journal ops (operands owned). `code` is the result type.
pub fn journal_op(op: RtOp, a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    match op {
        // The agent with guard `a` keeps its journal in file `b`.
        RtOp::JournalOpen => {
            let _ = crate::rt_std::JOURNALED_TIME.set(journaled_time);
            let path = text(b);
            if let Err(why) = kavach_runtime_core::agents::lock_folder(path.as_ref()) {
                panic_with(&why, line);
            }
            let (journal, unfinished) = Journal::open(path.into());
            crate::rt_agents::note_set_aside(a, &journal.set_aside);
            let mut all = DURABLE.lock().unwrap_or_else(|e| e.into_inner());
            all.retain(|d| d.handle != a);
            all.push(Durable {
                handle: a,
                journal,
                unfinished: unfinished.into(),
                again: None,
                calls: Vec::new(),
                found: None,
            });
            0
        }
        // The next unfinished event: its handler's place in the names
        // `b`, or -1 when there are no more.
        RtOp::JournalNext => {
            let names = text(b);
            let Some(u) = with(a, |d| d.unfinished.pop_front()) else {
                return -1;
            };
            let Some(i) = names.split(' ').position(|n| n == u.handler) else {
                let why = format!(
                    "the journal has an event of `{}`, which this agent no longer handles",
                    u.handler
                );
                panic_with(&why, line);
            };
            with(a, |d| d.again = Some(u));
            i as i64
        }
        // Argument `b` of the event starting again.
        RtOp::JournalArg => {
            let arg = with(a, |d| {
                d.again
                    .as_ref()
                    .and_then(|u| u.args.get(b as usize))
                    .cloned()
            });
            value(&arg.unwrap_or(Tree::Null), type_of(code), c, line)
        }
        // Handler `b` starts with the arguments in tuple `c`: a new event,
        // or the one starting again.
        RtOp::JournalBegin => {
            let (handler, args) = (text(b), items(c));
            let begun = with(a, |d| match d.again.take() {
                Some(u) => d.journal.again(u),
                None => d.journal.begin(&handler, args),
            });
            match begun {
                Ok(event) => crate::rt_agents::set_event(event),
                Err(why) => panic_with(&why, line),
            }
            0
        }
        // A handler whose task is being cancelled (the program ends while
        // it waits) didn't return: its event stays unfinished (spec 8.5).
        RtOp::JournalEnd if crate::rt_tasks::CANCELLING.get() => 0,
        RtOp::JournalEnd => {
            let ended = with(a, |d| d.journal.end(event()));
            stopped(ended, line)
        }
        // Before call `b`: 1 when the journal has its value, 2 its error,
        // 0 when it must run.
        RtOp::JournalRecorded => {
            let call = text(b);
            let found = with(a, |d| d.journal.recorded(event(), &call));
            match found {
                Ok(Some(o)) => {
                    let n = if matches!(o, Outcome::Ok(_)) { 1 } else { 2 };
                    with(a, |d| d.found = Some(o));
                    n
                }
                Ok(None) => {
                    with(a, |d| d.calls.push((event(), call)));
                    0
                }
                Err(why) => panic_with(&why, line),
            }
        }
        // The value or error found, with the decoding templates `b`.
        RtOp::JournalValue => {
            let found = with(a, |d| d.found.take());
            let tree = match found {
                Some(Outcome::Ok(t) | Outcome::Err(t)) => t,
                None => Tree::Null,
            };
            value(&tree, type_of(code), b, line)
        }
        // The call ran: its value (`b` is 1) or error (2), in tuple `c`.
        RtOp::JournalRecord => {
            let t = items(c).pop().unwrap_or(Tree::Null);
            let outcome = if b == 1 {
                Outcome::Ok(t)
            } else {
                Outcome::Err(t)
            };
            let event = event();
            let written = with(a, |d| {
                let at = d.calls.iter().rposition(|(e, _)| *e == event);
                let call = at.map(|i| d.calls.remove(i).1).unwrap_or_default();
                d.journal.record(event, &call, outcome)
            });
            stopped(written, line)
        }
        // `agents.call_key()` (spec 14.16): the running call's key, or
        // `none` outside a durable agent's capability call.
        RtOp::JournalCallKey => {
            let key = crate::rt_agents::call_running().and_then(|(handle, name)| {
                let all = DURABLE.lock().unwrap_or_else(|e| e.into_inner());
                all.iter()
                    .find(|d| d.handle == handle)
                    .and_then(|d| d.journal.call_key(event(), &name))
            });
            let text_ty = match shape(type_of(code)) {
                Shape::Optional(t) => *t,
                _ => type_of(code),
            };
            crate::rt_data::optional(key.map(|k| crate::shapes::new_text(&k, text_ty)), code)
        }
        _ => 0,
    }
}

/// 0 when the journal was written; a panic when it couldn't be, as a
/// journal that can't be written can't keep its promise (spec 8.5).
fn stopped(written: Result<(), String>, line: i32) -> i64 {
    match written {
        Ok(()) => 0,
        Err(why) => panic_with(&why, line),
    }
}

/// A durable handler's clock read or a wait's end, `call` (spec 8.5,
/// decision 0062): the moment its journal has, or `now`, journaled.
fn journaled_time(call: &str, now: i64, line: i32) -> i64 {
    let Some(handle) = crate::rt_agents::handler_running() else {
        return now;
    };
    let mut all = DURABLE.lock().unwrap_or_else(|e| e.into_inner());
    let Some(d) = all.iter_mut().find(|d| d.handle == handle) else {
        return now;
    };
    let event = event();
    match d.journal.recorded(event, call) {
        Ok(Some(Outcome::Ok(Tree::Int(ms)))) => ms,
        Ok(Some(_)) => now,
        Ok(None) => {
            let written = d.journal.record(event, call, Outcome::Ok(Tree::Int(now)));
            drop(all);
            stopped(written, line);
            now
        }
        Err(why) => {
            drop(all);
            panic_with(&why, line)
        }
    }
}

// The group's runtime entry (`RtOp::entry`, decision 0024).
crate::rt_data::entry!(kv_rt_journal, |op, a, b, c, code, line| {
    journal_op(op, a, b, c, code, line)
});

/// The event agent `handle` is about to run again: its number, handler
/// and arguments (`rt_replays.rs`).
pub(crate) fn replaying(handle: i64) -> Option<(i64, String, Vec<Tree>)> {
    let all = DURABLE.lock().unwrap_or_else(|e| e.into_inner());
    let u = all.iter().find(|d| d.handle == handle)?.again.as_ref()?;
    Some((u.number(), u.handler.clone(), u.args.clone()))
}

/// The key of the call agent `handle` (named `name`) checks now, in a
/// durable handler: an approval's request number, the same after a
/// restart (decision 0072). `None` outside a durable agent's event.
pub(crate) fn call_key_of(handle: i64, name: &str) -> Option<String> {
    let all = DURABLE.lock().unwrap_or_else(|e| e.into_inner());
    all.iter()
        .find(|d| d.handle == handle)
        .and_then(|d| d.journal.call_key(event(), name))
}
