//! The Kavach lexer: turns source text into tokens (spec chapter 1).
//!
//! [`lex`] never fails. It returns every token it could read plus a list of
//! errors, so later stages can report many problems in one run.

mod lexer;
mod literals;
mod strings;
mod token;

pub use lexer::block_comment_len;
pub use token::{Keyword, Span, Token, TokenKind};

/// A problem in the source text, such as an unknown character or an
/// unterminated string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

/// The result of lexing: all tokens (always ending with [`TokenKind::Eof`])
/// and any errors found along the way.
#[derive(Clone, Debug)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub errors: Vec<LexError>,
}

/// Splits `src` into tokens.
pub fn lex(src: &str) -> Lexed {
    lexer::Lexer::new(src).run()
}
