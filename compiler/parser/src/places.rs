// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What an assignment can set (spec 1.4, 1.14): a place, or a tuple of
//! places, and Python's `a, b = b, a`, which gets a message showing how
//! Kavach writes it.

use crate::parser::Parser;
use kavach_ast::{AssignOp, Expr, ExprKind, UnaryOp};
use kavach_lexer::TokenKind;

impl Parser<'_> {
    /// Checks the left side of `target op value`.
    pub(crate) fn check_target(&mut self, target: &Expr, op: AssignOp) {
        if matches!(target.kind, ExprKind::Tuple(_)) && op != AssignOp::Set {
            self.error(
                target.span,
                "only `=` sets several places at once: write each `op=` on its own line (spec 1.14)",
            );
        }
        self.check_place(target);
    }

    fn check_place(&mut self, e: &Expr) {
        match &e.kind {
            // `(a, b) = (b, a)` (spec 1.14), nested tuples too.
            ExprKind::Tuple(parts) => parts.iter().for_each(|p| self.check_place(p)),
            ExprKind::Name(_)
            | ExprKind::Field { .. }
            | ExprKind::TupleField { .. }
            | ExprKind::Index { .. }
            | ExprKind::Unary(UnaryOp::Deref, _)
            | ExprKind::Error => {}
            _ => self.error(e.span, "only a variable, field or index can be assigned to"),
        }
    }

    /// `xs[i], xs[j] = xs[j], xs[i]`, at the `,` after `target`: when a `=`
    /// follows on the line, says how Kavach writes it, skips the line and
    /// gives `true`.
    pub(crate) fn python_assign(&mut self, target: &Expr) -> bool {
        let (Some(eq), Some(end)) = (self.ahead(true), self.ahead(false)) else {
            return false;
        };
        // The last token on the line, so a comment after it is left out.
        let (eq, last) = (self.nth_span(eq), self.nth_span(end - 1));
        let left = self.src[target.span.start as usize..eq.start as usize].trim();
        let right = self.src[eq.end as usize..last.end.max(eq.end) as usize].trim();
        let mut message = format!(
            "Kavach writes several places in parentheses: `({left}) = ({right})` (spec 1.14)"
        );
        if let ExprKind::Index { base, .. } = &target.kind {
            let list = &self.src[base.span.start as usize..base.span.end as usize];
            message +=
                &format!("; two items of a list also swap with `{list}.swap(i, j)` (spec 2.2)");
        }
        self.error(target.span, message);
        self.recover_line();
        true
    }

    /// How many tokens ahead, at this depth, the first `=` is (`eq`), or the
    /// end of the line; `None` when `eq` and the line ends first.
    fn ahead(&self, eq: bool) -> Option<usize> {
        use TokenKind::*;
        let mut depth = 0usize;
        let mut k = 0;
        loop {
            match self.nth(k) {
                Eof => return (!eq).then_some(k),
                Newline if depth == 0 => return (!eq).then_some(k),
                Eq if depth == 0 && eq => return Some(k),
                LParen | LBracket | LBrace | InterpStart | StrStart => depth += 1,
                RBrace | InterpEnd | StrEnd if depth == 0 => return (!eq).then_some(k),
                RParen | RBracket if depth == 0 => return (!eq).then_some(k),
                RParen | RBracket | RBrace | InterpEnd | StrEnd => depth -= 1,
                _ => {}
            }
            k += 1;
        }
    }
}
