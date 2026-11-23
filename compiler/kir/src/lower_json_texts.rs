// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `json.decode_texts<T>(names, texts)` and `json.encode_texts(value)` (spec
//! 2.12): a record read from one text for each field, and written as
//! them. The runtime reads each text by its field's type
//! (`runtime/native/src/rt_json_texts.rs`).

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    pub(crate) fn json_texts_call(
        &mut self,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let at = |kind, ty| ir::Expr { kind, ty, line };
        if name == "encode_texts" {
            let Some(a) = args.first() else {
                return Err(self.unsupported("`json.encode_texts()`", span));
            };
            let value = self.expr(&a.value)?;
            let tuple_ty = self.tuple_type(vec![value.ty]);
            let operand = at(K::Tuple(vec![value]), tuple_ty);
            return Ok(at(K::Rt(RtOp::JsonEncodeTexts, vec![operand]), ty));
        }
        let params = ["names", "texts"];
        let mut given: Vec<Option<&'a ast::Expr>> = vec![None; 2];
        for (i, a) in args.iter().enumerate() {
            let p = match &a.name {
                Some(n) => params.iter().position(|p| *p == n.name),
                None => Some(i),
            };
            if let Some(slot) = p.and_then(|p| given.get_mut(p)) {
                *slot = Some(&a.value);
            }
        }
        let [Some(names), Some(texts)] = [given[0], given[1]] else {
            return Err(self.unsupported("`json.decode_texts` without its lists", span));
        };
        if self.decodes_whole(span) {
            return Err(self.unsupported(
                "decoding into an `Int<bits: n>` or an `Int<range: …>`",
                span,
            ));
        }
        let (names, texts) = (self.expr(names)?, self.expr(texts)?);
        let templates = self.templates(ty, line, span)?;
        let decoded = at(
            K::Rt(RtOp::JsonDecodeTexts, vec![names, texts, templates]),
            ty,
        );
        // The invariants are checked too, as `decode` checks them.
        let checks = self.invariant_checks(ty, line)?;
        if checks.is_empty() {
            return Ok(decoded);
        }
        Ok(at(
            K::Block(ir::Block {
                stmts: checks,
                value: Some(Box::new(decoded)),
            }),
            ty,
        ))
    }
}
