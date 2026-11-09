//! Keeps the lexer in step with the spec and the conformance tests.

mod common;
use common::line_of;
use kavach_lexer::{Keyword, lex};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn keywords_match_spec_section_1_6() {
    let spec = fs::read_to_string(repo_root().join("docs/spec/01-basics.md")).unwrap();
    let section = &spec[spec.find("## 1.6 Keywords").unwrap()..];
    let block = section.split("```").nth(1).unwrap();
    let in_spec: BTreeSet<&str> = block.split_whitespace().collect();
    let in_lexer: BTreeSet<&str> = Keyword::ALL.iter().map(|k| k.as_str()).collect();
    assert_eq!(in_spec, in_lexer, "spec 1.6 and Keyword::ALL differ");
    for k in Keyword::ALL {
        assert_eq!(Keyword::lookup(k.as_str()), Some(*k));
    }
}

fn kav_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            kav_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "kav") {
            out.push(path);
        }
    }
}

/// Every conformance test and example must lex without errors, except
/// compile-error tests, whose lexer errors (if any) must be on the line
/// marked `// error:`. For rules the lexer owns, it must find the error.
#[test]
fn conformance_tests_and_examples_lex_as_expected() {
    let root = repo_root();
    let mut files = Vec::new();
    kav_files(&root.join("tests/conformance"), &mut files);
    kav_files(&root.join("examples"), &mut files);
    assert!(files.len() > 90, "found only {} .kav files", files.len());

    let lexer_rules = ["no-semicolons.kav", "block-comments-must-close.kav"];
    let mut problems = Vec::new();
    for path in &files {
        let src = fs::read_to_string(path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let marked = src
            .lines()
            .position(|l| l.contains("// error:"))
            .map(|i| i + 1);
        let error_lines: Vec<usize> = lex(&src)
            .errors
            .iter()
            .map(|e| line_of(&src, e.span.start as usize))
            .collect();
        let ok = match marked {
            None => error_lines.is_empty(),
            Some(line) if lexer_rules.contains(&name.as_str()) => {
                !error_lines.is_empty() && error_lines.iter().all(|&l| l == line)
            }
            Some(line) => error_lines.iter().all(|&l| l == line),
        };
        if !ok {
            problems.push(format!(
                "{name}: lexer errors on lines {error_lines:?}, marked line {marked:?}"
            ));
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}
