//! The token cursor and helpers shared by every part of the parser.

use crate::SyntaxError;
use kavach_ast::{Ident, Path, Span};
use kavach_lexer::{Keyword, Token, TokenKind};

pub(crate) struct Parser<'a> {
    pub(crate) src: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    pub(crate) errors: Vec<SyntaxError>,
    /// Above zero while parsing a dot lambda such as `.status == Paid`.
    pub(crate) dot_lambda: u32,
    /// Set in a match guard, where `x => …` ends the guard instead of
    /// starting a lambda.
    pub(crate) in_guard: bool,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(src: &'a str, tokens: Vec<Token>) -> Self {
        // Lexer errors are already reported; their tokens carry no meaning.
        let tokens = tokens
            .into_iter()
            .filter(|t| t.kind != TokenKind::Error)
            .collect();
        Parser {
            src,
            tokens,
            pos: 0,
            errors: Vec::new(),
            dot_lambda: 0,
            in_guard: false,
        }
    }

    pub(crate) fn kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    pub(crate) fn nth(&self, n: usize) -> &TokenKind {
        let i = (self.pos + n).min(self.tokens.len() - 1);
        &self.tokens[i].kind
    }

    /// The text of the token `n` places ahead.
    pub(crate) fn nth_text(&self, n: usize) -> &'a str {
        let i = (self.pos + n).min(self.tokens.len() - 1);
        self.tokens[i].span.text(self.src)
    }

    pub(crate) fn token_at(&self, i: usize) -> &TokenKind {
        &self.tokens[i.min(self.tokens.len() - 1)].kind
    }

    pub(crate) fn position(&self) -> usize {
        self.pos
    }

    pub(crate) fn span(&self) -> Span {
        self.tokens[self.pos].span
    }

    pub(crate) fn text(&self) -> &'a str {
        self.span().text(self.src)
    }

    /// A span from `start` to the end of the last consumed token.
    pub(crate) fn span_from(&self, start: Span) -> Span {
        let end = if self.pos == 0 {
            start.start
        } else {
            self.tokens[self.pos - 1].span.end
        };
        Span {
            start: start.start,
            end: end.max(start.start),
        }
    }

    pub(crate) fn at(&self, kind: &TokenKind) -> bool {
        self.kind() == kind
    }

    pub(crate) fn at_kw(&self, keyword: Keyword) -> bool {
        matches!(self.kind(), TokenKind::Keyword(k) if *k == keyword)
    }

    /// At a contextual keyword such as `fails` or `context` (spec 1.6).
    pub(crate) fn at_word(&self, word: &str) -> bool {
        self.at(&TokenKind::Ident) && self.text() == word
    }

    pub(crate) fn at_end_of_line(&self) -> bool {
        matches!(
            self.kind(),
            TokenKind::Newline | TokenKind::RBrace | TokenKind::Eof
        )
    }

    pub(crate) fn bump(&mut self) {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
    }

    pub(crate) fn eat(&mut self, kind: &TokenKind) -> bool {
        let found = self.at(kind);
        if found {
            self.bump();
        }
        found
    }

    pub(crate) fn eat_kw(&mut self, keyword: Keyword) -> bool {
        let found = self.at_kw(keyword);
        if found {
            self.bump();
        }
        found
    }

    pub(crate) fn expect(&mut self, kind: &TokenKind, what: &str) -> bool {
        let found = self.eat(kind);
        if !found {
            self.error_here(format!("expected {what}"));
        }
        found
    }

    pub(crate) fn expect_word(&mut self, word: &str) {
        if self.at_word(word) {
            self.bump();
        } else {
            self.error_here(format!("expected `{word}`"));
        }
    }

    pub(crate) fn expect_kw(&mut self, keyword: Keyword) {
        if !self.eat_kw(keyword) {
            self.error_here(format!("expected `{}`", keyword.as_str()));
        }
    }

    /// Records an error, keeping only the first one at any position so a
    /// single mistake doesn't produce a cascade.
    pub(crate) fn error(&mut self, span: Span, message: impl Into<String>) {
        if self.errors.iter().any(|e| e.span.start == span.start) {
            return;
        }
        self.errors.push(SyntaxError {
            message: message.into(),
            span,
        });
    }

    pub(crate) fn error_here(&mut self, message: impl Into<String>) {
        let span = self.span();
        self.error(span, message);
    }

    pub(crate) fn skip_newlines(&mut self) {
        while self.at(&TokenKind::Newline) {
            self.bump();
        }
    }

    /// The next token that is not a line break, without consuming anything.
    pub(crate) fn peek_past_newlines(&self) -> &TokenKind {
        let mut i = self.pos;
        while self.tokens[i].kind == TokenKind::Newline {
            i += 1;
        }
        &self.tokens[i].kind
    }

    pub(crate) fn ident(&mut self, what: &str) -> Ident {
        let span = self.span();
        if self.at(&TokenKind::Ident) {
            let name = self.text().to_string();
            self.bump();
            Ident { name, span }
        } else {
            self.error_here(format!("expected {what}"));
            Ident {
                name: String::new(),
                span: Span {
                    start: span.start,
                    end: span.start,
                },
            }
        }
    }

    /// `a.b.c`
    pub(crate) fn path(&mut self, what: &str) -> Path {
        let start = self.span();
        let mut segments = vec![self.ident(what)];
        while self.at(&TokenKind::Dot) && *self.nth(1) == TokenKind::Ident {
            self.bump();
            segments.push(self.ident(what));
        }
        Path {
            segments,
            span: self.span_from(start),
        }
    }

    /// `a, b.c, d` (at least one).
    pub(crate) fn paths(&mut self, what: &str) -> Vec<Path> {
        let mut paths = vec![self.path(what)];
        while self.eat(&TokenKind::Comma) {
            paths.push(self.path(what));
        }
        paths
    }

    /// Skips to the end of the current line (or the `}` that closes the
    /// current block) after an error, so parsing can carry on.
    pub(crate) fn recover_line(&mut self) {
        use TokenKind::*;
        let mut depth = 0usize;
        loop {
            match self.kind() {
                Eof => return,
                Newline if depth == 0 => return,
                LParen | LBracket | LBrace | InterpStart | StrStart => depth += 1,
                // At depth 0, `}` ends the enclosing block: stop before it.
                RBrace | InterpEnd | StrEnd if depth == 0 => return,
                // Stray `)` or `]` at depth 0: skip them (the lexer reported them).
                RParen | RBracket if depth == 0 => {}
                RParen | RBracket | RBrace | InterpEnd | StrEnd => depth -= 1,
                _ => {}
            }
            self.bump();
        }
    }

    /// Consumes one `>`; splits a `>>` token when it closes two generic lists.
    pub(crate) fn close_angle(&mut self) {
        match self.kind() {
            TokenKind::Gt => self.bump(),
            TokenKind::Shr => {
                let token = &mut self.tokens[self.pos];
                token.kind = TokenKind::Gt;
                token.span.start += 1;
            }
            _ => self.error_here("expected `>`"),
        }
    }
}
