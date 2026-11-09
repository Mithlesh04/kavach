//! `spawn process f(args)` in native code (spec 9.8, decision 0075), as
//! the interpreter runs it (`compiler/interp/src/processes.rs`). The
//! program starts itself again (`runtime/core/src/processes.rs`); the
//! child runs one call, whose arguments came as JSON, and prints its
//! outcome last: `{"ok": value}`, or `{"error": {"type": …, "value": …}}`,
//! an error by its own type, which both share since they are one program.
//! The code that calls these ops is made by `compiler/kir/src/lower_process.rs`.
//! A module and entry of its own, so only a program that uses it links
//! it (decision 0024).

use crate::heap::{obj, release};
use crate::rt::type_of;
use kavach_kir::{RtOp, Type, type_code};
use kavach_runtime_core::json::{self, Limits, Tree};
use kavach_runtime_core::processes::{self as procs, Event};
use std::io::Write;

/// Limits wide enough for any arguments or outcome a program sends itself.
fn no_limits() -> Limits {
    Limits {
        max_depth: 10_000,
        max_bytes: usize::MAX,
        max_fields: usize::MAX,
        comments: false,
    }
}

crate::rt_data::entry!(kv_rt_process, |op, a, b, _c, code, line| {
    match op {
        // A task's work: call number `a` with the arguments in tuple `b`,
        // run in a child. Its value is of type `code`; its error is left
        // for `kv_take_error`.
        RtOp::ProcessRun => run(a, b, code, line),
        // In a child: the number of the call it runs, or -1.
        RtOp::ProcessSite => procs::site().unwrap_or(-1),
        // In a child: the call's arguments, as a tuple of type `code`.
        RtOp::ProcessArgs => {
            let given = json::parse(&procs::arguments(), &no_limits()).unwrap_or(Tree::Null);
            let decoded = crate::rt_json_decode::decode(&given, type_of(code), "", 0);
            decoded.unwrap_or_else(|_| {
                crate::rt::panic_with("this process was given wrong arguments", line)
            }) as i64
        }
        // In a child: the call's value, in one-item tuple `a` (0 for
        // nothing), or its error `a`, given back to the parent.
        RtOp::ProcessDone => {
            let value = match a {
                0 => Tree::Null,
                _ => match crate::rt_json::encode(a, obj(a).ty()) {
                    Tree::Array(mut items) if !items.is_empty() => items.remove(0),
                    _ => Tree::Null,
                },
            };
            if a != 0 {
                release(a);
            }
            give_back("ok", value);
            0
        }
        RtOp::ProcessFailed => {
            let ty = obj(a).ty();
            let error = Tree::Object(vec![
                ("type".into(), Tree::Int(type_code(ty))),
                ("value".into(), crate::rt_json::encode(a, ty)),
            ]);
            release(a);
            give_back("error", error);
            0
        }
        _ => 0,
    }
});

/// Prints the child's outcome, its last line.
fn give_back(key: &str, value: Tree) {
    let outcome = Tree::Object(vec![(key.into(), value)]);
    let line = procs::outcome_line(&json::write(&outcome, 0));
    crate::rt::print_piece(line.trim_end_matches('\n'));
    crate::rt::kv_print_end();
}

/// The parent's side: starts the child, prints its lines in turn while it
/// runs, and gives its value (or leaves its error). `cancel()` and a time
/// limit end the child.
fn run(site: i64, args: i64, code: i64, line: i32) -> i64 {
    let arguments = json::write(&crate::rt_json::encode(args, obj(args).ty()), 0);
    release(args);
    let running = procs::start(&procs::this_command(), site, &arguments)
        .unwrap_or_else(|e| crate::rt::panic_with(&format!("can't start a process: {e}"), line));
    loop {
        let seen = running.count();
        match running.next() {
            Some(Event::Line(text)) => {
                crate::rt::print_piece(&text);
                crate::rt::kv_print_end();
            }
            Some(Event::Ended {
                outcome,
                code: exit,
            }) => return outcome_of(outcome, exit, code),
            None => {
                let me = crate::rt_tasks::me();
                let waited = crate::rt_tasks::wait_outside(None, line, |wake| {
                    running.on_event(
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
                    running.kill();
                    return 0;
                }
            }
        }
    }
}

/// The value the child gave back, or its error left for the caller. A
/// child that gave nothing back panicked (it said why) or exited: so
/// does the program (spec 9.1).
fn outcome_of(outcome: Option<String>, exit: Option<i32>, code: i64) -> i64 {
    let tree = outcome.and_then(|o| json::parse(&o, &no_limits()).ok());
    let Some(Tree::Object(mut fields)) = tree else {
        let _ = std::io::stdout().flush();
        crate::rt::run_at_exit();
        std::process::exit(exit.unwrap_or(101));
    };
    let (key, value) = fields.remove(0);
    // The call panicked (the child printed it): the task's error, as for
    // any task (spec 9.1).
    if let ("panicked", Tree::Text(why)) = (key.as_str(), &value) {
        let text = crate::heap::first_of(&kavach_kir::Shape::Str);
        crate::rt_errors::set_error(crate::shapes::new_text(why, text));
        return 0;
    }
    if key == "ok" {
        let ty = type_of(code);
        if ty == Type::Unit {
            return 0;
        }
        return crate::rt_json_decode::decode(&value, ty, "", 0).unwrap_or(0) as i64;
    }
    let Tree::Object(mut e) = value else {
        return 0;
    };
    let (Some((_, Tree::Int(ty))), Some((_, v))) = (e.first().cloned(), e.pop()) else {
        return 0;
    };
    if let Ok(error) = crate::rt_json_decode::decode(&v, type_of(ty), "", 0) {
        crate::rt_errors::set_error(error as i64);
    }
    0
}
