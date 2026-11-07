//! Every conformance test and example must parse as the spec says, and the
//! parser must never panic or hang on random input.

mod common;
use common::line_of;
use kavach_parser::parse;
use std::fs;
use std::path::{Path, PathBuf};

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

/// Files without `// expect compile error` must parse cleanly. In
/// compile-error tests, syntax errors may only be on the line marked
/// `// error:`. For the rules the lexer and parser own, they must be found.
#[test]
fn conformance_tests_and_examples_parse() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    kav_files(&root.join("tests/conformance"), &mut files);
    kav_files(&root.join("examples"), &mut files);
    assert!(files.len() > 90, "found only {} .kav files", files.len());

    let syntax_rules = [
        "no-semicolons.kav",
        "block-comments-must-close.kav",
        "assignment-is-a-statement.kav",
        "signature-order-is-fixed.kav",
        "parameter-types-required.kav",
        "positional-arguments-come-first.kav",
        "optional-field-takes-one-question-mark.kav",
        "optional-field-has-no-other-default.kav",
    ];
    let mut problems = Vec::new();
    for path in &files {
        let src = fs::read_to_string(path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let marked = src
            .lines()
            .position(|l| l.contains("// error:"))
            .map(|i| i + 1);
        let errors: Vec<(usize, String)> = parse(&src)
            .errors
            .iter()
            .map(|e| (line_of(&src, e.span.start as usize), e.message.clone()))
            .collect();
        let on_marked_line = |line: usize| errors.iter().all(|(l, _)| *l == line);
        let ok = match marked {
            None => errors.is_empty(),
            Some(line) if syntax_rules.contains(&name.as_str()) => {
                !errors.is_empty() && on_marked_line(line)
            }
            Some(line) => on_marked_line(line),
        };
        if !ok {
            problems.push(format!("{name} (marked line {marked:?}): {errors:?}"));
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

const PIECES: &[&str] = &[
    "fn f(a: Int) -> Int ",
    "{",
    "}",
    "(",
    ")",
    "[",
    "]",
    "const x = ",
    "1",
    " + ",
    "a.b",
    "(x)",
    "\n",
    "if a ",
    "else ",
    "match x ",
    "=> ",
    "type T = A | B",
    "type R {",
    "x: Int",
    "\"s{",
    "}\"",
    ",",
    "catch ",
    "try ",
    "fail ",
    ".f",
    "<",
    ">",
    "?",
    "with ",
    "agent A ",
    "limit ",
    "per ",
    "require ",
    "! E",
    "uses Net",
    "example ",
    "for i in ",
    "while ",
    "_",
    "ok(",
    "-",
    "not ",
];

#[test]
fn random_input_never_breaks_the_parser() {
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state as usize
    };
    for _ in 0..3000 {
        let len = next() % 30;
        let src: String = (0..len).map(|_| PIECES[next() % PIECES.len()]).collect();
        let parsed = parse(&src);
        for e in &parsed.errors {
            assert!(
                e.span.end as usize <= src.len(),
                "bad error span in {src:?}"
            );
        }
    }
}
