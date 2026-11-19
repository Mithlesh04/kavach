// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Blocks and statements (spec 1.4, 4.4, 4.5).

use crate::parser::Parser;
use kavach_ast::{AssignOp, Block, ExprKind, Ident, Stmt, StmtKind, UnaryOp};
use kavach_lexer::{Keyword, TokenKind};

impl Parser<'_> {
    pub(crate) fn block(&mut self) -> Block {
        let start = self.span();
        if !self.expect(&TokenKind::LBrace, "`{`") {
            return Block {
                stmts: Vec::new(),
                span: start,
            };
        }
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&TokenKind::RBrace) || self.at(&TokenKind::Eof) {
                break;
            }
            let before = self.position();
            stmts.push(self.stmt());
            self.end_of_stmt();
            if self.position() == before {
                self.bump();
            }
        }
        self.expect(&TokenKind::RBrace, "`}` to close the block");
        Block {
            stmts,
            span: self.span_from(start),
        }
    }

    /// A statement must end at a line break, a `}` or the end of the file.
    pub(crate) fn end_of_stmt(&mut self) {
        if !self.at_end_of_line() {
            // `greet("Asha") { … }`: a block after a call (spec 4.8).
            if self.at(&TokenKind::LBrace) && self.previous() == Some(&TokenKind::RParen) {
                self.error_here(
                    "only `log.span(name)` and `random.seeded(seed)` take a block after them (spec 4.8): start the block on a new line, or call one of them",
                );
                self.recover_line();
                return;
            }
            self.error_here("expected the end of the statement: start a new line");
            self.recover_line();
        }
    }

    pub(crate) fn stmt(&mut self) -> Stmt {
        let start = self.span();
        let kind = match self.kind() {
            TokenKind::Keyword(Keyword::Const | Keyword::Let | Keyword::Var) => {
                // `const` never changes, `let` may; `var` is gone (decision 0034).
                if self.at_kw(Keyword::Var) {
                    self.error_here("`var` is not Kavach: write `let` for a variable, or `const` for a value that never changes");
                }
                let mutable = !self.at_kw(Keyword::Const);
                self.bump();
                // `let mut x`, Rust's way: say so, and read on as `let x`.
                if self.at_word("mut") && *self.nth(1) == TokenKind::Ident {
                    let name = self.nth_text(1);
                    self.error_here(format!(
                        "Kavach has no `let mut`: a `let` can already change, so write `let {name}`"
                    ));
                    self.bump();
                }
                let pattern = self.pattern();
                let ty = self.eat(&TokenKind::Colon).then(|| self.type_expr());
                self.expect(&TokenKind::Eq, "`=` and a value");
                let value = self.expr();
                StmtKind::Let {
                    mutable,
                    pattern,
                    ty,
                    value,
                }
            }
            // `outer: for …` (spec 4.6): a label, then the loop.
            TokenKind::Ident if self.at_loop_label() => {
                let label = Some(self.ident("a label"));
                self.bump();
                self.loop_stmt(label)
            }
            TokenKind::Keyword(Keyword::For | Keyword::While) => self.loop_stmt(None),
            // `on cores for …` (spec 4.9).
            TokenKind::Ident
                if self.at_loop_place(0)
                    && (self.place_then_for() || self.nth_text(1) == "threads") =>
            {
                self.loop_stmt(None)
            }
            TokenKind::Keyword(Keyword::Assert) => {
                self.bump();
                StmtKind::Assert(self.expr())
            }
            _ => self.expr_or_assign(),
        };
        Stmt {
            kind,
            span: self.span_from(start),
        }
    }

    fn expr_or_assign(&mut self) -> StmtKind {
        let target = self.expr();
        let Some(op) = assign_op(self.kind()) else {
            return StmtKind::Expr(target);
        };
        if !matches!(
            target.kind,
            ExprKind::Name(_)
                | ExprKind::Field { .. }
                | ExprKind::TupleField { .. }
                | ExprKind::Index { .. }
                | ExprKind::Unary(UnaryOp::Deref, _)
                | ExprKind::Error
        ) {
            self.error(
                target.span,
                "only a variable, field or index can be assigned to",
            );
        }
        self.bump();
        let value = self.expr();
        if assign_op(self.kind()).is_some() {
            self.error_here(
                "assignment is a statement, not an expression: put each assignment on its own line",
            );
            self.recover_line();
        }
        StmtKind::Assign { target, op, value }
    }
}

fn assign_op(kind: &TokenKind) -> Option<AssignOp> {
    Some(match kind {
        TokenKind::Eq => AssignOp::Set,
        TokenKind::PlusEq => AssignOp::Add,
        TokenKind::MinusEq => AssignOp::Sub,
        TokenKind::StarEq => AssignOp::Mul,
        TokenKind::SlashEq => AssignOp::Div,
        TokenKind::PercentEq => AssignOp::Rem,
        TokenKind::AtEq => AssignOp::MatMul,
        TokenKind::StarStarEq => AssignOp::Pow,
        TokenKind::AmpEq => AssignOp::BitAnd,
        TokenKind::PipeEq => AssignOp::BitOr,
        TokenKind::CaretEq => AssignOp::BitXor,
        TokenKind::ShlEq => AssignOp::Shl,
        TokenKind::ShrEq => AssignOp::Shr,
        _ => return None,
    })
}

