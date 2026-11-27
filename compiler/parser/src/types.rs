// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Types and generic parameters (spec 2), and whether a `<` after a
//! name starts the type arguments of a call.

use crate::parser::Parser;
use kavach_ast::{Expr, ExprKind, GenericParam, Path, TypeExpr, TypeKind};
use kavach_lexer::{Keyword, TokenKind};

impl Parser<'_> {
    pub(crate) fn type_expr(&mut self) -> TypeExpr {
        let start = self.span();
        let kind = match self.kind() {
            // `{String: Int}`, a dict (spec 2.2, decision 0130).
            TokenKind::LBrace => {
                self.bump();
                let key = Box::new(self.type_expr());
                self.expect(
                    &TokenKind::Colon,
                    "`:` between the key's and the value's types",
                );
                let value = Box::new(self.type_expr());
                self.expect(&TokenKind::RBrace, "`}`");
                self.with_nums(TypeKind::Map(key, value), start)
            }
            TokenKind::LBracket => {
                let open = self.span();
                self.bump();
                let item = Box::new(self.type_expr());
                let kind = if self.eat(&TokenKind::Colon) {
                    self.error(
                        open,
                        "a dict's type is written with braces: `{String: Int}` (spec 2.2)",
                    );
                    TypeKind::Map(item, Box::new(self.type_expr()))
                } else {
                    TypeKind::List(item)
                };
                // `[Int, max: 2]` reads as a list of two items: the number
                // goes after the type (spec 2.2, decision 0137).
                if self.at(&TokenKind::Comma) {
                    let at = self.span();
                    let mut nums = Vec::new();
                    while self.eat(&TokenKind::Comma) && !self.at(&TokenKind::RBracket) {
                        nums.push(self.num_arg());
                    }
                    let shown = nums.iter().map(show_num).collect::<Vec<_>>().join(", ");
                    let item = match &kind {
                        TypeKind::List(item) => show_type(item),
                        _ => "T".to_string(),
                    };
                    self.error(
                        at,
                        format!("a list's numbers go after its type: write `[{item}]<{shown}>` (spec 2.2)"),
                    );
                }
                self.expect(&TokenKind::RBracket, "`]`");
                self.with_nums(kind, start)
            }
            TokenKind::LParen
                if *self.nth(1) == TokenKind::Ident && *self.nth(2) == TokenKind::Colon =>
            {
                self.bump();
                let mut fields = Vec::new();
                while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
                    let name = self.ident("a field name");
                    self.expect(&TokenKind::Colon, "`:` and the field's type");
                    fields.push((name, self.type_expr()));
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen, "`)`");
                TypeKind::AnonRecord(fields)
            }
            TokenKind::LParen => TypeKind::Tuple(self.type_list()),
            TokenKind::Keyword(Keyword::Fn) => {
                self.bump();
                let params = if self.at(&TokenKind::LParen) {
                    self.type_list()
                } else {
                    self.error_here("expected `(` and parameter types after `fn`");
                    Vec::new()
                };
                let ret = self
                    .eat(&TokenKind::Arrow)
                    .then(|| Box::new(self.type_expr()));
                let errors = self
                    .eat(&TokenKind::Bang)
                    .then(|| Box::new(self.type_expr()));
                let uses = self.eat_kw(Keyword::Uses).then(|| self.fn_type_uses());
                TypeKind::Fn {
                    params,
                    ret,
                    errors,
                    uses,
                }
            }
            TokenKind::Ident => {
                let path = self.path("a type name");
                let args = if self.at(&TokenKind::Lt) {
                    self.type_args()
                } else {
                    Vec::new()
                };
                TypeKind::Named { path, args }
            }
            _ => {
                self.error_here("expected a type");
                if !self.at_end_of_line() {
                    self.bump();
                }
                TypeKind::Error
            }
        };
        let mut ty = TypeExpr {
            kind,
            span: self.span_from(start),
        };
        while self.eat(&TokenKind::Question) {
            ty = TypeExpr {
                kind: TypeKind::Optional(Box::new(ty)),
                span: self.span_from(start),
            };
        }
        ty
    }

    /// What follows `uses` in a function type (spec 3.4): one capability,
    /// or several in parentheses, so a `,` after the type still ends it.
    /// `uses ()` lists none: its functions may use nothing at all.
    fn fn_type_uses(&mut self) -> Vec<Path> {
        if !self.eat(&TokenKind::LParen) {
            return vec![self.path("a capability")];
        }
        let mut caps = Vec::new();
        while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
            caps.push(self.path("a capability"));
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "`)` after the capabilities");
        caps
    }

    /// `(A, B)`
    fn type_list(&mut self) -> Vec<TypeExpr> {
        self.bump();
        let mut types = Vec::new();
        while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
            types.push(self.type_expr());
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "`)`");
        types
    }

    /// `<A, B>` after a type or a call, maybe with numbers: `<Float,
    /// len: 3>` (spec 2.9).
    pub(crate) fn type_args(&mut self) -> Vec<TypeExpr> {
        self.bump();
        let mut args = vec![self.type_arg()];
        while self.eat(&TokenKind::Comma) {
            args.push(self.type_arg());
        }
        self.close_angle();
        args
    }

    /// A type, or a number with or without its name (`max: 2`, `2`).
    fn type_arg(&mut self) -> TypeExpr {
        let named = self.at(&TokenKind::Ident) && *self.nth(1) == TokenKind::Colon;
        if named || matches!(self.kind(), TokenKind::Int(_) | TokenKind::Minus) {
            return self.num_arg();
        }
        self.type_expr()
    }

    /// `max: 2`, or `2` alone, which the checker reports with its name.
    fn num_arg(&mut self) -> TypeExpr {
        let start = self.span();
        let name = (self.at(&TokenKind::Ident) && *self.nth(1) == TokenKind::Colon).then(|| {
            let name = self.ident("a number's name");
            self.bump();
            name
        });
        let value_start = self.span();
        let mut value = self.unary();
        // `range: 1..=65535` (spec 2.1): both ends are numbers.
        let op = match self.kind() {
            TokenKind::DotDot => Some(kavach_ast::BinaryOp::Range),
            TokenKind::DotDotEq => Some(kavach_ast::BinaryOp::RangeInclusive),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let end = self.unary();
            value = kavach_ast::Expr {
                kind: kavach_ast::ExprKind::Binary(op, Box::new(value), Box::new(end)),
                span: self.span_from(value_start),
            };
        }
        let value = Box::new(value);
        TypeExpr {
            kind: TypeKind::Num { name, value },
            span: self.span_from(start),
        }
    }

    /// `[Int]<max: 2>`, `{K: V}<max: 100>`: numbers after a list's or a
    /// dict's type (spec 2.2).
    fn with_nums(&mut self, kind: TypeKind, start: kavach_ast::Span) -> TypeKind {
        if !self.at(&TokenKind::Lt) {
            return kind;
        }
        let base = TypeExpr {
            kind,
            span: self.span_from(start),
        };
        let nums = self.type_args();
        TypeKind::WithNums(Box::new(base), nums)
    }

    /// `<T, U: Comparable>` after a function or type name.
    pub(crate) fn generic_params(&mut self) -> Vec<GenericParam> {
        let mut params = Vec::new();
        if !self.eat(&TokenKind::Lt) {
            return params;
        }
        loop {
            let name = self.ident("a type parameter name");
            let mut bounds = Vec::new();
            if self.eat(&TokenKind::Colon) {
                bounds.push(self.type_expr());
                while self.eat(&TokenKind::Plus) {
                    bounds.push(self.type_expr());
                }
            }
            // `size: Int = 64`: a number parameter's default (spec 2.9).
            let default = self.eat(&TokenKind::Eq).then(|| self.unary());
            params.push(GenericParam {
                name,
                bounds,
                default,
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.close_angle();
        params
    }

    /// Whether `<` starts type arguments of a call (`ask<Decision>(…)`) rather
    /// than a comparison: the angle brackets must hold only type syntax and
    /// be followed by `(`.
    pub(crate) fn generic_call_follows(&self, callee: &Expr) -> bool {
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
                // `zeros<n: 4>()`, `List<Int, max: 2>(xs)`: a named number.
                // `Int<range: -5..=5>(x)`: a range of numbers (spec 2.1).
                Int(_) | Minus
                    if depth > 0
                        && matches!(
                            self.token_at(i - 1),
                            Colon | Comma | Lt | DotDot | DotDotEq
                        ) => {}
                Int(_) if depth > 0 && matches!(self.token_at(i - 1), Minus) => {}
                DotDot | DotDotEq
                    if depth > 0 && matches!(self.token_at(i - 1), Int(_) | Ident) => {}
                _ => return false,
            }
            if depth <= 0 {
                let then = |k: usize| self.token_at(i + k).clone();
                let named = matches!(callee.kind, ExprKind::Name(_));
                let function = named && (then(1), then(2), then(3)) == (Dot, Ident, LParen);
                return depth == 0 && (then(1) == LParen || function);
            }
        }
        false
    }

    /// `: A, B` after a type name: the interfaces it implements.
    pub(crate) fn implements(&mut self) -> Vec<TypeExpr> {
        let mut types = Vec::new();
        if self.eat(&TokenKind::Colon) {
            types.push(self.type_expr());
            while self.eat(&TokenKind::Comma) {
                types.push(self.type_expr());
            }
        }
        types
    }
}

/// A number argument as written, for messages.
pub(crate) fn show_num(t: &TypeExpr) -> String {
    match &t.kind {
        TypeKind::Num { name, value } => {
            let v = match &value.kind {
                kavach_ast::ExprKind::Int(n) => n.to_string(),
                kavach_ast::ExprKind::Name(id) => id.name.clone(),
                _ => "…".to_string(),
            };
            match name {
                Some(n) => format!("{}: {v}", n.name),
                None => v,
            }
        }
        _ => show_type(t),
    }
}

/// A simple type as written, for messages.
pub(crate) fn show_type(t: &TypeExpr) -> String {
    match &t.kind {
        TypeKind::Named { path, args } => {
            let name: Vec<&str> = path.segments.iter().map(|s| s.name.as_str()).collect();
            let args: Vec<String> = args.iter().map(show_num).collect();
            match args.is_empty() {
                true => name.join("."),
                false => format!("{}<{}>", name.join("."), args.join(", ")),
            }
        }
        TypeKind::List(t) => format!("[{}]", show_type(t)),
        _ => "T".to_string(),
    }
}
