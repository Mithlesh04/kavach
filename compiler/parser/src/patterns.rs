//! Patterns for `match`, `let` and `for` (spec 4.3).

use crate::parser::Parser;
use kavach_ast::{FieldPattern, ListItem, MapKey, Pattern, PatternKind, RecordFieldPattern};
use kavach_lexer::{Keyword, TokenKind};

impl Parser<'_> {
    pub(crate) fn pattern(&mut self) -> Pattern {
        let start = self.span();
        let kind = match self.kind() {
            TokenKind::Ident => self.name_pattern(),
            TokenKind::Keyword(Keyword::None) => {
                self.bump();
                PatternKind::None
            }
            TokenKind::LParen => {
                self.bump();
                let mut items = Vec::new();
                while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
                    items.push(self.pattern());
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen, "`)`");
                PatternKind::Tuple(items)
            }
            TokenKind::LBracket => self.list_pattern(),
            TokenKind::LBrace => self.record_pattern(),
            TokenKind::Int(_)
            | TokenKind::Float
            | TokenKind::Char(_)
            | TokenKind::StrStart
            | TokenKind::Minus
            | TokenKind::Keyword(Keyword::True | Keyword::False) => {
                let lo = self.unary();
                let inclusive = self.at(&TokenKind::DotDotEq);
                if inclusive || self.at(&TokenKind::DotDot) {
                    self.bump();
                    let hi = self.unary();
                    PatternKind::Range { lo, hi, inclusive }
                } else {
                    PatternKind::Literal(lo)
                }
            }
            // `const match = 3`: a keyword where a name goes (spec 1.5).
            TokenKind::Keyword(k) => {
                let word = k.as_str();
                self.error_here(format!(
                    "`{word}` is a keyword, so it can't be a name (spec 1.5): choose another, such as `{word}_value`"
                ));
                self.bump();
                PatternKind::Wildcard
            }
            _ => {
                self.error_here("expected a pattern");
                if !self.at_end_of_line() && !self.at(&TokenKind::FatArrow) {
                    self.bump();
                }
                PatternKind::Wildcard
            }
        };
        Pattern {
            kind,
            span: self.span_from(start),
        }
    }

    /// `[first, second = 0, ...rest]` (spec 1.12).
    fn list_pattern(&mut self) -> PatternKind {
        self.bump();
        let mut items = Vec::new();
        let mut rest = None;
        while !self.at(&TokenKind::RBracket) && !self.at(&TokenKind::Eof) {
            if self.eat(&TokenKind::Ellipsis) {
                rest = Some(self.ident("a name for the rest of the list"));
                if !self.at(&TokenKind::RBracket) {
                    self.error_here("`...rest` comes last in a list pattern");
                }
                break;
            }
            let key = self.map_key();
            let pattern = self.pattern();
            let default = self.eat(&TokenKind::Eq).then(|| self.expr());
            items.push(ListItem {
                key,
                pattern,
                default,
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RBracket, "`]`");
        PatternKind::List { items, rest }
    }

    /// The key before an item of a map pattern: `host:`, `"asha":`, `7:`.
    fn map_key(&mut self) -> Option<MapKey> {
        if self.at(&TokenKind::Ident) && *self.nth(1) == TokenKind::Colon {
            let name = self.ident("a key");
            self.bump();
            return Some(MapKey::Name(name));
        }
        let literal = matches!(
            self.kind(),
            TokenKind::Int(_) | TokenKind::StrStart | TokenKind::Char(_) | TokenKind::Minus
        );
        if !literal {
            return None;
        }
        let key = self.unary();
        self.expect(&TokenKind::Colon, "`:` and a name for the value");
        Some(MapKey::Literal(key))
    }

    /// `{ name, age: years, city = "unknown" }` (spec 1.12).
    fn record_pattern(&mut self) -> PatternKind {
        self.bump();
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let field = self.ident("a field name");
            let pattern = if self.eat(&TokenKind::Colon) {
                self.pattern()
            } else {
                Pattern {
                    span: field.span,
                    kind: PatternKind::Binding(field.clone()),
                }
            };
            let default = self.eat(&TokenKind::Eq).then(|| self.expr());
            fields.push(RecordFieldPattern {
                field,
                pattern,
                default,
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RBrace, "`}`");
        PatternKind::Record(fields)
    }

    /// `_`, a binding, `ok(…)` / `err(…)`, or a variant (names start with a
    /// capital letter, spec 1.5).
    fn name_pattern(&mut self) -> PatternKind {
        let text = self.text();
        if text == "_" {
            self.bump();
            return PatternKind::Wildcard;
        }
        if (text == "ok" || text == "err") && *self.nth(1) == TokenKind::LParen {
            self.bump();
            self.bump();
            let inner = Box::new(self.pattern());
            self.expect(&TokenKind::RParen, "`)`");
            return if text == "ok" {
                PatternKind::Ok(inner)
            } else {
                PatternKind::Err(inner)
            };
        }
        // A small-letter name is a binding, unless a `.` follows: then it
        // names a module (`fs.NotFound`, spec 10.2).
        let module = *self.nth(1) == TokenKind::Dot;
        if !module && !text.chars().next().is_some_and(char::is_uppercase) {
            return PatternKind::Binding(self.ident("a name"));
        }
        let path = self.path("a variant name");
        let fields = self.at(&TokenKind::LParen).then(|| self.field_patterns());
        PatternKind::Variant { path, fields }
    }

    fn field_patterns(&mut self) -> Vec<FieldPattern> {
        self.bump();
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
            let name =
                (self.at(&TokenKind::Ident) && *self.nth(1) == TokenKind::Colon).then(|| {
                    let name = self.ident("a field name");
                    self.bump();
                    name
                });
            let pattern = self.pattern();
            fields.push(FieldPattern { name, pattern });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "`)`");
        fields
    }
}
