// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Dict literals, written with braces (spec 2.2, decision 0129):
//! `{"asha": 31}`, `{name: "Asha"}` (a plain name is that text),
//! `{[key]: 1}` (the value of `key`), and `{:}`, the empty dict. Where a
//! value is expected, `{` starts a dict when one of those follows it;
//! otherwise it starts a block.

use crate::parser::Parser;
use kavach_ast::*;
use kavach_lexer::{Keyword, TokenKind};

impl Parser<'_> {
    /// Whether the `{` here starts a map: `:` follows it, or a key and `:`.
    pub(crate) fn at_map(&self) -> bool {
        let next = |mut i: usize| {
            while *self.nth(i) == TokenKind::Newline {
                i += 1;
            }
            i
        };
        let at = next(1);
        let key_end = match self.nth(at) {
            TokenKind::Colon => return *self.nth(next(at + 1)) == TokenKind::RBrace,
            TokenKind::Int(_) | TokenKind::BigInt | TokenKind::Float | TokenKind::Char(_) => at + 1,
            TokenKind::Keyword(Keyword::True | Keyword::False) => at + 1,
            TokenKind::Minus => match self.nth(at + 1) {
                TokenKind::Int(_) | TokenKind::BigInt | TokenKind::Float => at + 2,
                _ => return false,
            },
            // `{rows: for …}` is a block whose loop has a label (4.6).
            TokenKind::Ident => {
                let after = next(at + 2);
                let loop_follows = matches!(
                    self.nth(after),
                    TokenKind::Keyword(Keyword::For | Keyword::While)
                ) || (*self.nth(after) == TokenKind::Ident
                    && self.nth_text(after) == "on");
                if loop_follows {
                    return false;
                }
                at + 1
            }
            TokenKind::StrStart => self.after_group(at, &TokenKind::StrStart, &TokenKind::StrEnd),
            TokenKind::LBracket => self.after_group(at, &TokenKind::LBracket, &TokenKind::RBracket),
            // A tuple key, `{(x, 1): …}`.
            TokenKind::LParen => self.after_group(at, &TokenKind::LParen, &TokenKind::RParen),
            _ => return false,
        };
        *self.nth(key_end) == TokenKind::Colon
    }

    /// The place just after the group that opens at `at` with `open` and
    /// closes with the matching `close`.
    fn after_group(&self, at: usize, open: &TokenKind, close: &TokenKind) -> usize {
        let mut depth = 0;
        let mut i = at;
        loop {
            let t = self.nth(i);
            if *t == TokenKind::Eof {
                return i;
            }
            if t == open {
                depth += 1;
            } else if t == close {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            i += 1;
        }
    }

    /// `{key: value, …}` or `{:}`, at its `{`.
    pub(crate) fn map_literal(&mut self) -> ExprKind {
        self.bump();
        self.skip_newlines();
        if self.at(&TokenKind::Colon) {
            self.bump();
            self.skip_newlines();
            self.expect(&TokenKind::RBrace, "`}` after `{:`, the empty dict");
            return ExprKind::Map(Vec::new());
        }
        let mut entries = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&TokenKind::RBrace) || self.at(&TokenKind::Eof) {
                break;
            }
            let key = self.literal_key();
            self.expect(&TokenKind::Colon, "`:` between key and value");
            self.skip_newlines();
            entries.push((key, self.expr()));
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.skip_newlines();
        self.expect(&TokenKind::RBrace, "`}` to close the dict");
        ExprKind::Map(entries)
    }

    /// Skips to the `]` that closes the computed key, past nested ones.
    fn skip_to_bracket(&mut self) {
        let mut depth = 0;
        while !self.at(&TokenKind::Eof) && !(depth == 0 && self.at(&TokenKind::RBracket)) {
            if self.at(&TokenKind::LBracket) {
                depth += 1;
            } else if self.at(&TokenKind::RBracket) {
                depth -= 1;
            }
            self.bump();
        }
    }

    /// A key: a plain name is that text, `[expr]` is the value of `expr`,
    /// anything else is a value written as one.
    fn literal_key(&mut self) -> Expr {
        if self.at(&TokenKind::Ident) && *self.nth(1) == TokenKind::Colon {
            let name = self.ident("a key");
            return Expr {
                kind: ExprKind::Str(vec![StrPart::Text(name.name)]),
                span: name.span,
            };
        }
        if self.eat(&TokenKind::LBracket) {
            let key = self.expr();
            if self.at(&TokenKind::Comma) {
                // `{["red", "big"]: 3}`: brackets hold one expression (2.2).
                self.error_here("a key in brackets is the value of one expression: a list as a key goes in a second pair of brackets, `{[[\"red\", \"big\"]]: 3}` (spec 2.2)");
                self.skip_to_bracket();
            }
            self.expect(&TokenKind::RBracket, "`]` after a computed key");
            return key;
        }
        self.expr()
    }
}
