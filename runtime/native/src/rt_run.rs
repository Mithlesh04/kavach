//! `std.process` in native code (spec 14.23, decision 0105), from
//! `kavach_runtime_process`, by the child's number, as the interpreter
//! does (`compiler/interp/src/std_process.rs`). Waiting for a child lets
//! other tasks take their turns (9.7). An entry of its own, so only a
//! program that runs other programs links it (decision 0024).

use crate::heap::{alloc_slots, new_str, release, shape, str_of};
use crate::rt::type_of;
use crate::rt_errors::set_error;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_core::fs_scope::may_run;
use kavach_runtime_process::{self as run, Event, Fail};

/// Leaves a `ProcessError` variant with `text` for `kv_take_error`; the
/// union is found by its first variant.
fn fail(variant: &str, text: &str) -> i64 {
    if let Some(ty) = crate::shapes::find_union("ProcessError", "NotFound") {
        let t = crate::shapes::new_text(text, crate::heap::first_of(&Shape::Str));
        set_error(crate::rt_json::new_variant(ty, variant, &[t as u64]));
    }
    0
}

/// A list of type `code` holding `texts`.
fn texts(texts: &[&str], code: i64) -> i64 {
    let item = match shape(type_of(code)) {
        Shape::List(t) => *t,
        _ => Type::I64,
    };
    let slots = texts
        .iter()
        .map(|s| new_str(s.to_string(), item) as u64)
        .collect();
    alloc_slots(type_of(code), 0, slots)
}

/// Child `id`'s next event, waiting for one; a cancelled task kills the
/// child's tree (spec 14.23) and gets its end.
fn next(id: i64, line: i32) -> Option<Event> {
    loop {
        let seen = run::count(id);
        if let Some(e) = run::next(id) {
            return Some(e);
        }
        if !run::is_open(id) {
            return None;
        }
        let me = crate::rt_tasks::me();
        let waited = crate::rt_tasks::wait_outside(None, line, |wake| {
            run::on_event(
                id,
                seen,
                Box::new(move || {
                    wake();
                    kavach_runtime_core::tasks::interrupt_queue_of(me);
                }),
            )
        });
        if !waited {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        if crate::rt_tasks::cancelling() {
            run::kill(id);
        }
    }
}

// `RunStart`: `a` the program, `b` its arguments, `c` the folder then
// the environment's names and values in turn (all owned). The others:
// `a` the child's number.
crate::rt_data::entry!(kv_rt_run, |op, a, b, c, code, line| {
    match op {
        RtOp::RunStart => {
            let program = str_of(a);
            let args = crate::rt_net::texts_of(b);
            let setup = crate::rt_net::texts_of(c);
            for p in [a, b, c] {
                release(p);
            }
            if !may_run(&crate::rt_scopes::scopes(), &program) {
                return fail("Denied", &program);
            }
            match run::start(&program, &args, &setup) {
                Ok(id) => id,
                Err(Fail::NotFound(p)) => fail("NotFound", &p),
                Err(Fail::Failed(why)) => fail("Failed", &why),
            }
        }
        RtOp::RunNext => match next(a, line) {
            Some(Event::Out(t)) => texts(&["out", &t], code),
            Some(Event::Err(t)) => texts(&["err", &t], code),
            _ => texts(&[], code),
        },
        RtOp::RunExit => {
            let exit = loop {
                match next(a, line) {
                    Some(Event::Ended(e)) => break e,
                    Some(_) => {}
                    None => {
                        break run::Exit {
                            code: -1,
                            cpu_ms: 0,
                            peak_bytes: 0,
                        };
                    }
                }
            };
            let numbers = [exit.code, exit.cpu_ms, exit.peak_bytes].map(|n| n as u64);
            alloc_slots(type_of(code), 0, numbers.to_vec())
        }
        RtOp::RunKill => {
            run::kill(a);
            0
        }
        RtOp::RunClose => {
            run::close(a);
            0
        }
        _ => run::peak_bytes(),
    }
});
