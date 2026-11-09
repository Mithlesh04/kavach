//! Writing JSON text (spec 2.12): compact, or indented one item or
//! field per line.

use super::Tree;

/// JSON text for `tree`: compact when `indent` is 0, otherwise one item
/// or field per line, indented by `indent` spaces a level.
pub fn write(tree: &Tree, indent: usize) -> String {
    let mut out = String::new();
    write_into(&mut out, tree, indent, 0);
    out
}

fn write_into(out: &mut String, tree: &Tree, indent: usize, level: usize) {
    let new_line = |out: &mut String, level: usize| {
        if indent > 0 {
            out.push('\n');
            out.extend(std::iter::repeat_n(' ', indent * level));
        }
    };
    match tree {
        Tree::Null => out.push_str("null"),
        Tree::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Tree::Int(n) => out.push_str(&n.to_string()),
        Tree::Number(text) => out.push_str(text),
        Tree::Float(f) if f.is_finite() => out.push_str(&crate::format_float(*f)),
        // As JavaScript does: JSON has no NaN or infinity.
        Tree::Float(_) => out.push_str("null"),
        Tree::Text(s) => write_text(out, s),
        Tree::Array(items) if items.is_empty() => out.push_str("[]"),
        Tree::Object(fields) if fields.is_empty() => out.push_str("{}"),
        Tree::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                new_line(out, level + 1);
                write_into(out, item, indent, level + 1);
            }
            new_line(out, level);
            out.push(']');
        }
        Tree::Object(fields) => {
            out.push('{');
            for (i, (key, value)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                new_line(out, level + 1);
                write_text(out, key);
                out.push_str(if indent > 0 { ": " } else { ":" });
                write_into(out, value, indent, level + 1);
            }
            new_line(out, level);
            out.push('}');
        }
    }
}

/// `s` in quotes, with `"`, `\` and control characters escaped.
pub fn write_text(out: &mut String, s: &str) {
    out.reserve(s.len() + 2);
    out.push('"');
    // What needs escaping is ASCII, so a plain run ends on a character's
    // edge whatever else the text holds.
    let mut run = 0;
    for (at, b) in s.bytes().enumerate() {
        if b >= 0x20 && b != b'"' && b != b'\\' {
            continue;
        }
        out.push_str(&s[run..at]);
        run = at + 1;
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            b => out.push_str(&format!("\\u{b:04x}")),
        }
    }
    out.push_str(&s[run..]);
    out.push('"');
}

/// `n`, as `n.to_string()` writes it, with no text of its own.
pub fn write_int(out: &mut String, n: i64) {
    if n < 0 {
        out.push('-');
    }
    let mut left = n.unsigned_abs();
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    loop {
        at -= 1;
        digits[at] = b'0' + (left % 10) as u8;
        left /= 10;
        if left == 0 {
            break;
        }
    }
    for d in &digits[at..] {
        out.push(char::from(*d));
    }
}