impl Parser<'_> {
    /// Whether a label comes next: a name, `:`, then `for`, `while` or
    /// `on cores`.
    fn at_loop_label(&self) -> bool {
        *self.nth(1) == TokenKind::Colon
            && (matches!(
                self.nth(2),
                TokenKind::Keyword(Keyword::For | Keyword::While)
            ) || self.at_loop_place(2))
    }

    /// Whether `on cores` or `on threads` starts `at` tokens on: `on`, then
    /// `cores` or `threads`, then `for` or `(` (spec 4.9).
    fn at_loop_place(&self, at: usize) -> bool {
        let word = |k: usize, w: &str| *self.nth(k) == TokenKind::Ident && self.nth_text(k) == w;
        word(at, "on")
            && (word(at + 1, "cores") || word(at + 1, "threads"))
            && matches!(
                self.nth(at + 2),
                TokenKind::Keyword(Keyword::For) | TokenKind::LParen
            )
    }

    /// Whether a `for` follows the place at the start (`on cores(n) for`),
    /// past the count's parentheses; else it goes before a value.
    fn place_then_for(&self) -> bool {
        let mut k = 2;
        if *self.nth(k) == TokenKind::LParen {
            let mut depth = 0;
            loop {
                match self.nth(k) {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth -= 1,
                    TokenKind::Eof => return false,
                    _ => {}
                }
                k += 1;
                if depth == 0 {
                    break;
                }
            }
        }
        *self.nth(k) == TokenKind::Keyword(Keyword::For)
    }

    /// Whether a value's place starts here: `on cores`, `on cpu` or `on gpu`
    /// with something after it on the line (spec 4.9).
    pub(crate) fn at_value_place(&self) -> bool {
        let word = |k: usize, w: &str| *self.nth(k) == TokenKind::Ident && self.nth_text(k) == w;
        word(0, "on")
            && (word(1, "cores") || word(1, "cpu") || word(1, "gpu"))
            && matches!(
                self.nth(2),
                TokenKind::Ident | TokenKind::LParen | TokenKind::Keyword(Keyword::SelfValue)
            )
    }

    /// `on cores(n) x`, `on cpu x` or `on gpu(n) x`: the place, then the
    /// value.
    pub(crate) fn value_place(&mut self) -> ExprKind {
        self.bump();
        let word = self.text().to_string();
        self.bump();
        let count = |p: &mut Self| {
            p.eat(&TokenKind::LParen).then(|| {
                let n = p.expr();
                p.expect(&TokenKind::RParen, "`)`");
                Box::new(n)
            })
        };
        let place = match word.as_str() {
            "cores" => kavach_ast::ValuePlace::Cores(count(self)),
            "cpu" => kavach_ast::ValuePlace::Cpu,
            _ => match count(self) {
                Some(n) => kavach_ast::ValuePlace::Gpu(n),
                None => {
                    self.error_here("`on gpu` names its GPU: `on gpu(1) xs.map(f)` (spec 4.9)");
                    kavach_ast::ValuePlace::Cpu
                }
            },
        };
        ExprKind::On {
            place,
            value: Box::new(self.unary()),
        }
    }

    /// `on cores`, `on cores(n)` or `on threads(n)` before a `for`.
    fn loop_place(&mut self) -> Option<kavach_ast::LoopPlace> {
        if !self.at_loop_place(0) {
            return None;
        }
        let start = self.span();
        self.bump();
        let threads = self.text() == "threads";
        self.bump();
        let count = self.eat(&TokenKind::LParen).then(|| {
            let n = self.expr();
            self.expect(&TokenKind::RParen, "`)`");
            n
        });
        if !self.at_kw(Keyword::For) {
            self.error_here("`on cores` and `on threads` go right before a `for` (spec 4.9)");
        }
        Some(kavach_ast::LoopPlace {
            threads,
            count,
            span: self.span_from(start),
        })
    }

    /// A `for` or `while` loop, with its label and place if it has them.
    fn loop_stmt(&mut self, label: Option<Ident>) -> StmtKind {
        let place = self.loop_place();
        if self.at_kw(Keyword::For) {
            self.bump();
            let pattern = self.pattern();
            self.expect_kw(Keyword::In);
            let iter = self.expr();
            let body = self.block();
            return StmtKind::For {
                label,
                place,
                pattern,
                iter,
                body,
            };
        }
        self.bump();
        let cond = self.condition();
        let body = self.block();
        StmtKind::While { label, cond, body }
    }

    /// The label after `break` or `continue` on the same line, if any.
    pub(crate) fn label_after_jump(&mut self) -> Option<Ident> {
        (*self.kind() == TokenKind::Ident && !self.at_end_of_line()).then(|| self.ident("a label"))
    }

    /// `thread` or `cores` right after `spawn`, when a name or a block
    /// follows it (spec 9.8): `spawn thread()` still calls `thread`.
    pub(crate) fn spawn_kind(&mut self) -> kavach_ast::SpawnKind {
        let kind = match (self.kind(), self.text()) {
            (TokenKind::Ident, "thread") => kavach_ast::SpawnKind::Thread,
            (TokenKind::Ident, "cores") => kavach_ast::SpawnKind::Cores,
            (TokenKind::Ident, "process") => kavach_ast::SpawnKind::Process,
            _ => return kavach_ast::SpawnKind::Coroutine,
        };
        let next = matches!(
            self.nth(1),
            TokenKind::Ident | TokenKind::LBrace | TokenKind::Keyword(Keyword::SelfValue)
        );
        if !next {
            return kavach_ast::SpawnKind::Coroutine;
        }
        self.bump();
        kind
    }
}
