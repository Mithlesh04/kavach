//! File and function length limits (spec 12.1). Every line counts:
//! code, comments and blank lines. A long file gets a suggested split.

use crate::config::{Level, Limits};
use crate::{Message, Severity};
use kavach_ast::{Item, ItemKind, Span};

/// The offset where line `line` (1-based) starts.
fn line_start(src: &str, line: usize) -> u32 {
    src.split_inclusive('\n')
        .take(line - 1)
        .map(str::len)
        .sum::<usize>() as u32
}

/// Checks one file: `items` are its items, `text` its text, which starts
/// at `start` in `src`, the source of every file of the program.
pub(crate) fn check(
    items: &[Item],
    src: &str,
    start: u32,
    text: &str,
    path: &str,
    limits: &Limits,
) -> Vec<Message> {
    let mut out = Vec::new();
    if limits.level == Level::Off {
        return out;
    }
    let lines = text.lines().count();
    if lines > limits.file_lines {
        let at = start + line_start(text, limits.file_lines + 1);
        let name = std::path::Path::new(path)
            .file_name()
            .map_or(path.into(), |n| n.to_string_lossy().to_string());
        out.push(Message {
            severity: match limits.level {
                Level::Warn => Severity::Warning,
                _ => Severity::Error,
            },
            text: format!(
                "{name} has {lines} lines (limit {}){}",
                limits.file_lines,
                split_hint(items, src, start, &name, lines - limits.file_lines)
            ),
            span: Span::new(at as usize, at as usize),
        });
    }
    for item in items {
        let fns: Vec<&kavach_ast::FnDecl> = match &item.kind {
            ItemKind::Fn(f) => vec![f],
            ItemKind::Type(t) => match &t.body {
                kavach_ast::TypeBody::Record(r) => r
                    .methods
                    .iter()
                    .filter_map(|m| match &m.kind {
                        ItemKind::Fn(f) => Some(f),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            },
            ItemKind::Extend(e) => e
                .methods
                .iter()
                .filter_map(|m| match &m.kind {
                    ItemKind::Fn(f) => Some(f),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        for f in fns {
            let Some(body) = &f.body else { continue };
            // Counted in the body's own text: the program's size doesn't matter.
            let n = body.span.text(src).matches('\n').count() + 1;
            if n > limits.fn_lines {
                out.push(Message {
                    severity: Severity::Warning,
                    text: format!(
                        "`{}` has {n} lines (limit {}): consider moving a part into its own function",
                        f.name.name, limits.fn_lines
                    ),
                    span: f.name.span,
                });
            }
        }
    }
    out
}

/// "The file has 3 separate parts. Suggested split: …", moving the biggest
/// top-level parts out until at least `over` lines are gone.
fn split_hint(items: &[Item], src: &str, start: u32, name: &str, over: usize) -> String {
    let stem = name.trim_end_matches(".kav");
    // The line (1-based) within this file that offset `at` is on.
    let line = |at: u32| src[start as usize..at as usize].matches('\n').count() + 1;
    let mut parts: Vec<(usize, usize, String, String)> = items
        .iter()
        .filter_map(|item| {
            let (label, file_name) = match &item.kind {
                ItemKind::Fn(f) => (format!("fn {}", f.name.name), f.name.name.clone()),
                ItemKind::Type(t) => {
                    let methods = match &t.body {
                        kavach_ast::TypeBody::Record(r) => r.methods.len(),
                        _ => 0,
                    };
                    let label = match methods {
                        0 => format!("type {}", t.name.name),
                        n => format!("type {} + {n} methods", t.name.name),
                    };
                    (label, snake(&t.name.name))
                }
                ItemKind::Extend(_) | ItemKind::Interface(_) | ItemKind::Test(_) => {
                    let first = item
                        .span
                        .text(src)
                        .lines()
                        .next()
                        .unwrap_or("")
                        .trim_end_matches(" {");
                    (first.to_string(), "more".into())
                }
                _ => return None,
            };
            let (a, b) = (line(item.span.start), line(item.span.end));
            Some((a, b, label, file_name))
        })
        .collect();
    if parts.len() < 2 {
        return "\n  Split it into smaller files by topic.".into();
    }
    let count = parts.len();
    parts.sort_by_key(|(a, b, _, _)| std::cmp::Reverse(b - a));
    let mut moved = 0;
    let mut text = format!("\n  The file has {count} separate parts. Suggested split:");
    for (a, b, label, file_name) in parts {
        if moved >= over {
            break;
        }
        moved += b - a + 1;
        text.push_str(&format!(
            "\n    {label} (lines {a}–{b})  →  {stem}/{file_name}.kav"
        ));
    }
    text
}

/// `UserAccount` → `user_account`.
fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(c.to_lowercase());
    }
    out
}
