//! Number and character literals, and the escapes shared with strings (spec 1.7).

use crate::lexer::{Lexer, is_ident_continue};
use crate::token::TokenKind;

impl Lexer<'_> {
    pub(crate) fn number(&mut self) {
        let start = self.pos;
        let rest = self.rest();
        if rest.starts_with("0x") || rest.starts_with("0b") || rest.starts_with("0o") {
            let (radix, name) = match rest.as_bytes()[1] {
                b'x' => (16, "hex"),
                b'o' => (8, "octal"),
                _ => (2, "binary"),
            };
            self.pos += 2;
            let digits_start = self.pos;
            self.eat_while(is_ident_continue);
            let digits: String = self.src[digits_start..self.pos]
                .chars()
                .filter(|&c| c != '_')
                .collect();
            if digits.is_empty() {
                return self.error_token(
                    start,
                    format!("a {name} number needs digits after `{}`", &rest[..2]),
                );
            }
            if !digits.chars().all(|c| c.is_digit(radix)) {
                return self.error_token(start, format!("invalid digit in {name} number"));
            }
            return match u64::from_str_radix(&digits, radix) {
                Ok(value) => self.push(TokenKind::Int(value), start),
                Err(_) => self.error_token(start, "this number is too large"),
            };
        }

        let digits = |c: char| c.is_ascii_digit() || c == '_';
        self.eat_while(digits);
        let mut float = false;
        if self.peek() == Some('.') && self.peek_nth(1).is_some_and(|c| c.is_ascii_digit()) {
            float = true;
            self.pos += 1;
            self.eat_while(digits);
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            let signed = matches!(self.peek_nth(1), Some('+' | '-'));
            let first_digit = if signed { 2 } else { 1 };
            if self
                .peek_nth(first_digit)
                .is_some_and(|c| c.is_ascii_digit())
            {
                float = true;
                self.pos += first_digit;
                self.eat_while(digits);
            }
        }
        if self.peek().is_some_and(is_ident_continue) {
            self.eat_while(is_ident_continue);
            // `2nd_place`: meant as a name (spec 1.5).
            let word = &self.src[start..self.pos];
            let parts: Vec<&str> = word.split('_').filter(|p| !p.is_empty()).collect();
            let better = match parts.as_slice() {
                [first, rest @ ..] if !rest.is_empty() => format!("{}_{first}", rest.join("_")),
                _ => format!("n{word}"),
            };
            // Reported once; then read as the name it was meant to be, so the
            // rest of the program is checked as usual.
            self.error(
                start,
                self.pos,
                format!("a name can't start with a digit (spec 1.5): start it with a letter, as `{better}`"),
            );
            return self.push(TokenKind::Ident, start);
        }
        if float {
            return self.push(TokenKind::Float, start);
        }
        let text: String = self.src[start..self.pos]
            .chars()
            .filter(|&c| c != '_')
            .collect();
        match text.parse::<u64>() {
            Ok(value) => self.push(TokenKind::Int(value), start),
            Err(_) => self.push(TokenKind::BigInt, start),
        }
    }

    pub(crate) fn char_literal(&mut self) {
        let start = self.pos;
        self.pos += 1;
        let value = match self.peek() {
            None | Some('\n') => {
                return self.error_token(start, "this character literal is never closed with `'`");
            }
            Some('\'') => {
                self.pos += 1;
                return self.error_token(
                    start,
                    "empty character literal: a Char holds exactly one character",
                );
            }
            Some('\\') => self.escape('\''),
            Some(c) => {
                self.pos += c.len_utf8();
                Some(c)
            }
        };
        if self.peek() == Some('\'') {
            self.pos += 1;
            match value {
                Some(c) => self.push(TokenKind::Char(c), start),
                None => self.push(TokenKind::Error, start),
            }
            return;
        }
        let line = &self.rest()[..self.rest().find('\n').unwrap_or(self.rest().len())];
        if let Some(i) = line.find('\'') {
            self.pos += i + 1;
        }
        self.error_token(
            start,
            "a Char holds exactly one character; use \"…\" for text",
        );
    }

    /// Reads an escape starting at `\`. `quote` is `"` in strings and `'` in
    /// characters, which decides whether `\"`, `\{` or `\'` is allowed.
    /// Reports an error and returns `None` for an invalid escape.
    pub(crate) fn escape(&mut self, quote: char) -> Option<char> {
        let start = self.pos;
        self.pos += 1;
        let c = match self.peek() {
            None | Some('\n') => {
                self.error(
                    start,
                    self.pos,
                    "unfinished escape: `\\` needs a character after it",
                );
                return None;
            }
            Some(c) => c,
        };
        self.pos += c.len_utf8();
        let value = match c {
            'n' => '\n',
            't' => '\t',
            '\\' => '\\',
            '"' | '{' if quote == '"' => c,
            '\'' if quote == '\'' => c,
            'u' => return self.unicode_escape(start),
            _ => {
                self.error(start, self.pos, format!("unknown escape `\\{c}`"));
                return None;
            }
        };
        Some(value)
    }

    fn unicode_escape(&mut self, start: usize) -> Option<char> {
        let mut digits = String::new();
        if self.peek() == Some('{') {
            self.pos += 1;
            while let Some(c) = self.peek().filter(char::is_ascii_hexdigit) {
                digits.push(c);
                self.pos += 1;
            }
            if self.peek() == Some('}') && (1..=6).contains(&digits.len()) {
                self.pos += 1;
                let value = u32::from_str_radix(&digits, 16)
                    .ok()
                    .and_then(char::from_u32);
                if value.is_none() {
                    self.error(
                        start,
                        self.pos,
                        format!("U+{digits} is not a valid Unicode character"),
                    );
                }
                return value;
            }
        }
        self.error(
            start,
            self.pos,
            "write Unicode escapes as \\u{1F600}: 1 to 6 hex digits in braces",
        );
        None
    }
}
