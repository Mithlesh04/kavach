// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What is written inside `x[ … ]` (spec 2.19): an index, `#k` from the
//! end, a slice `start:end:step` with each part optional, or one part per
//! dimension, separated by commas.

use crate::parser::Parser;
use kavach_ast::{Expr, ExprKind};
use kavach_lexer::TokenKind;

impl Parser<'_> {
    /// The index of `x[ … ]`, after the `[`; the `]` is left for the caller.
    pub(crate) fn index_inside(&mut self) -> Expr {
        let start = self.span();
        let mut parts = vec![self.index_part()];
        while self.eat(&TokenKind::Comma) {
            parts.push(self.index_part());
        }
        if parts.len() == 1 {
            return parts.pop().expect("one part");
        }
        Expr {
            kind: ExprKind::Dims(parts),
            span: self.span_from(start),
        }
    }

    /// One part: `i`, `#k`, or `start:end:step`.
    fn index_part(&mut self) -> Expr {
        let start = self.span();
        let first = (!self.at(&TokenKind::Colon)).then(|| self.bound());
        if !self.at(&TokenKind::Colon) {
            return first.unwrap_or_else(|| Expr {
                kind: ExprKind::Error,
                span: self.span_from(start),
            });
        }
        self.bump();
        let end = (!self.at_part_end()).then(|| Box::new(self.bound()));
        let step = if self.eat(&TokenKind::Colon) {
            (!self.at_part_end()).then(|| Box::new(self.expr()))
        } else {
            None
        };
        Expr {
            kind: ExprKind::Part {
                start: first.map(Box::new),
                end,
                step,
            },
            span: self.span_from(start),
        }
    }

    fn at_part_end(&self) -> bool {
        matches!(
            self.kind(),
            TokenKind::Colon | TokenKind::Comma | TokenKind::RBracket
        )
    }

    /// An index or an end of a slice: `#k` covers the whole part after
    /// it, up to `:`, `,` or `]`.
    fn bound(&mut self) -> Expr {
        let start = self.span();
        if !self.eat(&TokenKind::Hash) {
            return self.expr();
        }
        let k = self.expr();
        Expr {
            kind: ExprKind::FromEnd(Box::new(k)),
            span: self.span_from(start),
        }
    }
}
