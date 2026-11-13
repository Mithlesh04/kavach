//! Tagged literals (spec 10.15): the `literal` declaration, and `sql"…"`
//! or `pg.sql"…"`, a name written right before a string.

use crate::parser::Parser;
use kavach_ast::*;
use kavach_lexer::TokenKind;

impl Parser<'_> {
    /// Whether a string starts right where `before` ends: `sql"…"`.
    pub(crate) fn tag_follows(&self, before: Span) -> bool {
        self.at(&TokenKind::StrStart) && self.span().start == before.end
    }

    /// The tagged literal whose tag is `segments`, at its string.
    pub(crate) fn tagged(&mut self, segments: Vec<Ident>) -> ExprKind {
        let first = segments.first().map_or(self.span(), |s| s.span);
        let start = self.span();
        let text = self.string();
        let span = self.span_from(start);
        let path = Path {
            span: Span {
                start: first.start,
                end: start.start,
            },
            segments,
        };
        let value = Expr { kind: text, span };
        ExprKind::Tagged {
            tag: path,
            args: vec![Arg { name: None, value }],
        }
    }

    /// `literal sql: Query check check_sql language "sql"`.
    pub(crate) fn literal_decl(&mut self) -> LiteralDecl {
        self.bump(); // `literal`
        let name = self.ident("the tag's name");
        self.expect(&TokenKind::Colon, "`:` and the type the literal makes");
        let ty = self.type_expr();
        if self.at_word("check") {
            self.bump();
        } else {
            self.error_here("`check` and the function that checks the text: `check check_sql`");
        }
        let check = self.ident("the function that checks the text");
        let mut language = None;
        if self.at_word("language") {
            self.bump();
            if let Some(ExprKind::Str(parts)) = self.at(&TokenKind::StrStart).then(|| self.string())
                && let [StrPart::Text(t)] = parts.as_slice()
            {
                language = Some(t.clone());
            } else {
                self.error_here("the language's name, as text: `language \"sql\"`");
            }
        }
        LiteralDecl {
            name,
            ty,
            check,
            language,
        }
    }
}
