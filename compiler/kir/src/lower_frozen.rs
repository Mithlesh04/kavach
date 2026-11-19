// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `f.unfreeze(key: k)` (spec 2.20). The driver has already turned the
//! rest of frozen values into plain code (`compiler/driver/src/frozen.rs`):
//! a sealed value is `(value, key)`. Here the key is checked, and the
//! value given back.

use crate::ir::{self, BinOp, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// `{ let p = f; check p[1] == k; p[0] }`.
    pub(crate) fn sealed_unfreeze(
        &mut self,
        base: &'a ast::Expr,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let sealed = self.expr(base)?;
        let Some(Shape::Tuple(parts)) = self.shapes.shape(sealed.ty).cloned() else {
            return Err(self.unsupported("this sealed value", base.span));
        };
        let (Some(arg), [value_ty, key_ty]) = (args.first(), parts.as_slice()) else {
            return Err(self.unsupported("this sealed value", base.span));
        };
        let (value_ty, key_ty) = (*value_ty, *key_ty);
        let key = self.expr(&arg.value)?;
        let key = self.coerce(key, key_ty)?;
        let pair = self.new_local(sealed.ty);
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let part = |i: i128, ty: Type| {
            at(
                K::Index(
                    Box::new(at(K::Local(pair), sealed.ty)),
                    Box::new(at(K::Int(i), Type::I64)),
                ),
                ty,
            )
        };
        let same = at(
            K::Binary(BinOp::Eq, Box::new(part(1, key_ty)), Box::new(key)),
            Type::Bool,
        );
        let value = part(0, value_ty);
        let stmts = vec![
            Stmt::Let(pair, sealed),
            Stmt::Check {
                cond: same,
                message: kavach_checker::WRONG_KEY.to_string(),
                line,
            },
        ];
        Ok(at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(value)),
            }),
            ty,
        ))
    }
}
