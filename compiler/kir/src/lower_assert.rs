//! `assert` (spec 11.8): a failing comparison shows both sides, as
//! `assertion failed: total == 10 (left: 9, right: 10)`. Each side is
//! worked out once; other conditions show only their text.

use crate::ir::{self, Block, ExprKind as K, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, BinaryOp, ExprKind};

impl<'a> Lowerer<'a> {
    pub(crate) fn assert_stmt(&mut self, e: &'a ast::Expr, out: &mut Vec<Stmt>) -> Lowered<()> {
        let line = self.line(e.span);
        let text = self.text(e.span);
        let ExprKind::Binary(
            op @ (BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge),
            l,
            r,
        ) = &e.kind
        else {
            let cond = self.expr(e)?;
            out.push(Stmt::Check {
                cond,
                message: format!("assertion failed: {text}"),
                line,
            });
            return Ok(());
        };
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let (l, r) = (self.expr(l)?, self.expr(r)?);
        let r = self.coerce(r, l.ty)?;
        let l = self.coerce(l, r.ty)?;
        let (a, b) = (self.new_local(l.ty), self.new_local(r.ty));
        let (a_ty, b_ty) = (l.ty, r.ty);
        out.push(Stmt::Let(a, l));
        out.push(Stmt::Let(b, r));
        let Some(bin) = crate::lower_expr::binary_op(*op) else {
            return Err(self.unsupported("this `assert`", e.span));
        };
        let holds = at(
            K::Binary(
                bin,
                Box::new(at(K::Local(a), a_ty)),
                Box::new(at(K::Local(b), b_ty)),
            ),
            Type::Bool,
        );
        let string = self.str_type();
        let message = at(
            K::Format(vec![
                at(K::Str(format!("assertion failed: {text} (left: ")), string),
                at(K::Local(a), a_ty),
                at(K::Str(", right: ".into()), string),
                at(K::Local(b), b_ty),
                at(K::Str(")".into()), string),
            ]),
            string,
        );
        let fail = Stmt::Expr(at(K::Rt(RtOp::PanicText, vec![message]), Type::Unit));
        out.push(Stmt::Expr(at(
            K::If(
                Box::new(at(K::Unary(ir::UnOp::Not, Box::new(holds)), Type::Bool)),
                Block {
                    stmts: vec![fail],
                    value: None,
                },
                Block::default(),
            ),
            Type::Unit,
        )));
        Ok(())
    }
}
