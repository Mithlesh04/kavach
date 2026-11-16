//! Text views and builders (spec 2.18). Here a `StrView` is its text, a
//! `Value::Str` sharing its string's `Rc`, and a `TextBuilder` is a
//! `Value::Str` changed in place while nothing else holds it, copied
//! first when something does (spec 6.1). Nothing a program does can tell
//! these from the native forms.

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::Span;
use std::rc::Rc;

/// Byte offset of character `n` of `s`, clamped to its end.
fn byte_at(s: &str, n: i128) -> usize {
    let n = n.max(0) as usize;
    s.char_indices().nth(n).map_or(s.len(), |(i, _)| i)
}

impl<'a> Interp<'a> {
    /// `TextBuilder()`, `TextBuilder(text)`.
    pub(crate) fn text_builder(&self, args: Vec<Value<'a>>) -> Value<'a> {
        match args.into_iter().next() {
            Some(Value::Str(s)) => Value::Str(s),
            _ => Value::Str(Rc::new(String::new())),
        }
    }

    /// `TextBuilder.capacity(n)`: empty, with room for `n` bytes.
    pub(crate) fn builder_with_room(&self, n: i128) -> Value<'a> {
        let room = n.clamp(0, 1 << 30) as usize;
        Value::Str(Rc::new(String::with_capacity(room)))
    }

    /// A builder's change in place: `push`, `push_char`, `insert`,
    /// `replace`, `remove` and `clear`. A `start` after `end` panics.
    pub(crate) fn builder_change(
        &mut self,
        mut text: Rc<String>,
        name: &str,
        args: Vec<Value<'a>>,
        span: Span,
    ) -> Eval<'a> {
        let int = |i: usize| match args.get(i) {
            Some(Value::Int(n, _)) => *n,
            _ => 0,
        };
        let piece = |i: usize| match args.get(i) {
            Some(Value::Str(t)) => t.to_string(),
            Some(Value::Char(c)) => c.to_string(),
            _ => String::new(),
        };
        let (start, end, with) = match name {
            "push" | "push_char" => (i128::MAX, i128::MAX, piece(0)),
            "insert" => (int(0), int(0), piece(1)),
            "replace" => (int(0), int(1), piece(2)),
            "remove" => (int(0), int(1), String::new()),
            "clear" => (0, i128::MAX, String::new()),
            _ => {
                let shown = Value::Str(text).display();
                return Err(self.panic(format!("{shown} has no method `{name}`"), span));
            }
        };
        if start > end {
            let message = format!("contract failed: `{name}` starts after its end");
            return Err(self.panic(message, span));
        }
        let s = Rc::make_mut(&mut text);
        let (a, b) = (byte_at(s, start), byte_at(s, end));
        s.replace_range(a..b, &with);
        Ok(Value::Str(text))
    }
}
