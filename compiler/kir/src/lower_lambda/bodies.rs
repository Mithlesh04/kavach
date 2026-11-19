// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `flat_map`'s inner loop and a lambda's body of statements (spec 2.2,
//! 3.3), for `lower_lambda.rs`.

use crate::ir::{self, Block, ExprKind as K, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// `flat_map`: pushes every item of the inner list `value` onto `acc`.
    pub(super) fn flat_push(
        &mut self,
        acc: Local,
        value: ir::Expr,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let Some(Shape::List(inner_item)) = self.shapes.shape(value.ty).cloned() else {
            return Err(crate::Unsupported {
                feature: "`flat_map` with a lambda that doesn't give a list".into(),
                line,
            });
        };
        let (inner, var, index) = (
            self.new_local(value.ty),
            self.new_local(inner_item),
            self.new_local(Type::I64),
        );
        let walk = Stmt::ForEach {
            var,
            index,
            list: inner,
            body: self.push_to(acc, self.local_expr(var, line)),
        };
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts: vec![Stmt::Let(inner, value), walk],
                value: None,
            }),
            ty: Type::Unit,
            line,
        })
    }

    /// A lambda's body: a value, or for a body that gives nothing
    /// (`x => print(x)`), its statements, which `print` needs.
    pub(super) fn lambda_body(&mut self, body: &'a ast::Expr, line: u32) -> Lowered<ir::Expr> {
        let ty = self.c.expr_types.get(&(body.span.start, body.span.end));
        if !matches!(ty, Some(kavach_checker::Ty::Unit)) {
            return self.expr(body);
        }
        let mut stmts = Vec::new();
        self.expr_stmt(body, &mut stmts)?;
        Ok(ir::Expr {
            kind: K::Block(Block { stmts, value: None }),
            ty: Type::Unit,
            line,
        })
    }
}
