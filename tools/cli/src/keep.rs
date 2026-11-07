//! `kavach test --keep` (spec 11.4): found cases that now pass become
//! `example` clauses on their functions, and leave `tests/found/`.

use kavach_ast::{FnDecl, Item, ItemKind, TypeBody};
use std::process::ExitCode;

/// Every function of `items`, methods too.
fn functions(items: &[Item]) -> Vec<&FnDecl> {
    let mut out = Vec::new();
    for item in items {
        match &item.kind {
            ItemKind::Fn(f) => out.push(f),
            ItemKind::Type(t) => {
                if let TypeBody::Record(r) = &t.body {
                    out.extend(functions(&r.methods));
                }
            }
            _ => {}
        }
    }
    out
}

/// `src` with `clauses` added to the function whose name starts at
/// `name`, before its body: on their own lines, indented one level more
/// than the function, with its `{` moved to a line of its own.
fn add_clauses(src: &str, brace: usize, name: usize, clauses: &[String]) -> String {
    let line_of = |at: usize| src[..at].rfind('\n').map_or(0, |i| i + 1);
    let fn_line = line_of(name);
    let indent: String = src[fn_line..].chars().take_while(|c| *c == ' ').collect();
    let lines: String = clauses
        .iter()
        .map(|c| format!("{indent}    {c}\n"))
        .collect();
    let brace_line = line_of(brace);
    if src[brace_line..brace].trim().is_empty() {
        return format!("{}{lines}{}", &src[..brace_line], &src[brace_line..]);
    }
    let sig_end = src[..brace].trim_end().len();
    format!("{}\n{lines}{indent}{}", &src[..sig_end], &src[brace..])
}

pub(crate) fn keep(path: &str, opts: &kavach_interp::TestOptions) -> ExitCode {
    let compiled = match crate::load(path) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let kept = kavach_interp::keep(&compiled, opts);
    for note in &kept.notes {
        eprintln!("note: kept as a found case: {note}");
    }
    if kept.examples.is_empty() {
        eprintln!("no found case passes yet: nothing to keep");
        return ExitCode::SUCCESS;
    }
    // Each function's body, from the last in the file to the first, so
    // earlier positions stay right.
    let mut targets: Vec<(usize, usize, Vec<String>)> = Vec::new();
    for f in functions(compiled.own_items()) {
        let clauses: Vec<String> = kept
            .examples
            .iter()
            .filter(|(at, _)| *at == f.name.span.start)
            .map(|(_, c)| c.clone())
            .collect();
        if let (false, Some(body)) = (clauses.is_empty(), &f.body) {
            targets.push((
                body.span.start as usize,
                f.name.span.start as usize,
                clauses,
            ));
        }
    }
    targets.sort_by_key(|a| std::cmp::Reverse(a.0));
    let Ok(mut src) = std::fs::read_to_string(path) else {
        eprintln!("error: can't read {path}");
        return ExitCode::from(1);
    };
    for (brace, name, clauses) in &targets {
        src = add_clauses(&src, *brace, *name, clauses);
    }
    let src = kavach_fmt::format(&src).unwrap_or(src);
    let check = kavach_driver::compile(&src, path, kavach_driver::Options::default());
    if check.has_errors() {
        eprintln!(
            "error: the new examples don't compile, so nothing was changed:\n{}",
            check.render()
        );
        return ExitCode::from(1);
    }
    if let Err(e) = std::fs::write(path, &src) {
        eprintln!("error: can't write {path}: {e}");
        return ExitCode::from(1);
    }
    if let Some(found) = &opts.found_file {
        let mut text = String::from(
            "// Found by `kavach test` (spec 11.4). Generated: commit it, never edit it.\n",
        );
        for line in &kept.remaining {
            text.push_str(line);
            text.push('\n');
        }
        let written = if kept.remaining.is_empty() {
            std::fs::remove_file(found)
        } else {
            std::fs::write(found, text)
        };
        if let Err(e) = written {
            eprintln!("error: can't update {}: {e}", found.display());
        }
    }
    for (_, clause) in &kept.examples {
        eprintln!("kept: {clause}");
    }
    ExitCode::SUCCESS
}
