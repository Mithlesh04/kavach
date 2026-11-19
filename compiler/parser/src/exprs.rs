// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Operators and postfix chains (spec 1.8): `catch` is loosest, then the
//! binary precedence table, then unary operators, then calls and fields.

use crate::parser::Parser;
use kavach_ast::{Arg, BinaryOp, Block, Expr, ExprKind, Ident, Span, Stmt, StmtKind, UnaryOp};
use kavach_lexer::{Keyword, TokenKind};

const COMPARISON: u8 = 4;
const RANGE: u8 = 5;

/// Spec 1.8, loosest first: `??`, `or`, `and`, comparisons, ranges, `|`,
/// `^`, `&`, shifts, `+ -`, `* / %`.
fn binary_op(kind: &TokenKind) -> Option<(BinaryOp, u8)> {
    use TokenKind::*;
    Some(match kind {
        QuestionQuestion => (BinaryOp::Default, 1),
        Keyword(self::Keyword::Or) => (BinaryOp::Or, 2),
        Keyword(self::Keyword::And) => (BinaryOp::And, 3),
        EqEq => (BinaryOp::Eq, COMPARISON),
        NotEq => (BinaryOp::NotEq, COMPARISON),
        Lt => (BinaryOp::Lt, COMPARISON),
        Le => (BinaryOp::Le, COMPARISON),
        Gt => (BinaryOp::Gt, COMPARISON),
        Ge => (BinaryOp::Ge, COMPARISON),
        DotDot => (BinaryOp::Range, RANGE),
        DotDotEq => (BinaryOp::RangeInclusive, RANGE),
        Pipe => (BinaryOp::BitOr, 6),
        Caret => (BinaryOp::BitXor, 7),
        Amp => (BinaryOp::BitAnd, 8),
        Shl => (BinaryOp::Shl, 9),
        Shr => (BinaryOp::Shr, 9),
        Plus => (BinaryOp::Add, 10),
        Minus => (BinaryOp::Sub, 10),
        Star => (BinaryOp::Mul, 11),
        Slash => (BinaryOp::Div, 11),
        Percent => (BinaryOp::Rem, 11),
        At => (BinaryOp::MatMul, 11),
        _ => return None,
    })
}

