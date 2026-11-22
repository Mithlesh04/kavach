// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
            // `type NetId = distinct Int` (spec 2.21).
            if self.at_word("distinct")
                && matches!(
                    self.nth(1),
                    TokenKind::Ident | TokenKind::LBracket | TokenKind::LBrace
                )
            {
                let span = self.span();
                self.bump();
                let field = Field {
                    name: Ident {
                        name: "value".to_string(),
                        span,
                    },
                    ty: self.type_expr(),
                    default: None,
                    is_pub: true,
                    is_read: false,
                    is_secret: false,
                    docs: Vec::new(),
                };
                let body = RecordBody {
                    fields: vec![field],
                    methods: Vec::new(),
                    invariants: Vec::new(),
                    distinct: true,
                };
                return TypeDecl {
                    name,
                    generics,
                    implements,
                    body: TypeBody::Record(body),
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
        // `"IN" | "US"`, as TypeScript writes a union (spec 2.6).
        if self.at(&TokenKind::StrStart) {
            return self.text_variant();
        }
        let mut name = self.ident("a variant name");
        // `U.S` or `New Zealand`: text that isn't one name (spec 2.6).
        if matches!(self.kind(), TokenKind::Dot | TokenKind::Ident) {
            name = self.spaced_variant(name);
        }
        let mut fields = Vec::new();
        if self.eat(&TokenKind::LParen) {
            while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
                fields.push(self.field(Vec::new(), (false, false), false));
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::RParen, "`)`");
        }
        // `NotFound = 404`, `Get = "GET"`: an enum variant's value (2.6).
        let value = self
            .eat(&TokenKind::Eq)
            .then(|| self.variant_value())
            .flatten();
        Variant {
            name,
            fields,
            value,
        }
    }

    /// A variant's value: a whole number, or plain text in quotes (or in
    /// backticks, with nothing filled in).
    fn variant_value(&mut self) -> Option<VariantValue> {
        let negative = self.eat(&TokenKind::Minus);
        match self.kind().clone() {
            TokenKind::Int(n) => {
                self.bump();
                Some(VariantValue::Int(if negative {
                    -(n as i128)
                } else {
                    n as i128
                }))
            }
            TokenKind::StrStart if !negative => {
                let start = self.span();
                let ExprKind::Str(parts) = self.string() else {
                    return None;
                };
                let mut text = String::new();
                for part in parts {
                    match part {
                        StrPart::Text(t) => text.push_str(&t),
                        StrPart::Expr(_) => {
                            let span = self.span_from(start);
                            self.error(span, "a variant's text is fixed when compiling: write it without `{…}` (spec 2.6)");
                            return None;
                        }
                    }
                }
                Some(VariantValue::Text(text))
            }
            _ => {
                self.error_here("expected a whole number or text in quotes for the variant");
                None
            }
        }
    }

    /// `U.S` or `New Zealand` where a variant's name goes: refused, with
    /// the name and text to write instead. Gives that name.
    fn spaced_variant(&mut self, first: Ident) -> Ident {
        let start = first.span;
        while matches!(self.kind(), TokenKind::Dot | TokenKind::Ident) {
            self.bump();
        }
        let span = self.span_from(start);
        let text = span.text(self.src).to_string();
        let name = name_for(&text);
        self.error(
            span,
            format!(
                "`{text}` is not a variant name: a name is one word, with no dots or spaces. To keep this text, write `{name} = \"{text}\"` (spec 2.6)"
            ),
        );
        Ident { name, span }
    }

    /// `"IN"` where a variant goes: refused, with the name to write.
    fn text_variant(&mut self) -> Variant {
        let start = self.span();
        let text = match self.string() {
            ExprKind::Str(parts) => parts
                .iter()
                .map(|p| match p {
                    StrPart::Text(t) => t.as_str(),
                    StrPart::Expr(_) => "",
                })
                .collect::<String>(),
            _ => String::new(),
        };
        let span = self.span_from(start);
        let name = name_for(&text);
        self.error(
            span,
            format!(
                "a variant is a name, not text: to keep the text, write `{name} = \"{text}\"` (spec 2.6)"
            ),
        );
        Variant {
            name: Ident { name, span },
            fields: Vec::new(),
            value: Some(VariantValue::Text(text)),
        }
    }

    fn field(
        &mut self,
        docs: Vec<String>,
        (is_pub, is_read): (bool, bool),
        is_secret: bool,
    ) -> Field {
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
            is_read,
            is_secret,
            docs,
        }
    }

    fn record_body(&mut self) -> RecordBody {
        let mut body = RecordBody {
            fields: Vec::new(),
            methods: Vec::new(),
            invariants: Vec::new(),
            distinct: false,
        };
        self.members(|p| {
            let start = p.span();
            let docs = p.docs();
            let is_pub = p.eat_kw(Keyword::Pub);
            let is_read = is_pub && p.pub_read();
            let is_secret = p.eat_kw(Keyword::Secret);
            match p.kind() {
                TokenKind::Keyword(Keyword::Fn) => {
                    let kind = ItemKind::Fn(p.fn_decl(true));
                    body.methods.push(Item {
                        kind,
                        docs,
                        is_pub,
                        is_out: false,
                        span: p.span_from(start),
                    });
                }
                TokenKind::Keyword(Keyword::Invariant) => {
                    p.bump();
                    body.invariants.push(p.expr());
                }
                TokenKind::Ident => body
                    .fields
                    .push(p.field(docs, (is_pub, is_read), is_secret)),
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

/// A variant name for `text`, for a message: each word capitalized,
/// short words in capitals kept (`U.S` is `US`, `"INDIA"` is `India`).
fn name_for(text: &str) -> String {
    let words =
        text.split(|c: char| c.is_whitespace() || (c.is_ascii() && !c.is_ascii_alphanumeric()));
    let mut name = String::new();
    for word in words.filter(|w| !w.is_empty()) {
        if word.len() <= 3
            && word
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            name.push_str(word);
            continue;
        }
        let mut chars = word.chars();
        if let Some(c) = chars.next() {
            name.extend(c.to_uppercase());
            name.extend(chars.flat_map(char::to_lowercase));
        }
    }
    match name.chars().next() {
        Some(c) if c.is_alphabetic() => name,
        _ => "Name".to_string(),
    }
}
