//! Every conformance test and example must type-check as the spec says.

use kavach_driver::{Options, Severity, compile};
use std::fs;
use std::path::{Path, PathBuf};

fn kav_files(dir: &Path, out: &mut Vec<PathBuf>) {
    // Project folders (several modules) are checked by the conformance
    // runner, which compiles them together.
    if dir.join("kavach.toml").is_file() && dir.join("src").is_dir() {
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            kav_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "kav") {
            out.push(path);
        }
    }
}

/// Files without `// expect compile error` must check cleanly. Every other
/// compile-error test must be caught by some stage, on its marked line.
#[test]
fn conformance_tests_and_examples_check() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    kav_files(&root.join("tests/conformance"), &mut files);
    kav_files(&root.join("examples"), &mut files);

    let mut problems = Vec::new();
    for path in &files {
        let src = fs::read_to_string(path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        // Compiled as a program is, with the standard library modules
        // every program has; only this file's errors count.
        let c = compile(&src, &name, Options::default());
        let errors: Vec<(usize, String)> = c
            .messages
            .iter()
            .filter(|m| m.severity == Severity::Error && c.is_own(m.span.start))
            .map(|m| (c.location(m.span.start).1, m.text.clone()))
            .collect();
        let marked = src
            .lines()
            .position(|l| l.contains("// error:"))
            .map(|i| i + 1);
        // An error a tag's checker finds comes after this stage, when
        // `kavach` runs the checker (spec 10.15); the conformance runner
        // checks those.
        if src.contains("// error: from check_sql") {
            continue;
        }
        let ok = match marked {
            None => errors.is_empty(),
            Some(m) => !errors.is_empty() && errors.iter().all(|(l, _)| *l == m),
        };
        if !ok {
            problems.push(format!("{name} (marked line {marked:?}): {errors:?}"));
        }
    }
    assert!(files.len() > 100, "found only {} files", files.len());
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}
