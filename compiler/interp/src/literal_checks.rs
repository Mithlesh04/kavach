//! Tagged literals checked while the program is compiled (spec 10.15):
//! each tag's checker runs on each literal's text, and what it finds
//! becomes a compile error or warning at its place in the literal.

use crate::interp::Interp;
use crate::value::{Flow, Value};
use kavach_driver::{Message, Severity};

/// About a second of work for one check (the generated tests' rate).
const CHECK_STEPS: u64 = 1_000_000;

impl Interp<'_> {
    pub(crate) fn literal_problems(&mut self) -> Vec<Message> {
        let c = self.c;
        let mut out = Vec::new();
        for u in &c.literal_uses {
            let Some(f) = self.fns.get(&u.check).copied() else {
                continue;
            };
            let quote = if c.src[u.span.start as usize..].starts_with("\"\"\"") {
                3
            } else {
                1
            };
            let text_start = u.span.start + quote;
            let whole = u.span;
            self.steps = 0;
            self.step_limit = Some(CHECK_STEPS);
            let arg = self.str_value(&u.text);
            let result = self.call_fn(f, None, vec![(None, arg)], whole);
            self.step_limit = None;
            let items = match result {
                Ok(Value::List(items)) => items,
                Ok(_) => continue,
                Err(Flow::Panic(why, _)) => {
                    let text = format!("the checker of `{}` stopped: {why} (spec 10.15)", u.tag);
                    out.push(Message {
                        severity: Severity::Error,
                        text,
                        span: whole,
                    });
                    continue;
                }
                Err(_) => continue,
            };
            for item in items.iter() {
                let Value::Record { fields, .. } = item else {
                    continue;
                };
                let get = |name: &str| fields.iter().find(|f| &*f.name == name).map(|f| &f.value);
                let int = |name: &str| match get(name) {
                    Some(Value::Int(n, _)) => usize::try_from(*n).unwrap_or(0),
                    _ => 0,
                };
                let text = |name: &str| match get(name) {
                    Some(Value::Str(s)) => s.to_string(),
                    _ => String::new(),
                };
                let warning = matches!(get("warning"), Some(Value::Bool(true)));
                // Characters of the text to bytes of the source; in text
                // over lines, the literal's start.
                let span = if quote == 1 {
                    let byte = |i: usize| {
                        u.text
                            .char_indices()
                            .nth(i)
                            .map_or(u.text.len(), |(b, _)| b)
                    };
                    let (at, length) = (int("at"), int("length").max(1));
                    kavach_ast::Span {
                        start: text_start + byte(at) as u32,
                        end: text_start + byte(at + length) as u32,
                    }
                } else {
                    whole
                };
                let hint = text("hint");
                let mut message = format!("{}: {}", u.tag, text("message"));
                if !hint.is_empty() {
                    message.push_str(&format!(" ({hint})"));
                }
                let severity = if warning {
                    Severity::Warning
                } else {
                    Severity::Error
                };
                out.push(Message {
                    severity,
                    text: message,
                    span,
                });
            }
        }
        out
    }
}
