//! Hover and go to definition. Both come from the compile that gives the
//! diagnostics: the checker's type of every expression, and the resolver's
//! definition of every name.

use crate::position::{offset, position};
use kavach_checker::Ty;
use kavach_driver::Compiled;
use serde_json::{Value, json};

/// Where `at` (an LSP position in `text`, the file at `path`) is in the
/// compiled program's combined source.
fn global(c: &Compiled, path: &str, text: &str, at: &Value) -> Option<u32> {
    let file = c.files.iter().find(|f| same_file(&f.path, path))?;
    Some(file.start + offset(text, at) as u32)
}

pub(crate) fn same_file(a: &str, b: &str) -> bool {
    a == b
        || match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        }
}

/// The file holding byte `at` of the combined source, and `at` within it.
fn local(c: &Compiled, at: u32) -> Option<(&str, u32)> {
    let file = c.files.iter().rev().find(|f| f.start <= at)?;
    Some((file.path.as_str(), at - file.start))
}

/// `textDocument/hover`: the type of the smallest expression under the
/// cursor (a name's, with the name), or `null`.
pub fn hover(c: &Compiled, path: &str, text: &str, at: &Value) -> Value {
    let Some(at) = global(c, path, text, at) else {
        return Value::Null;
    };
    let found = c
        .expr_types
        .iter()
        .filter(|((start, end), _)| *start <= at && at < *end)
        .min_by_key(|((start, end), _)| end - start);
    let Some(((start, end), ty)) = found else {
        return Value::Null;
    };
    let code = &c.src[*start as usize..*end as usize];
    let shown = show(c, ty);
    let is_name = !code.is_empty() && code.chars().all(|ch| ch.is_alphanumeric() || ch == '_');
    let value = if is_name {
        format!("{code}: {shown}")
    } else {
        shown
    };
    let file_start = *start - local(c, *start).map_or(0, |(_, l)| l);
    let file_text = &c.src[file_start as usize..];
    json!({
        "contents": {"kind": "markdown", "value": format!("```kavach\n{value}\n```")},
        "range": {
            "start": position(file_text, (*start - file_start) as usize),
            "end": position(file_text, (*end - file_start) as usize),
        },
    })
}

/// `textDocument/definition`: where the name under the cursor is declared,
/// as `{uri, range}` with the file's path; `null` for built-ins and
/// anything else.
pub fn definition(c: &Compiled, path: &str, text: &str, at: &Value) -> Option<(String, Value)> {
    let at = global(c, path, text, at)?;
    let res = &c.resolution;
    let (_, def) = res.uses.iter().find(|(start, def)| {
        let len = res.defs[**def].name.len() as u32;
        **start <= at && at < **start + len.max(1)
    })?;
    let span = res.defs[*def].span?;
    let (file, start) = local(c, span.start)?;
    let file_start = span.start - start;
    let file_text = &c.src[file_start as usize..];
    let range = json!({
        "start": position(file_text, start as usize),
        "end": position(file_text, (span.end - file_start) as usize),
    });
    Some((file.to_string(), range))
}

/// A type as people write it: `[Int]`, `Order?`, `Receipt ! PaymentError`.
pub fn show(c: &Compiled, ty: &Ty) -> String {
    let list = |ts: &[Ty]| ts.iter().map(|t| show(c, t)).collect::<Vec<_>>().join(", ");
    let args = |ts: &[Ty]| {
        if ts.is_empty() {
            String::new()
        } else {
            format!("<{}>", list(ts))
        }
    };
    let name = |d: usize| c.resolution.defs[d].name.clone();
    match ty {
        Ty::Prim(p) => p.name().to_string(),
        Ty::Unit => "nothing".to_string(),
        Ty::Never => "never".to_string(),
        Ty::Error => "?".to_string(),
        Ty::List(t) => format!("[{}]", show(c, t)),
        Ty::Set(t) => format!("Set<{}>", show(c, t)),
        Ty::Map(k, v) => format!("[{}: {}]", show(c, k), show(c, v)),
        Ty::Optional(t) => format!("{}?", show(c, t)),
        Ty::Range(t) => format!("Range<{}>", show(c, t)),
        Ty::Tuple(ts) => format!("({})", list(ts)),
        Ty::Fn(ps, r, limit) => {
            let uses = match limit.as_deref() {
                None => String::new(),
                Some([one]) => format!(" uses {one}"),
                Some(caps) => format!(" uses ({})", caps.join(", ")),
            };
            format!("fn({}) -> {}{uses}", list(ps), show(c, r))
        }
        Ty::Adt(d, ts) => format!("{}{}", name(*d), args(ts)),
        Ty::Builtin(n, ts) => format!("{n}{}", args(ts)),
        Ty::Param(d) => name(*d),
        Ty::Result(ok, err) => format!("{} ! {}", show(c, ok), show(c, err)),
        _ => "_".to_string(),
    }
}
