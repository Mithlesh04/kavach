//! Shared helpers: show tokens as short text so tests read like the source.
// Each test file uses a different subset of these helpers.
#![allow(dead_code)]

use kavach_lexer::{TokenKind, lex};

/// Renders the tokens of `src` as one line, for example
/// `const x = int:1 ⏎ "< text:"hi" >"`. Names, keywords and punctuation are
/// shown as written; `${` and `}$` mark an interpolation.
pub fn show(src: &str) -> String {
    let lexed = lex(src);
    let mut out = Vec::new();
    for t in &lexed.tokens {
        let text = t.span.text(src);
        out.push(match &t.kind {
            TokenKind::Int(v) => format!("int:{v}"),
            TokenKind::Float => format!("float:{text}"),
            TokenKind::BigInt => format!("bigint:{text}"),
            TokenKind::Char(c) => format!("char:{c:?}"),
            TokenKind::Text(s) => format!("text:{s:?}"),
            TokenKind::DocComment(s) => format!("doc:{s:?}"),
            TokenKind::StrStart => "\"<".to_string(),
            TokenKind::StrEnd => ">\"".to_string(),
            TokenKind::InterpStart => "${".to_string(),
            TokenKind::InterpEnd => "}$".to_string(),
            TokenKind::Newline => "⏎".to_string(),
            TokenKind::Error => "ERR".to_string(),
            TokenKind::Eof => continue,
            _ => text.to_string(),
        });
    }
    out.join(" ")
}

/// The lexer's error messages for `src`, each prefixed with its line number.
pub fn errors(src: &str) -> Vec<String> {
    lex(src)
        .errors
        .iter()
        .map(|e| format!("{}: {}", line_of(src, e.span.start as usize), e.message))
        .collect()
}

pub fn line_of(src: &str, offset: usize) -> usize {
    src[..offset].matches('\n').count() + 1
}

/// Checks every `(source, expected)` pair and reports all mismatches at once.
pub fn check_all(cases: &[(&str, &str)]) {
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(src, want)| {
            let got = show(src);
            (got != *want).then(|| format!("source: {src:?}\n  want: {want}\n   got: {got}"))
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
