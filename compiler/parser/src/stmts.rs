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
    /// Whether a label comes next: a name, `:`, then `for` or `while`.
    fn at_loop_label(&self) -> bool {
        *self.nth(1) == TokenKind::Colon
            && matches!(
                self.nth(2),
                TokenKind::Keyword(Keyword::For | Keyword::While)
            )
    }

    /// A `for` or `while` loop, with its label if it has one.
    fn loop_stmt(&mut self, label: Option<Ident>) -> StmtKind {
        if self.at_kw(Keyword::For) {
            self.bump();
            let pattern = self.pattern();
            self.expect_kw(Keyword::In);
            let iter = self.expr();
            let body = self.block();
            return StmtKind::For {
                label,
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
