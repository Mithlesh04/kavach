//! The main loop: spaces, comments, line breaks, names and punctuation.
//! Numbers and characters are in `literals.rs`, strings in `strings.rs`.

use crate::strings::MultiLine;
use crate::token::{Keyword, PUNCTUATION, Span, Token, TokenKind, continues_line};
use crate::{LexError, Lexed};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StrKind {
    Plain,
    Raw,
    Multi,
    /// `sql"""…"""`: a tagged literal over lines, raw (spec 10.15).
    RawMulti,
}

impl StrKind {
    pub(crate) fn is_multi(self) -> bool {
        matches!(self, StrKind::Multi | StrKind::RawMulti)
    }

    pub(crate) fn is_raw(self) -> bool {
        matches!(self, StrKind::Raw | StrKind::RawMulti)
    }
}

/// An open bracket, or the `{` of a string interpolation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Delim {
    Paren,
    Bracket,
    Brace,
    Interp(StrKind),
}

pub(crate) struct Lexer<'a> {
    pub(crate) src: &'a str,
    pub(crate) pos: usize,
    pub(crate) tokens: Vec<Token>,
    pub(crate) errors: Vec<LexError>,
    /// Open delimiters with the position where each was opened.
    pub(crate) delims: Vec<(Delim, usize)>,
    /// Set while reading the text of a string (outside its interpolations).
    pub(crate) in_string: Option<StrKind>,
    /// Indentation bookkeeping for each open multi-line string.
    pub(crate) multiline: Vec<MultiLine>,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(src: &'a str) -> Self {
        Lexer {
            src,
            pos: 0,
            tokens: Vec::new(),
            errors: Vec::new(),
            delims: Vec::new(),
            in_string: None,
            multiline: Vec::new(),
        }
    }

    pub(crate) fn run(mut self) -> Lexed {
        loop {
            if let Some(kind) = self.in_string {
                self.string_body(kind);
            } else if self.skip_trivia() {
                self.token();
            } else {
                break;
            }
        }
        let open_strings: Vec<usize> = self
            .delims
            .iter()
            .filter(|(d, _)| matches!(d, Delim::Interp(_)))
            .map(|&(_, at)| at)
            .collect();
        for at in open_strings {
            self.error(
                at,
                at + 1,
                "this `{` inside a string is never closed with `}`",
            );
        }
        let end = self.src.len();
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(end, end),
        });
        Lexed {
            tokens: self.tokens,
            errors: self.errors,
        }
    }

    pub(crate) fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    pub(crate) fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    pub(crate) fn peek_nth(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
    }

    pub(crate) fn eat_while(&mut self, keep: impl Fn(char) -> bool) {
        while let Some(c) = self.peek() {
            if !keep(c) {
                break;
            }
            self.pos += c.len_utf8();
        }
    }

    /// Adds a token that runs from `start` to the current position.
    pub(crate) fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    pub(crate) fn error(&mut self, start: usize, end: usize, message: impl Into<String>) {
        self.errors.push(LexError {
            message: message.into(),
            span: Span::new(start, end),
        });
    }

    /// Reports an error and adds an `Error` token from `start` to here.
    pub(crate) fn error_token(&mut self, start: usize, message: impl Into<String>) {
        self.error(start, self.pos, message);
        self.push(TokenKind::Error, start);
    }

    /// Skips spaces, comments and line breaks, adding `Newline` tokens where a
    /// statement ends. Returns false at the end of the input.
    fn skip_trivia(&mut self) -> bool {
        loop {
            match self.peek() {
                None => return false,
                Some(' ' | '\t' | '\r') => self.pos += 1,
                // A byte order mark, as some editors write first (spec 1.1).
                Some('\u{feff}') => self.pos += '\u{feff}'.len_utf8(),
                Some('\n') => self.line_break(),
                Some('/') if is_line_comment(self.rest()) => self.eat_while(|c| c != '\n'),
                Some('/') if self.rest().starts_with("/*") => self.block_comment(),
                Some(_) => return true,
            }
        }
    }

    fn line_break(&mut self) {
        let start = self.pos;
        self.pos += 1;
        if !self.statement_continues() {
            self.push(TokenKind::Newline, start);
        }
    }

    /// Spec 1.1: a line break ends a statement unless it is inside `(…)` or
    /// `[…]`, the line ends with an operator, or the next line starts with `.`.
    fn statement_continues(&self) -> bool {
        if matches!(
            self.delims.last(),
            Some((Delim::Paren | Delim::Bracket | Delim::Interp(_), _))
        ) {
            return true;
        }
        match self.tokens.last() {
            None => return true,
            Some(t) if t.kind == TokenKind::Newline || continues_line(&t.kind) => return true,
            Some(_) => {}
        }
        let mut rest = self.rest();
        loop {
            rest = rest.trim_start_matches([' ', '\t', '\r', '\n']);
            if is_line_comment(rest) {
                rest = rest.find('\n').map_or("", |i| &rest[i..]);
            } else if rest.starts_with("/*") {
                rest = &rest[block_comment_len(rest).unwrap_or(rest.len())..];
            } else {
                return rest.starts_with('.');
            }
        }
    }

    fn token(&mut self) {
        let start = self.pos;
        let rest = self.rest();
        let Some(c) = self.peek() else { return };
        if c == 'r' && rest[1..].starts_with('"') {
            self.open_string(StrKind::Raw, 2);
        } else if is_ident_start(c) {
            self.eat_while(is_ident_continue);
            let kind = match Keyword::lookup(&self.src[start..self.pos]) {
                Some(keyword) => TokenKind::Keyword(keyword),
                None => TokenKind::Ident,
            };
            let ident = kind == TokenKind::Ident;
            self.push(kind, start);
            // `sql"…"`: a name right before a quote tags a raw literal
            // (spec 10.15).
            if ident {
                let rest = self.rest();
                if rest.starts_with("\"\"\"") {
                    self.open_string(StrKind::RawMulti, 3);
                } else if rest.starts_with('"') {
                    self.open_string(StrKind::Raw, 1);
                }
            }
        } else if c.is_ascii_digit() {
            self.number();
        } else if rest.starts_with("\"\"\"") {
            self.open_string(StrKind::Multi, 3);
        } else if c == '"' {
            self.open_string(StrKind::Plain, 1);
        } else if c == '\'' {
            self.char_literal();
        } else if rest.starts_with("///") {
            self.doc_comment();
        } else if c == ';' {
            self.pos += 1;
            self.error_token(
                start,
                "Kavach has no semicolons: a statement ends at the end of its line",
            );
        } else if rest.starts_with("===") || rest.starts_with("!==") {
            // JavaScript's strict comparisons: an error, and the expression
            // goes on as `==` or `!=`, so it says so once.
            let (op, fix, kind) = match rest.starts_with("===") {
                true => ("===", "==", TokenKind::EqEq),
                false => ("!==", "!=", TokenKind::NotEq),
            };
            self.pos += 3;
            self.error(
                start,
                self.pos,
                format!("Kavach has no `{op}`: write `{fix}`, which compares values by their contents (spec 1.8)"),
            );
            self.punctuation(kind, start);
        } else if rest.starts_with("&&") || rest.starts_with("||") {
            // C's and JavaScript's logic symbols: an error, and the
            // expression goes on as `and` or `or`, so it says so once.
            let (op, word, kind) = match rest.starts_with("&&") {
                true => ("&&", "and", Keyword::And),
                false => ("||", "or", Keyword::Or),
            };
            self.pos += 2;
            self.error(
                start,
                self.pos,
                format!("Kavach has no `{op}`: write `{word}` (spec 1.8)"),
            );
            self.punctuation(TokenKind::Keyword(kind), start);
        } else if let Some((text, kind)) = PUNCTUATION.iter().find(|(t, _)| rest.starts_with(t)) {
            self.pos += text.len();
            self.punctuation(kind.clone(), start);
        } else {
            self.pos += c.len_utf8();
            self.error_token(start, format!("unexpected character `{c}`"));
        }
    }

    /// A `/* … */` comment (spec 1.2), which may hold others: skipped
    /// like spaces, its line breaks included.
    fn block_comment(&mut self) {
        let start = self.pos;
        match block_comment_len(self.rest()) {
            Some(len) => self.pos += len,
            None => {
                self.pos = self.src.len();
                self.error(
                    start,
                    start + 2,
                    "this `/*` comment is never closed with `*/`",
                );
            }
        }
    }

    fn doc_comment(&mut self) {
        let start = self.pos;
        self.pos += 3;
        if self.peek() == Some(' ') {
            self.pos += 1;
        }
        let text_start = self.pos;
        self.eat_while(|c| c != '\n');
        let text = self.src[text_start..self.pos]
            .trim_end_matches('\r')
            .to_string();
        self.push(TokenKind::DocComment(text), start);
    }

    fn punctuation(&mut self, kind: TokenKind, start: usize) {
        match kind {
            TokenKind::LParen => self.delims.push((Delim::Paren, start)),
            TokenKind::LBracket => self.delims.push((Delim::Bracket, start)),
            TokenKind::LBrace => self.delims.push((Delim::Brace, start)),
            TokenKind::RParen => self.close(Delim::Paren, ")", start),
            TokenKind::RBracket => self.close(Delim::Bracket, "]", start),
            TokenKind::RBrace if self.close_brace(start) => {
                return;
            }
            _ => {}
        }
        self.push(kind, start);
    }

    fn close(&mut self, want: Delim, text: &str, start: usize) {
        if matches!(self.delims.last(), Some((d, _)) if *d == want) {
            self.delims.pop();
        } else {
            self.error(
                start,
                self.pos,
                format!("unexpected `{text}`: nothing here to close"),
            );
        }
    }

    /// Closes a block or an interpolation. Brackets left open inside it are
    /// reported. Returns true when the `}` ended an interpolation.
    fn close_brace(&mut self, start: usize) -> bool {
        let found = self
            .delims
            .iter()
            .rposition(|(d, _)| matches!(d, Delim::Brace | Delim::Interp(_)));
        let Some(index) = found else {
            self.error(start, self.pos, "unexpected `}`: nothing here to close");
            return false;
        };
        let unclosed: Vec<usize> = self.delims.drain(index + 1..).map(|(_, at)| at).collect();
        for at in unclosed {
            self.error(at, at + 1, "this bracket is never closed");
        }
        match self.delims.pop() {
            Some((Delim::Interp(kind), _)) => {
                self.push(TokenKind::InterpEnd, start);
                self.in_string = Some(kind);
                true
            }
            _ => false,
        }
    }
}

