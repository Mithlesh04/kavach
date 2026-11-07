//! `kavach query` (spec 13.2): exact answers from the compiler, one line
//! per hit, instead of text search and reading files.

use crate::index::{Decl, Index};

const KINDS: &str = "callers, uses, fails, type or find";

/// Answers `kind` about `name`. Errors explain what went wrong.
pub fn query(index: &Index, kind: &str, name: &str) -> Result<String, String> {
    let lines = match kind {
        "callers" => callers(index, name)?,
        "uses" => functions(index, |d| d.uses.iter().any(|u| u == name)),
        "fails" => {
            let failing = fails_with(index, name);
            functions(index, |d| failing.iter().any(|f| std::ptr::eq(*f, d)))
        }
        "type" => return type_outline(index, name),
        "find" => find(index, name),
        _ => return Err(format!("unknown query `{kind}`: use {KINDS}")),
    };
    if lines.is_empty() {
        return Ok(format!("no results for `{kind} {name}`\n"));
    }
    Ok(lines.join("\n") + "\n")
}

fn functions(index: &Index, keep: impl Fn(&Decl) -> bool) -> Vec<String> {
    index
        .decls
        .iter()
        .filter(|d| !d.signature.is_empty() && keep(d))
        .map(|d| format!("{}:{}  {}", d.file, d.line, d.signature))
        .collect()
}

/// The functions that can fail with `error` (spec 13.2): those whose
/// signature says `! error`, and those whose signature says `! Error`
/// (any error, spec 5.5) and pass one on with `try` from a call to such
/// a function.
fn fails_with<'i>(index: &'i Index, error: &str) -> Vec<&'i Decl> {
    let mut found: Vec<&Decl> = index
        .decls
        .iter()
        .filter(|d| d.fails.as_deref() == Some(error))
        .collect();
    loop {
        let passes_on = |d: &Decl| {
            index.refs.iter().any(|r| {
                r.within == d.name
                    && r.text.contains("try ")
                    && found
                        .iter()
                        .any(|f| f.name == r.name && f.module == r.module)
            })
        };
        let more: Vec<&Decl> = index
            .decls
            .iter()
            .filter(|d| d.fails.as_deref() == Some("Error"))
            .filter(|d| !found.iter().any(|f| std::ptr::eq(*f, *d)) && passes_on(d))
            .collect();
        if more.is_empty() {
            return found;
        }
        found.extend(more);
    }
}

/// Every place a declaration named `name` is used (not declared).
fn callers(index: &Index, name: &str) -> Result<Vec<String>, String> {
    let hits: Vec<String> = index
        .refs
        .iter()
        .filter(|r| r.is_named(name))
        .map(|r| format!("{}:{}  in {}: {}", r.file, r.line, r.within, r.text))
        .collect();
    let declared = index
        .decls
        .iter()
        .any(|d| d.name == name || d.is_named(name));
    if hits.is_empty() && !declared {
        return Err(format!("`{name}` is not declared in {}", index.entry));
    }
    Ok(hits)
}

fn type_outline(index: &Index, name: &str) -> Result<String, String> {
    let found = index.decls.iter().find(|d| {
        matches!(
            d.kind.as_str(),
            "type" | "interface" | "capability" | "agent"
        ) && d.is_named(name)
    });
    match found {
        Some(d) => Ok(format!("{}:{}\n{}\n", d.file, d.line, d.outline)),
        None => Err(format!(
            "no type, interface, capability or agent named `{name}` in {}",
            index.entry
        )),
    }
}

/// Declarations whose name contains `text`, ignoring case.
fn find(index: &Index, text: &str) -> Vec<String> {
    let text = text.to_lowercase();
    index
        .decls
        .iter()
        .filter(|d| d.name.to_lowercase().contains(&text))
        .map(|d| {
            let what = match (&d.owner, d.kind.as_str()) {
                (Some(owner), "field") => format!("field {owner}.{}", d.name),
                _ => format!("{} {}", d.kind, d.name),
            };
            format!("{}:{}  {what}", d.file, d.line)
        })
        .collect()
}
