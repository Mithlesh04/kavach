//! `kavach check --format json` (spec 13.6): one compact line per message.

use kavach_driver::{Compiled, Severity};

/// Every message as a JSON object, one per line.
pub fn messages_json(c: &Compiled) -> String {
    let mut out = String::new();
    for m in &c.messages {
        let start = m.span.start as usize;
        let line_start = c.src[..start].rfind('\n').map_or(0, |i| i + 1);
        let col = c.src[line_start..start].chars().count() + 1;
        let severity = match m.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        out.push_str(&format!(
            "{{\"file\":{},\"line\":{},\"col\":{col},\"severity\":\"{severity}\",\"message\":{}}}\n",
            quote(c.location(m.span.start).0),
            c.location(m.span.start).1,
            quote(&m.text)
        ));
    }
    out
}

/// A JSON string literal.
fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
