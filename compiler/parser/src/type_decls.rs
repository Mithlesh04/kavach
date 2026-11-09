//! `type` declarations: unions (`type Status = Pending | Paid`) and records
//! with fields, invariants and methods (spec 2.4-2.6, 7.3).

use crate::parser::Parser;
use kavach_ast::*;
use kavach_lexer::{Keyword, TokenKind};

impl Parser<'_> {
    pub(crate) fn type_decl(&mut self) -> TypeDecl {
        self.bump();
        let name = self.ident("a type name");
        let generics = self.generic_params();
        let implements = self.implements();
        let body = if self.eat(&TokenKind::Eq) {
            if self.at(&TokenKind::LParen) {
                return TypeDecl {
                    name,
                    generics,
                    implements,
                    body: TypeBody::Alias(self.type_expr()),
                };
            }
            let mut variants = vec![self.variant()];
            while self.eat(&TokenKind::Pipe) {
                variants.push(self.variant());
            }
            TypeBody::Union(variants)
        } else if self.at(&TokenKind::LBrace) {
            TypeBody::Record(self.record_body())
        } else {
            self.error_here("expected `=` and variants, or `{` and fields");
            TypeBody::Union(Vec::new())
        };
        TypeDecl {
            name,
            generics,
            implements,
            body,
        }
    }

    fn variant(&mut self) -> Variant {
        let name = self.ident("a variant name");
        let mut fields = Vec::new();
        if self.eat(&TokenKind::LParen) {
            while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
                fields.push(self.field(Vec::new(), false, false));
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::RParen, "`)`");
        }
        // `NotFound = 404`: an enum variant's number (spec 2.6).
        let mut value = None;
        if self.eat(&TokenKind::Eq) {
            let negative = self.eat(&TokenKind::Minus);
            match self.kind().clone() {
                TokenKind::Int(n) => {
                    self.bump();
                    value = Some(if negative { -(n as i128) } else { n as i128 });
                }
                _ => self.error_here("expected a whole number for the variant"),
            }
        }
        Variant {
            name,
            fields,
            value,
        }
    }

    fn field(&mut self, docs: Vec<String>, is_pub: bool, is_secret: bool) -> Field {
        let name = self.ident("a field name");
        // `summary?: String`: may be left out, so `String? = none` (2.4).
        let may_skip = self.eat(&TokenKind::Question);
        self.expect(&TokenKind::Colon, "`:` and the field's type");
        let mut ty = self.type_expr();
        let mut default = self.eat(&TokenKind::Eq).then(|| self.expr());
        if may_skip {
            if matches!(ty.kind, TypeKind::Optional(_)) {
                let message = format!(
                    "`{}?` already makes the field optional: write the type without `?`",
                    name.name
                );
                self.error(ty.span, message);
            } else {
                let span = ty.span;
                ty = TypeExpr {
                    kind: TypeKind::Optional(Box::new(ty)),
                    span,
                };
            }
            if let Some(d) = &default {
                let message = format!(
                    "`{0}?` is `none` when left out: for another default, write `{0}: T? = …`",
                    name.name
                );
                self.error(d.span, message);
            }
            default = Some(Expr {
                kind: ExprKind::None,
                span: name.span,
            });
        }
        Field {
            name,
            ty,
            default,
            is_pub,
            is_secret,
            docs,
        }
    }

    fn record_body(&mut self) -> RecordBody {
        let mut body = RecordBody {
            fields: Vec::new(),
            methods: Vec::new(),
            invariants: Vec::new(),
        };
        self.members(|p| {
            let start = p.span();
            let docs = p.docs();
            let is_pub = p.eat_kw(Keyword::Pub);
            let is_secret = p.eat_kw(Keyword::Secret);
            match p.kind() {
                TokenKind::Keyword(Keyword::Fn) => {
                    let kind = ItemKind::Fn(p.fn_decl(true));
                    body.methods.push(Item {
                        kind,
                        docs,
                        is_pub,
                        span: p.span_from(start),
                    });
                }
                TokenKind::Keyword(Keyword::Invariant) => {
                    p.bump();
                    body.invariants.push(p.expr());
                }
                TokenKind::Ident => body.fields.push(p.field(docs, is_pub, is_secret)),
                _ => {
                    let at = p.position();
                    let keyword = match p.token_at(at) {
                        TokenKind::Keyword(k) if *p.token_at(at + 1) == TokenKind::Colon => {
                            Some(k.as_str())
                        }
                        _ => None,
                    };
                    match keyword {
                        Some(k) => p.error_here(format!(
                            "`{k}` is a keyword, so it can't name a field: pick another name"
                        )),
                        None => p.error_here("expected a field, a method or an invariant"),
                    }
                    p.recover_line();
                }
            }
        });
        body
    }
}