/// The length of the `/* … */` comment `rest` starts with, the comments
/// inside it included; none when it is never closed.
pub fn block_comment_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let (mut depth, mut i) = (0usize, 0);
    while i + 1 < bytes.len() {
        match (bytes[i], bytes[i + 1]) {
            (b'/', b'*') => {
                depth += 1;
                i += 2;
            }
            (b'*', b'/') => {
                depth -= 1;
                i += 2;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => i += 1,
        }
    }
    None
}

fn is_line_comment(rest: &str) -> bool {
    rest.starts_with("//") && (!rest.starts_with("///") || rest.starts_with("////"))
}

pub(crate) fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

pub(crate) fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric() || is_mark_like(c)
}

/// Non-ASCII characters that may continue a name, such as the combining
/// marks of Devanagari (the virama in `नमस्ते`). The standard library has no
/// Unicode category tables, so this approximates UAX #31: it accepts marks
/// and rejects whitespace, punctuation, symbols and emoji.
fn is_mark_like(c: char) -> bool {
    !c.is_ascii()
        && !c.is_whitespace()
        && !c.is_control()
        && !matches!(
            c as u32,
            0x00A0..=0x00BF | 0x00D7 | 0x00F7 | 0x2000..=0x2BFF | 0x3000..=0x303F | 0xFE10..=0xFE6F | 0x1F000..=0x1FAFF
        )
}
