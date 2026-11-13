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
                // Characters of the text to bytes of the source.
                let (at, length) = (int("at"), int("length").max(1));
                let place = |i: usize| match quote {
                    1 => text_start as usize + byte_of(&u.text, i),
                    _ => in_lines(&c.src, text_start as usize, &u.text, i),
                };
                let span = kavach_ast::Span {
                    start: place(at) as u32,
                    end: place(at + length) as u32,
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

/// The byte where character `i` of `text` starts (its end, past the last).
fn byte_of(text: &str, i: usize) -> usize {
    text.char_indices().nth(i).map_or(text.len(), |(b, _)| b)
}

/// Where character `i` of multi-line text `text` is in `src`, whose
/// `"""` ends at `after_quotes`: the text's lines are the source's
/// lines after that one, each with the common indentation removed
/// (spec 1.7), and a tagged literal has no escapes (spec 10.15).
fn in_lines(src: &str, after_quotes: usize, text: &str, i: usize) -> usize {
    let next_line = |from: usize| src[from..].find('\n').map_or(src.len(), |n| from + n + 1);
    let at = byte_of(text, i);
    let line = text[..at].matches('\n').count();
    let line_start = text[..at].rfind('\n').map_or(0, |n| n + 1);
    let text_line = text[line_start..].split('\n').next().unwrap_or("");
    let mut start = next_line(after_quotes);
    for _ in 0..line {
        start = next_line(start);
    }
    let src_line = src[start..].split('\n').next().unwrap_or("");
    let src_line = src_line.strip_suffix('\r').unwrap_or(src_line);
    let indent = src_line.len().saturating_sub(text_line.len());
    (start + indent + (at - line_start)).min(src.len())
}
