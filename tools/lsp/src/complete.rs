//! Code completion. After a `.`, the fields and methods of what is before
//! it; anywhere else, the names in scope: the program's declarations, the
//! enclosing function's parameters and the locals before the cursor, and
//! the built-ins. Only those starting with what is typed so far.
//!
//! A half-typed `order.` doesn't parse, so it is checked without the `.`
//! and what follows it on the line: then `order` has a type.

use crate::position::offset;
use kavach_ast::{FnDecl, ItemKind, TypeBody, TypeKind};
use kavach_checker::Ty;
use kavach_driver::Compiled;
use kavach_resolver::DefKind;
use serde_json::{Value, json};

/// LSP completion item kinds.
const METHOD: u64 = 2;
const FUNCTION: u64 = 3;
const FIELD: u64 = 5;
const VARIABLE: u64 = 6;
const CLASS: u64 = 7;
const INTERFACE: u64 = 8;
const MODULE: u64 = 9;
const ENUM_MEMBER: u64 = 20;

fn item(label: &str, kind: u64) -> Value {
    json!({"label": label, "kind": kind})
}

/// `textDocument/completion` at `at` in `text`, the file at `path`;
/// `check` compiles a version of the file's text.
pub fn complete(path: &str, text: &str, at: &Value, check: impl Fn(&str) -> Compiled) -> Value {
    let cursor = offset(text, at);
    let before = &text[..cursor];
    let typed = before.len()
        - before
            .trim_end_matches(|c: char| c.is_alphanumeric() || c == '_')
            .len();
    let prefix = &before[cursor - typed..];
    let head = &before[..cursor - typed];
    let items = match head.strip_suffix('.') {
        Some(receiver) => {
            let line_end = text[cursor..].find('\n').map_or(text.len(), |i| cursor + i);
            let without = format!("{receiver}{}", &text[line_end..]);
            members(&check(&without), path, receiver.len())
        }
        None => names(&check(text), path, cursor - typed),
    };
    let mut seen = std::collections::HashSet::new();
    let items: Vec<Value> = items
        .into_iter()
        .filter(|i| {
            let label = i["label"].as_str().unwrap_or("");
            label.starts_with(prefix) && seen.insert(label.to_string())
        })
        .collect();
    json!({"isIncomplete": false, "items": items})
}

/// Where byte `at` of the file at `path` is in the combined source.
fn global(c: &Compiled, path: &str, at: usize) -> Option<u32> {
    let file = c.files.iter().find(|f| {
        f.path == path || std::fs::canonicalize(&f.path).ok() == std::fs::canonicalize(path).ok()
    })?;
    Some(file.start + at as u32)
}

/// The fields and methods of the expression that ends at byte `end`.
fn members(c: &Compiled, path: &str, end: usize) -> Vec<Value> {
    let Some(end) = global(c, path, end) else {
        return Vec::new();
    };
    let found = c
        .expr_types
        .iter()
        .filter(|((_, e), _)| *e == end)
        .min_by_key(|((s, e), _)| e - s);
    let Some((_, ty)) = found else {
        return Vec::new();
    };
    let ty = match ty {
        Ty::Result(ok, _) => ok.as_ref(),
        t => t,
    };
    let Ty::Adt(def, _) = ty else {
        let methods = kavach_checker::builtin_methods(ty, &c.resolution);
        return methods.into_iter().map(|m| item(m, METHOD)).collect();
    };
    let mut out: Vec<Value> = c
        .field_types
        .keys()
        .filter(|(d, _)| d == def)
        .map(|(_, name)| item(name, FIELD))
        .collect();
    out.sort_by(|a, b| a["label"].as_str().cmp(&b["label"].as_str()));
    let declared = c.resolution.defs[*def].span.map(|s| s.start);
    let has_self = |f: &FnDecl| f.params.first().is_some_and(|p| p.ty.is_none());
    for i in &c.file.items {
        let methods = match &i.kind {
            ItemKind::Type(t) if Some(t.name.span.start) == declared => match &t.body {
                TypeBody::Record(r) => &r.methods,
                TypeBody::Union(_) | TypeBody::Alias(_) => continue,
            },
            ItemKind::Extend(e) => match &e.target.kind {
                TypeKind::Named { path, .. }
                    if c.resolution.uses.get(&path.segments[0].span.start) == Some(def) =>
                {
                    &e.methods
                }
                _ => continue,
            },
            _ => continue,
        };
        for m in methods {
            if let ItemKind::Fn(f) = &m.kind
                && has_self(f)
            {
                out.push(item(&f.name.name, METHOD));
            }
        }
    }
    out
}

/// The names in scope at byte `at` of the file at `path`.
fn names(c: &Compiled, path: &str, at: usize) -> Vec<Value> {
    let Some(at) = global(c, path, at) else {
        return Vec::new();
    };
    // The innermost function around the cursor, for its parameters and
    // locals.
    let mut around = None;
    let mut visit = |items: &[kavach_ast::Item]| {
        for i in items {
            if let ItemKind::Fn(_) = &i.kind
                && i.span.start <= at
                && at <= i.span.end
            {
                around = Some(i.span);
            }
        }
    };
    visit(&c.file.items);
    for i in &c.file.items {
        match &i.kind {
            ItemKind::Type(t) => {
                if let TypeBody::Record(r) = &t.body {
                    visit(&r.methods);
                }
            }
            ItemKind::Extend(e) => visit(&e.methods),
            _ => {}
        }
    }
    // Methods are reached through a value (`members`), not by name.
    let in_types: Vec<_> = c
        .file
        .items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::Type(_) | ItemKind::Extend(_)))
        .map(|i| i.span)
        .collect();
    let is_method = |s: kavach_ast::Span| {
        in_types
            .iter()
            .any(|t| t.start <= s.start && s.end <= t.end)
    };
    let mut out = Vec::new();
    for d in &c.resolution.defs {
        let kind = match d.kind {
            DefKind::Param | DefKind::Local { .. } => {
                let inside = d
                    .span
                    .zip(around)
                    .is_some_and(|(s, f)| f.start <= s.start && s.start < at);
                if !inside {
                    continue;
                }
                VARIABLE
            }
            DefKind::Builtin => match d.name.chars().next() {
                Some(c) if c.is_uppercase() => CLASS,
                _ => FUNCTION,
            },
            _ if !d.span.is_some_and(|s| c.is_own(s.start)) => continue,
            DefKind::Fn if d.span.is_some_and(is_method) => continue,
            DefKind::Fn => FUNCTION,
            DefKind::Type | DefKind::Agent => CLASS,
            DefKind::Variant { .. } => ENUM_MEMBER,
            DefKind::Interface | DefKind::Capability => INTERFACE,
            DefKind::Import | DefKind::Module => MODULE,
            _ => continue,
        };
        out.push(item(&d.name, kind));
    }
    out
}
