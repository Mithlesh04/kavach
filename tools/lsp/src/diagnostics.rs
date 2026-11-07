//! Checking a document, and its errors and warnings as LSP diagnostics.
//! The messages are the compiler's own, so they read the same as in
//! `kavach check`.

use crate::position::position;
use kavach_driver::{Compiled, Options, Severity, Source};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Checks the file at `path` whose text is `text`. In a project, the
/// whole program is checked: `src/main.kav` first, then every other file
/// of `src/` (spec 10.1–10.3). `open` gives the editor's text of files
/// that are open, which may not be saved. Also returns the error in
/// `kavach.toml`, if there is one.
pub fn check(
    path: &str,
    text: &str,
    open: impl Fn(&str) -> Option<String>,
) -> (Compiled, Option<String>) {
    let file = Path::new(path);
    let entry = project_main(file).unwrap_or_else(|| file.to_path_buf());
    let (options, config_error) = match kavach_driver::Config::for_file(&entry) {
        Ok(config) => (config.options_for(&entry), None),
        Err(e) => (Options::default(), Some(e)),
    };
    let compiled = match kavach_driver::project_sources(&entry) {
        Some(sources) => {
            let sources: Vec<Source> = sources
                .into_iter()
                .map(|s| Source {
                    text: open(&s.path).unwrap_or(s.text),
                    ..s
                })
                .collect();
            kavach_driver::compile_sources(sources, options)
        }
        None => kavach_driver::compile(text, path, options),
    };
    (compiled, config_error)
}

/// The `src/main.kav` of the project whose `src/` holds `file`.
fn project_main(file: &Path) -> Option<PathBuf> {
    let root = file.ancestors().find(|d| d.join("kavach.toml").is_file())?;
    let src = root.join("src");
    let main = src.join("main.kav");
    (file.starts_with(&src) && main.is_file()).then_some(main)
}

/// The diagnostics of each of the program's own files, by path. A file
/// without messages gets an empty list, which clears what it showed.
pub fn diagnostics(c: &Compiled) -> Vec<(String, Vec<Value>)> {
    let own = c.files.iter().take_while(|f| c.is_own(f.start)).count();
    let mut out: Vec<(String, Vec<Value>)> = c.files[..own]
        .iter()
        .map(|f| (f.path.clone(), Vec::new()))
        .collect();
    if own == 0 {
        return out;
    }
    for m in &c.messages {
        // Text the parser can't close (a string left open) runs on into
        // the standard library; such messages belong to the last own file.
        let i = c
            .files
            .partition_point(|f| f.start <= m.span.start)
            .saturating_sub(1)
            .min(own - 1);
        let start = c.files[i].start as usize;
        let end = c.files.get(i + 1).map_or(c.src.len(), |f| f.start as usize);
        let text = &c.src[start..end];
        let at = |offset: u32| position(text, (offset as usize).saturating_sub(start));
        let severity = match m.severity {
            Severity::Error => 1,
            Severity::Warning => 2,
        };
        out[i].1.push(json!({
            "range": {"start": at(m.span.start), "end": at(m.span.end.max(m.span.start))},
            "severity": severity,
            "source": "kavach",
            "message": m.text,
        }));
    }
    out
}

/// A diagnostic for an error in `kavach.toml`, shown on the first line.
pub fn config_diagnostic(error: &str) -> Value {
    let at = json!({"line": 0, "character": 0});
    json!({
        "range": {"start": at, "end": at},
        "severity": 1,
        "source": "kavach",
        "message": format!("kavach.toml: {error}"),
    })
}
