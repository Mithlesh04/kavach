//! Types and generic parameters (spec 2).

use crate::parser::Parser;
use kavach_ast::{GenericParam, Path, TypeExpr, TypeKind};
use kavach_lexer::{Keyword, TokenKind};

impl Parser<'_> {
    pub(crate) fn type_expr(&mut self) -> TypeExpr {
        let start = self.span();
        let kind = match self.kind() {
            TokenKind::LBracket => {
                self.bump();
                let item = Box::new(self.type_expr());
                let kind = if self.eat(&TokenKind::Colon) {
                    TypeKind::Map(item, Box::new(self.type_expr()))
                } else {
                    TypeKind::List(item)
                };
                self.expect(&TokenKind::RBracket, "`]`");
                kind
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
        if caps.is_empty() {
            self.error_here("name a capability, or leave `uses` out");
        }
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

    /// `<A, B>` after a type or a call.
    pub(crate) fn type_args(&mut self) -> Vec<TypeExpr> {
        self.bump();
        let mut args = vec![self.type_expr()];
        while self.eat(&TokenKind::Comma) {
            args.push(self.type_expr());
        }
        self.close_angle();
        args
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
            params.push(GenericParam { name, bounds });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.close_angle();
        params
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
