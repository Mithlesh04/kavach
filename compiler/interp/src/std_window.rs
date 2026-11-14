//! `std.window` in the interpreter (spec 14.22), from
//! `kavach_runtime_window`, as native code does: windows by number.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_checker::Prim;
use kavach_runtime_window as win;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// `window.open_id`, `events_id` and the rest, with every argument
    /// given.
    pub(crate) fn window_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
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
                // Held while waiting: the task keeps its turn (spec 14.22).
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
                Value::List(Rc::new(both.to_vec()))
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
            _ => {
                win::close(n);
                Value::Unit
            }
        })
    }
}
