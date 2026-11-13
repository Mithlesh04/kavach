//! The Kavach parser: turns tokens into a syntax tree ([`kavach_ast`]).
//!
//! [`parse`] never fails. It returns a tree for everything it could read and
//! a list of errors (lexer errors included), so many problems are reported
//! in one run.

mod exprs;
mod fns;
mod items;
mod literals;
mod parser;
mod patterns;
mod primary;
mod stmts;
mod type_decls;
mod types;

use kavach_ast::{File, Span};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxError {
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Parsed {
    pub file: File,
    pub errors: Vec<SyntaxError>,
}

/// Lexes and parses one `.kav` file.
pub fn parse(src: &str) -> Parsed {
    let lexed = kavach_lexer::lex(src);
    let mut errors: Vec<SyntaxError> = lexed
        .errors
        .into_iter()
        .map(|e| SyntaxError {
            message: e.message,
            span: e.span,
        })
        .collect();
    let mut parser = parser::Parser::new(src, lexed.tokens);
    let file = parser.file();
    // A lexer error (such as an unmatched `]`) often makes the parser stumble
    // at the same spot; keep only the lexer's clearer message.
    let lexer_starts: Vec<u32> = errors.iter().map(|e| e.span.start).collect();
    errors.extend(
        parser
            .errors
            .into_iter()
            .filter(|e| !lexer_starts.contains(&e.span.start)),
    );
    errors.sort_by_key(|e| e.span.start);
    Parsed { file, errors }
}
