// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.window` in the interpreter (spec 14.22), from
//! `kavach_runtime_window`, as native code does: windows by number; and
//! `std.input`'s gamepads (spec 14.26), from `kavach_runtime_input`.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use kavach_runtime_window as win;
use std::rc::Rc;
use std::sync::Arc;

impl<'a> Interp<'a> {
    /// `window.open_id`, `events_id` and the rest, with every argument
    /// given.
    pub(crate) fn window_call(&mut self, name: &str, args: &[Value<'a>], span: Span) -> Eval<'a> {
        let int = |i: usize| match args.get(i) {
            Some(Value::Int(n, _)) => (*n).clamp(i64::MIN as i128, i64::MAX as i128) as i64,
            _ => 0,
        };
        let flag = |i: usize| matches!(args.get(i), Some(Value::Bool(true)));
        let text = |i: usize| match args.get(i) {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let n = int(0);
        Ok(match name {
            "open_id" => match win::open(&text(0), int(1), int(2)) {
                Ok(id) => Value::Int(i128::from(id), Prim::Int),
                Err(why) => {
                    let reason = self.str_value(&why);
                    return Err(Flow::Fail(self.builtin_variant(
                        "WindowError",
                        "Unavailable",
                        vec![("reason", reason)],
                    )));
                }
            },
            "events_id" => {
                if flag(1) {
                    self.wait_for_events(n, span)?;
                }
                self.flush();
                let numbers = win::take_numbers(n, flag(1));
                Value::List(Rc::new(numbers.into_iter().map(Value::Float).collect()))
            }
            "texts_id" => {
                let texts = win::texts(n).iter().map(|t| self.str_value(t)).collect();
                Value::List(Rc::new(texts))
            }
            "size_id" => {
                let (w, h) = win::size(n);
                let both = [w, h].map(|v| Value::Int(i128::from(v), Prim::Int));
                Value::List(Rc::new(both.to_vec().into()))
            }
            "scale_id" => Value::Float(win::scale(n)),
            "listen_id" => {
                win::listen_input(n);
                Value::Unit
            }
            "set_title_id" => {
                win::set_title(n, &text(1));
                Value::Unit
            }
            "set_fullscreen_id" => {
                win::set_fullscreen(n, flag(1));
                Value::Unit
            }
            "lock_cursor_id" => {
                win::lock_cursor(n, flag(1));
                Value::Unit
            }
            "wake_id" => {
                win::wake(n);
                Value::Unit
            }
            "text_input_id" => {
                win::set_text_input(n, flag(1));
                Value::Unit
            }
            "caret_id" => {
                let part = |k: usize| match args.get(1) {
                    Some(Value::List(xs)) => match xs.get(k) {
                        Some(Value::Float(f)) => *f,
                        _ => 0.0,
                    },
                    _ => 0.0,
                };
                win::set_caret(n, [part(0), part(1), part(2), part(3)]);
                Value::Unit
            }
            _ => {
                win::close(n);
                Value::Unit
            }
        })
    }

    /// Waits until window `n` has an event or is closed, while other
    /// tasks run (spec 14.22); without tasks, `take` waits alone.
    fn wait_for_events(&mut self, n: i64, span: Span) -> Result<(), Flow<'a>> {
        let Some(sched) = self.tasks.as_ref().map(|t| Arc::clone(&t.sched)) else {
            return Ok(());
        };
        while !win::ready(n) {
            self.flush();
            let me = self.task;
            let (wait, waker) = (sched.next_wait(me), Arc::clone(&sched));
            if !win::on_event(n, Box::new(move || waker.wake_from_thread(me, wait))) {
                continue;
            }
            if let Err(stop) = sched.wait_io(me, None) {
                return Err(self.stopped(stop, span));
            }
        }
        Ok(())
    }

    /// `input.pads_id`, `names_id`, `rumble_id` and `start_id`.
    pub(crate) fn input_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        use kavach_runtime_input as input;
        Ok(match name {
            "pads_id" => {
                let numbers = input::pad_numbers();
                Value::List(Rc::new(numbers.into_iter().map(Value::Float).collect()))
            }
            "names_id" => {
                let names = input::names().iter().map(|t| self.str_value(t)).collect();
                Value::List(Rc::new(names))
            }
            "rumble_id" => {
                let pad = match args.first() {
                    Some(Value::Int(n, _)) => *n as i64,
                    _ => -1,
                };
                let part = |k: usize| match args.get(1) {
                    Some(Value::List(m)) => match m.get(k) {
                        Some(Value::Float(f)) => *f,
                        _ => 0.0,
                    },
                    _ => 0.0,
                };
                let ms = match args.get(2) {
                    Some(Value::Int(n, _)) => (*n).clamp(0, 60_000) as i64,
                    _ => 0,
                };
                input::rumble_pad(pad, part(0), part(1), ms);
                Value::Unit
            }
            _ => {
                input::start();
                Value::Unit
            }
        })
    }
}
