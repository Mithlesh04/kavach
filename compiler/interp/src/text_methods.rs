//! Text methods (spec 14.2) from `kavach_runtime_core::text_methods`, as
//! native code calls them: `capitalize`, `title`, `words`, padding and
//! the rest of the methods that came from `std.text`, and those added
//! with them.

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use kavach_runtime_core::text_methods as tm;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// `s.name(args)` when it is one of these methods; `None` otherwise.
    pub(crate) fn text_method(
        &mut self,
        s: &str,
        name: &str,
        args: &[Value<'a>],
        span: Span,
    ) -> Option<Eval<'a>> {
        let text = |i: usize| args.get(i).map(Value::display).unwrap_or_default();
        let int = |i: usize| match args.get(i) {
            Some(Value::Int(n, _)) => *n,
            _ => 0,
        };
        let number = |n: usize| Value::Int(n as i128, Prim::Int);
        Some(Ok(match name {
            "trim_start" => self.str_value(s.trim_start()),
            "trim_end" => self.str_value(s.trim_end()),
            "capitalize" => self.str_value(&tm::capitalize(s)),
            "title" => self.str_value(&tm::title(s)),
            "is_blank" => Value::Bool(tm::is_blank(s)),
            "words" => {
                let all = tm::words(s)
                    .into_iter()
                    .map(|w| self.str_value(w))
                    .collect();
                Value::List(Rc::new(all))
            }
            "last_index_of" => match tm::last_index_of(s, &text(0)) {
                Some(at) => number(at),
                None => Value::None,
            },
            "count" => number(tm::count(s, &text(0))),
            "remove_prefix" => self.str_value(tm::remove_prefix(s, &text(0))),
            "remove_suffix" => self.str_value(tm::remove_suffix(s, &text(0))),
            "truncate" => {
                if int(0) < 0 {
                    return Some(Err(
                        self.panic("`truncate` takes a length of 0 or more", span)
                    ));
                }
                self.str_value(tm::truncate(s, int(0) as usize))
            }
            "pad_left" | "pad_right" => {
                let width = int(0);
                if !(0..=i128::from(tm::MOST_WIDTH)).contains(&width) {
                    let why = format!("`{name}` takes a width from 0 to {}", tm::MOST_WIDTH);
                    return Some(Err(self.panic(why, span)));
                }
                let fill = match args.get(1) {
                    Some(Value::Char(c)) => *c,
                    _ => ' ',
                };
                self.str_value(&tm::pad(s, width as usize, fill, name == "pad_left"))
            }
            _ => return None,
        }))
    }
}
