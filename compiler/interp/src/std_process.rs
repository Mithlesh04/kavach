// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.process` in the interpreter (spec 14.23), from
//! `kavach_runtime_process`, as native code does: children by number.
//! Waiting for a child lets other tasks take their turns (9.7).

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use kavach_runtime_core::fs_scope::may_run;
use kavach_runtime_process::{self as run, Event, Fail};
use std::rc::Rc;
use std::sync::Arc;

impl<'a> Interp<'a> {
    /// `process.start_id`, `next_id` and the rest, with every argument
    /// given.
    pub(crate) fn process_call(&mut self, name: &str, args: &[Value<'a>], span: Span) -> Eval<'a> {
        let id = match args.first() {
            Some(Value::Int(n, _)) => (*n).clamp(i64::MIN as i128, i64::MAX as i128) as i64,
            _ => 0,
        };
        let texts = |v: Option<&Value<'a>>| -> Vec<String> {
            match v {
                Some(Value::List(items)) => (items.iter())
                    .map(|t| match t {
                        Value::Str(s) => s.to_string(),
                        _ => String::new(),
                    })
                    .collect(),
                _ => Vec::new(),
            }
        };
        let list = |items: Vec<Value<'a>>| Value::List(Rc::new(items.into()));
        Ok(match name {
            "start_id" => {
                let program = match args.first() {
                    Some(Value::Str(s)) => s.to_string(),
                    _ => String::new(),
                };
                let (given, setup) = (texts(args.get(1)), texts(args.get(2)));
                if !may_run(&self.fs_scopes, &program) {
                    return Err(self.process_error("Denied", "program", &program));
                }
                match run::start(&program, &given, &setup) {
                    Ok(id) => Value::Int(i128::from(id), Prim::Int),
                    Err(Fail::NotFound(p)) => {
                        return Err(self.process_error("NotFound", "program", &p));
                    }
                    Err(Fail::Failed(why)) => {
                        return Err(self.process_error("Failed", "reason", &why));
                    }
                }
            }
            "next_id" => match self.next_event(id, span)? {
                Some(Event::Out(t)) => list(vec![self.str_value("out"), self.str_value(&t)]),
                Some(Event::Err(t)) => list(vec![self.str_value("err"), self.str_value(&t)]),
                _ => list(Vec::new()),
            },
            "exit_id" => {
                let exit = loop {
                    match self.next_event(id, span)? {
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
                let numbers = [exit.code, exit.cpu_ms, exit.peak_bytes];
                list(
                    numbers
                        .map(|n| Value::Int(i128::from(n), Prim::Int))
                        .to_vec(),
                )
            }
            "kill_id" => {
                run::kill(id);
                Value::Unit
            }
            "close_id" => {
                run::close(id);
                Value::Unit
            }
            "cpu_ms_id" => Value::Int(i128::from(run::cpu_ms()), Prim::Int),
            _ => Value::Int(i128::from(run::peak_bytes()), Prim::Int),
        })
    }

    /// `std.system`'s built-in functions (spec 14.25): a fact of the
    /// machine by its number, and the GPUs as four texts each.
    pub(crate) fn system_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        let which = match args.first() {
            Some(Value::Int(n, _)) => (*n).clamp(i64::MIN as i128, i64::MAX as i128) as i64,
            _ => -1,
        };
        let text = |s: String| Value::Str(Rc::new(s));
        Ok(match name {
            "basic_text_id" | "text_id" => text(run::machine::text(which)),
            "basic_number_id" | "number_id" => {
                Value::Int(i128::from(run::machine::number(which)), Prim::Int)
            }
            "rows_id" | "critical_id" => {
                // A critical kind only through its own call.
                let critical = run::parts::CRITICAL.contains(&which);
                let rows = if critical == (name == "critical_id") {
                    run::parts::rows(which)
                } else {
                    Vec::new()
                };
                Value::List(Rc::new(rows.into_iter().map(text).collect()))
            }
            // Tests can't change the machine they run on (spec 14.25).
            "act_id" if self.no_changes => {
                text("Denied:a test can't change the machine (spec 14.25)".to_string())
            }
            "act_id" => {
                let words = match args.get(1) {
                    Some(Value::Str(s)) => s.to_string(),
                    _ => String::new(),
                };
                let number = match args.get(2) {
                    Some(Value::Int(n, _)) => (*n).clamp(i64::MIN as i128, i64::MAX as i128) as i64,
                    _ => 0,
                };
                text(run::acts::act(which, &words, number))
            }
            _ => {
                let gpus = kavach_runtime_gpu::adapters().into_iter().flatten();
                Value::List(Rc::new(gpus.map(text).collect()))
            }
        })
    }

    /// A `ProcessError` variant with one text field.
    fn process_error(&self, variant: &str, field: &'static str, text: &str) -> Flow<'a> {
        let v = self.str_value(text);
        Flow::Fail(self.builtin_variant("ProcessError", variant, vec![(field, v)]))
    }

    /// Child `id`'s next event, waiting for one while other tasks run; a
    /// cancelled task kills the child's tree (spec 14.23).
    fn next_event(&mut self, id: i64, span: Span) -> Result<Option<Event>, Flow<'a>> {
        loop {
            let seen = run::count(id);
            if let Some(e) = run::next(id) {
                return Ok(Some(e));
            }
            if !run::is_open(id) {
                return Ok(None);
            }
            let Some(sched) = self.tasks.as_ref().map(|t| Arc::clone(&t.sched)) else {
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            };
            self.flush();
            let me = self.task;
            let (wait, waker) = (sched.next_wait(me), Arc::clone(&sched));
            if !run::on_event(id, seen, Box::new(move || waker.wake_from_thread(me, wait))) {
                continue;
            }
            if let Err(stop) = sched.wait_io(me, None) {
                run::kill(id);
                return Err(self.stopped(stop, span));
            }
        }
    }
}