impl Parser<'_> {
    pub(crate) fn expr(&mut self) -> Expr {
        let start = self.span();
        let mut expr = self.binary(0);
        while self.eat_kw(Keyword::Catch) {
            let (binding, handler) =
                if self.at(&TokenKind::Ident) && *self.nth(1) == TokenKind::FatArrow {
                    let name = self.ident("a name for the error");
                    self.bump();
                    (Some(name), self.expr())
                } else {
                    (None, self.binary(0))
                };
            expr = Expr {
                kind: ExprKind::Catch {
                    expr: Box::new(expr),
                    binding,
                    handler: Box::new(handler),
                },
                span: self.span_from(start),
            };
        }
        expr
    }

    fn binary(&mut self, min: u8) -> Expr {
        let start = self.span();
        let mut lhs = self.unary();
        let mut last_prec = None;
        while let Some((op, prec)) = binary_op(self.kind()) {
            if prec < min {
                break;
            }
            if last_prec == Some(prec) && prec == RANGE {
                self.error_here("ranges can't be chained: use `and` or parentheses");
            }
            let chained = last_prec == Some(prec) && prec == COMPARISON;
            last_prec = Some(prec);
            self.bump();
            // `a ?? b ?? c` is `a ?? (b ?? c)` (spec 1.8).
            let rhs = self.binary(if op == BinaryOp::Default {
                prec
            } else {
                prec + 1
            });
            // `a < b < c` (spec 1.8): the comparisons so far, one more.
            let kind = match lhs.kind {
                ExprKind::Binary(first, a, b) if chained => {
                    ExprKind::Compare(a, vec![(first, *b), (op, rhs)])
                }
                ExprKind::Compare(a, mut links) if chained => {
                    links.push((op, rhs));
                    ExprKind::Compare(a, links)
                }
                kind => ExprKind::Binary(
                    op,
                    Box::new(Expr {
                        kind,
                        span: lhs.span,
                    }),
                    Box::new(rhs),
                ),
            };
            lhs = Expr {
                kind,
                span: self.span_from(start),
            };
        }
        lhs
    }

    pub(crate) fn unary(&mut self) -> Expr {
        let start = self.span();
        let op = match self.kind() {
            TokenKind::Minus => UnaryOp::Neg,
            TokenKind::Keyword(Keyword::Not) => UnaryOp::Not,
            TokenKind::Tilde => UnaryOp::BitNot,
            TokenKind::Star => UnaryOp::Deref,
            _ => return self.prefix_keyword(),
        };
        self.bump();
        let operand = self.unary();
        Expr {
            kind: ExprKind::Unary(op, Box::new(operand)),
            span: self.span_from(start),
        }
    }

    /// `try`, `fail`, `return`, `break`, `continue` and `spawn`.
    fn prefix_keyword(&mut self) -> Expr {
        let start = self.span();
        let kind = match self.kind() {
            TokenKind::Keyword(Keyword::Try) => {
                self.bump();
                let expr = Box::new(self.unary());
                let context = self.at_word("context").then(|| {
                    self.bump();
                    Box::new(self.unary())
                });
                ExprKind::Try { expr, context }
            }
            TokenKind::Keyword(Keyword::Fail) => {
                self.bump();
                ExprKind::Fail(Box::new(self.expr()))
            }
            TokenKind::Keyword(Keyword::Return) => {
                self.bump();
                let ends = self.at_end_of_line()
                    || matches!(self.kind(), TokenKind::RParen | TokenKind::Comma);
                ExprKind::Return((!ends).then(|| Box::new(self.expr())))
            }
            TokenKind::Keyword(Keyword::Break) => {
                self.bump();
                ExprKind::Break(self.label_after_jump())
            }
            TokenKind::Keyword(Keyword::Continue) => {
                self.bump();
                ExprKind::Continue(self.label_after_jump())
            }
            TokenKind::Keyword(Keyword::Spawn) => {
                self.bump();
                let kind = self.spawn_kind();
                let x = Box::new(self.unary());
                // `spawn work() within 5.seconds` (spec 9.6).
                let within = self.at_word("within").then(|| {
                    self.bump();
                    Box::new(self.unary())
                });
                ExprKind::Spawn(kind, x, within)
            }
            TokenKind::Ident if self.at_value_place() => self.value_place(),
            _ => {
                let primary = self.primary();
                let base = self.postfix(primary);
                return self.power(base, start);
            }
        };
        Expr {
            kind,
            span: self.span_from(start),
        }
    }

    /// `base ** exponent` (spec 1.8): tighter than a minus on its left
    /// (`-2 ** 2` is `-(2 ** 2)`), and right to left, since the exponent
    /// is a unary expression that may hold another `**`.
    fn power(&mut self, base: Expr, start: Span) -> Expr {
        if !self.at(&TokenKind::StarStar) {
            return base;
        }
        self.bump();
        let exponent = self.unary();
        Expr {
            kind: ExprKind::Binary(BinaryOp::Pow, Box::new(base), Box::new(exponent)),
            span: self.span_from(start),
        }
    }

    fn postfix(&mut self, mut expr: Expr) -> Expr {
        let start = expr.span;
        loop {
            let kind = match self.kind() {
                TokenKind::Dot | TokenKind::QuestionDot => {
                    let optional = self.at(&TokenKind::QuestionDot);
                    let dot = self.span();
                    self.bump();
                    if let (TokenKind::Int(index), false) = (self.kind().clone(), optional) {
                        self.bump();
                        ExprKind::TupleField {
                            base: Box::new(expr),
                            index,
                        }
                    } else {
                        let name = self.ident("a field or method name after `.`");
                        // `x?.m(a)` (spec 2.7): `if let v = x { v.m(a) } else { none }`.
                        if optional && self.at(&TokenKind::LParen) {
                            expr = self.optional_call(expr, name, dot, start);
                            continue;
                        }
                        // `pg.sql"…"` (spec 10.15).
                        match (&expr.kind, optional) {
                            (ExprKind::Name(module), false) if self.tag_follows(name.span) => {
                                self.tagged(vec![module.clone(), name])
                            }
                            _ => ExprKind::Field {
                                base: Box::new(expr),
                                name,
                                optional,
                            },
                        }
                    }
                }
                TokenKind::LParen => {
                    let call = ExprKind::Call {
                        callee: Box::new(expr),
                        generics: Vec::new(),
                        args: self.args(),
                    };
                    self.scoped(call, start)
                }
                TokenKind::Lt if self.generic_call_follows(&expr) => {
                    let generics = self.type_args();
                    ExprKind::Call {
                        callee: Box::new(expr),
                        generics,
                        args: self.args(),
                    }
                }
                TokenKind::LBracket => {
                    self.bump();
                    let index = self.expr();
                    self.expect(&TokenKind::RBracket, "`]`");
                    ExprKind::Index {
                        base: Box::new(expr),
                        index: Box::new(index),
                    }
                }
                TokenKind::Keyword(Keyword::With) => {
                    self.bump();
                    ExprKind::With {
                        base: Box::new(expr),
                        fields: self.with_fields(),
                    }
                }
                _ => return expr,
            };
            expr = Expr {
                kind,
                span: self.span_from(start),
            };
        }
    }

    /// A block given to `log.span(…)` or `random.seeded(…)` on the same
    /// line (spec 4.8); any other call is left as it is, so `if f(x) {`
    /// keeps its meaning. The checker makes sure they are the standard
    /// library's.
    fn scoped(&mut self, call: ExprKind, start: Span) -> ExprKind {
        let ExprKind::Call { callee, .. } = &call else {
            return call;
        };
        let takes_block = match &callee.kind {
            ExprKind::Field { base, name, .. } => {
                matches!(
                    (&base.kind, name.name.as_str()),
                    (ExprKind::Name(m), "span") if m.name == "log"
                ) || matches!(
                    (&base.kind, name.name.as_str()),
                    (ExprKind::Name(m), "seeded") if m.name == "random"
                )
            }
            _ => false,
        };
        if !takes_block || !self.at(&TokenKind::LBrace) {
            return call;
        }
        let call = Expr {
            kind: call,
            span: self.span_from(start),
        };
        ExprKind::Scoped {
            call: Box::new(call),
            body: self.block(),
        }
    }

    /// Whether `<` starts type arguments of a call (`ask<Decision>(…)`) rather
    /// than a comparison: the angle brackets must hold only type syntax and
    /// be followed by `(`.
    fn generic_call_follows(&self, callee: &Expr) -> bool {
        use TokenKind::*;
        if !matches!(callee.kind, ExprKind::Name(_) | ExprKind::Field { .. }) {
            return false;
        }
        let mut depth = 0i32;
        let start = self.position();
        for i in start..start + 64 {
            match self.token_at(i) {
                Lt => depth += 1,
                Gt => depth -= 1,
                Shr => depth -= 2,
                Ident | Comma | Dot | Question | LBracket | RBracket | LBrace | RBrace | Colon
                | Arrow => {}
                Keyword(self::Keyword::Fn) | LParen | RParen if depth > 0 => {}
                _ => return false,
            }
            if depth <= 0 {
                return depth == 0 && *self.token_at(i + 1) == LParen;
            }
        }
        false
    }

    /// `(a, name: b, .field > 2)`. Positional arguments come first (spec 3.2).
    pub(crate) fn args(&mut self) -> Vec<Arg> {
        self.bump();
        let mut args: Vec<Arg> = Vec::new();
        while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
            let start = self.span();
            let name =
                (self.at(&TokenKind::Ident) && *self.nth(1) == TokenKind::Colon).then(|| {
                    let name = self.ident("an argument name");
                    self.bump();
                    name
                });
            if name.is_none() && args.iter().any(|a| a.name.is_some()) {
                self.error(start, "positional arguments must come before named ones");
            }
            let value = self.arg_value();
            args.push(Arg { name, value });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "`)` to close the arguments");
        args
    }

    fn arg_value(&mut self) -> Expr {
        if !self.at(&TokenKind::Dot) {
            return self.expr();
        }
        let start = self.span();
        self.dot_lambda += 1;
        let body = self.expr();
        self.dot_lambda -= 1;
        Expr {
            kind: ExprKind::DotLambda(Box::new(body)),
            span: self.span_from(start),
        }
    }

    fn with_fields(&mut self) -> Vec<(kavach_ast::Ident, Expr)> {
        let mut fields = Vec::new();
        if !self.expect(&TokenKind::LBrace, "`{` after `with`") {
            return fields;
        }
        loop {
            self.skip_newlines();
            if self.at(&TokenKind::RBrace) || self.at(&TokenKind::Eof) {
                break;
            }
            let name = self.ident("a field name");
            self.expect(&TokenKind::Colon, "`:` and the new value");
            fields.push((name, self.expr()));
            if !self.eat(&TokenKind::Comma) && !self.at(&TokenKind::Newline) {
                break;
            }
        }
        self.skip_newlines();
        self.expect(&TokenKind::RBrace, "`}` to close `with`");
        fields
    }

    /// `base?.name(args)` (spec 2.7), the arguments next: written as
    /// `if let v = base { v.name(args) } else { none }`. The value's name
    /// (`?`) can't be written, and it and its use take the places of the
    /// `?` and the `.` of `dot`, which no other name has.
    fn optional_call(&mut self, base: Expr, name: Ident, dot: Span, start: Span) -> Expr {
        let at = |k: u32| Span {
            start: dot.start + k,
            end: dot.start + k + 1,
        };
        let value = |k: u32| Ident {
            name: "?".into(),
            span: at(k),
        };
        let args = self.args();
        let end = self.span_from(start);
        // The call inside gets a place of its own: types are kept by place,
        // and the call's isn't the whole's, which is optional.
        let inner = Span {
            start: name.span.start,
            end: end.end,
        };
        let method = name.span;
        let receiver = Expr {
            kind: ExprKind::Name(value(1)),
            span: at(1),
        };
        let call = Expr {
            kind: ExprKind::Call {
                callee: Box::new(Expr {
                    kind: ExprKind::Field {
                        base: Box::new(receiver),
                        name,
                        optional: false,
                    },
                    span: method,
                }),
                generics: Vec::new(),
                args,
            },
            span: inner,
        };
        let then = Block {
            stmts: vec![Stmt {
                kind: StmtKind::Expr(call),
                span: inner,
            }],
            span: inner,
        };
        let none = Expr {
            kind: ExprKind::None,
            span: at(0),
        };
        let otherwise = Expr {
            kind: ExprKind::Block(Block {
                stmts: vec![Stmt {
                    kind: StmtKind::Expr(none),
                    span: at(0),
                }],
                span: at(0),
            }),
            span: at(0),
        };
        Expr {
            kind: ExprKind::IfLet {
                name: value(0),
                value: Box::new(base),
                then,
                otherwise: Some(Box::new(otherwise)),
            },
            span: end,
        }
    }
}
