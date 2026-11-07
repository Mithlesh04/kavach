//! Chained comparisons in native code (spec 1.8, decision 0040):
//! `a < b <= c` holds each operand in a local, worked out once and left
//! to right, and joins the comparisons with `and`, so the chain stops at
//! the first pair that doesn't hold.

use crate::ir::{self, Block, ExprKind as K, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::lower_expr::binary_op;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    pub(crate) fn chain(
        &mut self,
        first: &'a ast::Expr,
        links: &'a [(ast::BinaryOp, ast::Expr)],
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let first = self.expr(first)?;
        let held = self.new_local(first.ty);
        let rest = self.links(self.local_expr(held, line), links, span)?;
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts: vec![Stmt::Let(held, first)],
                value: Some(Box::new(rest)),
            }),
            ty: Type::Bool,
            line,
        })
    }

    /// `left op1 x1 op2 x2 …`: `x1` worked out, compared, and the rest
    /// only when that holds.
    fn links(
        &mut self,
        left: ir::Expr,
        links: &'a [(ast::BinaryOp, ast::Expr)],
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = left.line;
        let ((op, x), rest) = links.split_first().expect("a chain has links");
        let right = self.expr(x)?;
        if rest.is_empty() {
            return self.compared(*op, left, right, span);
        }
        let held = self.new_local(right.ty);
        let here = self.compared(*op, left, self.local_expr(held, line), span)?;
        let later = self.links(self.local_expr(held, line), rest, span)?;
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts: vec![Stmt::Let(held, right)],
                value: Some(Box::new(ir::Expr {
                    kind: K::And(Box::new(here), Box::new(later)),
                    ty: Type::Bool,
                    line,
                })),
            }),
            ty: Type::Bool,
            line,
        })
    }

    /// `l op r` for a comparison `op`, `T` against `T?` as optionals.
    fn compared(
        &mut self,
        op: ast::BinaryOp,
        l: ir::Expr,
        r: ir::Expr,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = l.line;
        let r = self.coerce(r, l.ty)?;
        let l = self.coerce(l, r.ty)?;
        let op =
            binary_op(op).ok_or_else(|| self.unsupported(format!("`{}`", op.as_str()), span))?;
        Ok(ir::Expr {
            kind: K::Binary(op, Box::new(l), Box::new(r)),
            ty: Type::Bool,
            line,
        })
    }
}
