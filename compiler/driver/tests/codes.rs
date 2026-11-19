// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Every message has a code saying its kind (spec 13.6), and the table
//! knows the kind of every error the conformance tests expect: a new
//! message whose wording the table doesn't know fails here, not quietly
//! as `check`.

use kavach_driver::{Options, Severity, codes, compile};
use std::path::Path;

/// The single-file conformance tests that expect a compile error.
fn compile_error_tests() -> Vec<std::path::PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance");
    let mut found = Vec::new();
    for chapter in std::fs::read_dir(&root).expect("the conformance tests") {
        let Ok(files) = std::fs::read_dir(chapter.expect("a chapter").path()) else {
            continue;
        };
        for file in files {
            let path = file.expect("a file").path();
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            if path.extension().is_some_and(|e| e == "kav")
                && text.lines().any(|l| l == "// expect compile error")
            {
                found.push(path);
            }
        }
    }
    found
}

#[test]
fn every_expected_error_has_a_known_kind() {
    let tests = compile_error_tests();
    assert!(tests.len() > 100, "found {} tests", tests.len());
    let mut unknown = Vec::new();
    for path in tests {
        let src = std::fs::read_to_string(&path).expect("the test");
        let name = path.to_string_lossy().to_string();
        let c = compile(&src, &name, Options::default());
        let Some(first) = c.messages.iter().find(|m| m.severity == Severity::Error) else {
            continue;
        };
        assert!(codes::all().contains(&first.code), "{}", first.code);
        if first.code == "check" {
            unknown.push(format!("{name}: {}", first.text));
        }
    }
    assert!(
        unknown.is_empty(),
        "add these messages' words to `compiler/driver/src/codes.rs`:\n{}",
        unknown.join("\n")
    );
}

#[test]
fn another_languages_habit_says_so_even_in_the_parser() {
    for src in [
        "fn main() {\n    let mut n = 1\n    print(n)\n}\n",
        "fn main() {\n    const x = 1;\n    print(x)\n}\n",
    ] {
        let c = compile(src, "main.kav", Options::default());
        assert_eq!(
            c.messages.first().map(|m| m.code),
            Some("not-kavach"),
            "{src}"
        );
    }
}

#[test]
fn syntax_errors_say_so() {
    let src = "fn main() {\n    print(1\n}\n";
    let c = compile(src, "main.kav", Options::default());
    assert_eq!(c.messages.first().map(|m| m.code), Some("syntax"));
}
