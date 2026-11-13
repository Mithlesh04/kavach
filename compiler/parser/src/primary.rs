//! The smallest expressions: literals, names, strings, lambdas, collections,
//! blocks, `if` and `match`.

use crate::parser::Parser;
use kavach_ast::{BinaryOp, Expr, ExprKind, LambdaParam, MatchArm, Span, StrPart};
use kavach_lexer::{Keyword, TokenKind};

impl Parser<'_> {
    pub(crate) fn primary(&mut self) -> Expr {
        let start = self.span();
        let kind = match self.kind().clone() {
            TokenKind::Int(value) => {
                self.bump();
                ExprKind::Int(value)
            }
            TokenKind::Float => {
                let text = self.text().replace('_', "");
                self.bump();
                ExprKind::Float(text)
            }
            TokenKind::BigInt => {
                let text = self.text().replace('_', "");
                self.bump();
                ExprKind::BigInt(text)
            }
            TokenKind::Char(c) => {
                self.bump();
                ExprKind::Char(c)
            }
            TokenKind::Keyword(
                k @ (Keyword::True | Keyword::False | Keyword::None | Keyword::SelfValue),
            ) => {
                self.bump();
                match k {
                    Keyword::True => ExprKind::Bool(true),
                    Keyword::False => ExprKind::Bool(false),
                    Keyword::None => ExprKind::None,
                    _ => ExprKind::SelfValue,
                }
            }
            TokenKind::StrStart => self.string(),
            TokenKind::Ident if *self.nth(1) == TokenKind::FatArrow && !self.in_guard => {
                let name = self.ident("a parameter name");
                self.bump();
                let body = Box::new(self.expr());
                ExprKind::Lambda {
                    params: vec![LambdaParam { name, ty: None }],
                    body,
                }
            }
            TokenKind::Ident => {
                let name = self.ident("a name");
                // `sql"…"` (spec 10.15).
                if self.tag_follows(name.span) {
                    self.tagged(vec![name])
                } else {
                    ExprKind::Name(name)
                }
            }
            TokenKind::LParen if !self.in_guard && self.paren_is_lambda() => self.lambda(),
            TokenKind::LParen => return self.paren_or_tuple(),
            TokenKind::LBracket => self.list_or_map(),
            TokenKind::LBrace => ExprKind::Block(self.block()),
            TokenKind::Keyword(Keyword::If) => self.if_expr(),
            TokenKind::Keyword(Keyword::Match) => self.match_expr(),
            TokenKind::Keyword(Keyword::Arena) => {
                self.bump();
                ExprKind::Arena(self.block())
            }
            TokenKind::Keyword(Keyword::Unsafe) => {
                self.bump();
                ExprKind::Unsafe(self.block())
            }
            TokenKind::Dot if self.dot_lambda > 0 => {
                // `.status` inside a dot lambda: the field access is read by postfix.
                return Expr {
                    kind: ExprKind::It,
                    span: Span {
                        start: start.start,
                        end: start.start,
                    },
                };
            }
            // `!ready`, as C, JavaScript and Rust write it (spec 1.8): read
            // as `not ready`, so one mistake gives one error.
            TokenKind::Bang => {
                // The name it negates, when there is one: `not locked`.
                let shown = match self.nth(1) {
                    TokenKind::Ident => format!("not {}", self.nth_text(1)),
                    _ => "not (…)".to_string(),
                };
                self.error_here(format!(
                    "Kavach has no `!` for \"not\": write `not`, as in `{shown}` (spec 1.8)"
                ));
                self.bump();
                ExprKind::Unary(kavach_ast::UnaryOp::Not, Box::new(self.unary()))
            }
            TokenKind::Dot => {
                self.error_here(
                    "a leading `.` only works as a function argument, like `items.filter(.paid)`",
                );
                self.bump();
                self.eat(&TokenKind::Ident);
                ExprKind::Error
            }
            _ => {
                self.error_here("expected an expression");
                if !self.at_end_of_line()
                    && !matches!(self.kind(), TokenKind::RParen | TokenKind::RBracket)
                {
                    self.bump();
                }
                ExprKind::Error
            }
        };
        Expr {
            kind,
            span: self.span_from(start),
        }
    }

    pub(crate) fn string(&mut self) -> ExprKind {
        self.bump();
        let mut parts = Vec::new();
        loop {
            match self.kind().clone() {
                TokenKind::Text(text) => {
                    self.bump();
                    parts.push(StrPart::Text(text));
                }
                TokenKind::InterpStart => {
                    self.bump();
                    parts.push(StrPart::Expr(self.expr()));
                    self.expect(
                        &TokenKind::InterpEnd,
                        "`}` to close the `{…}` in the string",
                    );
                }
                TokenKind::StrEnd => {
                    self.bump();
                    break;
                }
                TokenKind::Eof => break,
                _ => {
                    self.error_here("unexpected token inside a string");
                    self.bump();
                }
            }
        }
        ExprKind::Str(parts)
    }

    /// `(a, b) => …`: a `(` whose matching `)` is followed by `=>`.
    fn paren_is_lambda(&self) -> bool {
        let mut depth = 0usize;
        let mut i = self.position();
        loop {
            match self.token_at(i) {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        return *self.token_at(i + 1) == TokenKind::FatArrow;
                    }
                }
                TokenKind::Eof | TokenKind::Newline => return false,
                _ => {}
            }
            i += 1;
        }
    }

    fn lambda(&mut self) -> ExprKind {
        self.bump();
        let mut params = Vec::new();
        while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
            if self.at_word("inout") && *self.nth(1) == TokenKind::Ident {
                self.error_here("a lambda can't have `inout` parameters (spec 3.9): give the function a name, as `fn bump(inout n: Int) { … }`");
                self.bump();
            }
            let name = self.ident("a parameter name");
            let ty = self.eat(&TokenKind::Colon).then(|| self.type_expr());
            params.push(LambdaParam { name, ty });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "`)`");
        self.expect(&TokenKind::FatArrow, "`=>`");
        ExprKind::Lambda {
            params,
            body: Box::new(self.expr()),
        }
    }

    fn paren_or_tuple(&mut self) -> Expr {
        let start = self.span();
        self.bump();
        if self.at_anon_field() {
            let mut fields = Vec::new();
            while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
                let name = self.ident("a field name");
                self.expect(&TokenKind::Colon, "`:` and the field's value");
                fields.push((name, self.expr()));
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::RParen, "`)`");
            return Expr {
                kind: ExprKind::AnonRecord(fields),
                span: self.span_from(start),
            };
        }
        let mut items = Vec::new();
        let mut trailing_comma = false;
        while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
            items.push(self.expr());
            trailing_comma = self.eat(&TokenKind::Comma);
            if !trailing_comma {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "`)`");
        let span = self.span_from(start);
        if items.len() == 1 && !trailing_comma {
            let inner = items.pop().unwrap();
            return Expr {
                kind: inner.kind,
                span,
            };
        }
        Expr {
            kind: ExprKind::Tuple(items),
            span,
        }
    }

    /// After `(`: a field name and `:`, so an anonymous record
    /// (spec 2.14). `(x: Int) => …` was taken as a lambda before this.
    pub(crate) fn at_anon_field(&self) -> bool {
        matches!(self.kind(), TokenKind::Ident) && *self.nth(1) == TokenKind::Colon
    }

    /// `[1, 2]`, `["a": 1]`, `[]` and `[:]` (spec 2.2).
    fn list_or_map(&mut self) -> ExprKind {
        self.bump();
        if self.at(&TokenKind::Colon) && *self.nth(1) == TokenKind::RBracket {
            self.bump();
            self.bump();
            return ExprKind::Map(Vec::new());
        }
        if self.eat(&TokenKind::RBracket) {
            return ExprKind::List(Vec::new());
        }
        let first = self.expr();
        let kind = if self.eat(&TokenKind::Colon) {
            let mut entries = vec![(first, self.expr())];
            while self.eat(&TokenKind::Comma) && !self.at(&TokenKind::RBracket) {
                let key = self.expr();
                self.expect(&TokenKind::Colon, "`:` between key and value");
                entries.push((key, self.expr()));
            }
            ExprKind::Map(entries)
        } else {
            let mut items = vec![first];
            while self.eat(&TokenKind::Comma) && !self.at(&TokenKind::RBracket) {
                items.push(self.expr());
            }
            ExprKind::List(items)
        };
        self.expect(&TokenKind::RBracket, "`]`");
        kind
    }

    fn if_expr(&mut self) -> ExprKind {
        self.bump();
        if self.eat_kw(Keyword::Let) {
            let name = self.ident("a name");
            self.expect(&TokenKind::Eq, "`=`");
            let value = Box::new(self.expr());
            let then = self.block();
            let otherwise = self.else_branch();
            return ExprKind::IfLet {
                name,
                value,
                then,
                otherwise,
            };
        }
        let cond = Box::new(self.condition());
        let then = self.block();
        let otherwise = self.else_branch();
        ExprKind::If {
            cond,
            then,
            otherwise,
        }
    }

    /// The condition of an `if` or a `while`. An `=` there is a mistake for
    /// `==` (spec 4.1): one error says so, and it is read as `==`.
    pub(crate) fn condition(&mut self) -> Expr {
        let cond = self.expr();
        if !self.at(&TokenKind::Eq) {
            return cond;
        }
        self.error_here("`=` sets a variable: to compare, write `==` (spec 4.1)");
        self.bump();
        let right = self.expr();
        Expr {
            span: Span {
                start: cond.span.start,
                end: right.span.end,
            },
            kind: ExprKind::Binary(BinaryOp::Eq, Box::new(cond), Box::new(right)),
        }
    }

    fn else_branch(&mut self) -> Option<Box<Expr>> {
        if *self.peek_past_newlines() != TokenKind::Keyword(Keyword::Else) {
            return None;
        }
        self.skip_newlines();
        self.bump();
        let start = self.span();
        let kind = if self.at_kw(Keyword::If) {
            self.if_expr()
        } else {
            ExprKind::Block(self.block())
        };
        Some(Box::new(Expr {
            kind,
            span: self.span_from(start),
        }))
    }

    fn match_expr(&mut self) -> ExprKind {
        self.bump();
        let scrutinee = Box::new(self.expr());
        let mut arms = Vec::new();
        if self.expect(&TokenKind::LBrace, "`{` to start the match arms") {
            loop {
                self.skip_newlines();
                if self.at(&TokenKind::RBrace) || self.at(&TokenKind::Eof) {
                    break;
                }
                let before = self.position();
                let pattern = self.pattern();
                let guard = self.eat_kw(Keyword::If).then(|| {
                    self.in_guard = true;
                    let guard = self.expr();
                    self.in_guard = false;
                    guard
                });
                self.expect(&TokenKind::FatArrow, "`=>` after the pattern");
                let body = self.expr();
                arms.push(MatchArm {
                    pattern,
                    guard,
                    body,
                });
                if !self.eat(&TokenKind::Comma) {
                    self.end_of_stmt();
                }
                if self.position() == before {
                    self.bump();
                }
            }
            self.expect(&TokenKind::RBrace, "`}` to close the match");
        }
        ExprKind::Match { scrutinee, arms }
    }
}
