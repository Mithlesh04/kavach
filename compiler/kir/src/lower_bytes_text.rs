// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `b.text(start, end)` and `b[start:end].text()` on `Bytes`
//! (spec 2.13): the range's text, copied once, with no `Bytes` made in
//! between (Sakshi's S12: three small texts a line in a VCD header).

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, ExprKind};
use kavach_checker::{Prim, Ty};

impl<'a> Lowerer<'a> {
    /// `text` on `Bytes` with a range, given or from a slice `b[start:end]` right before
    /// it; `None` for any other `text`.
    pub(crate) fn bytes_text(
        &mut self,
        base: &'a ast::Expr,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
    ) -> Lowered<Option<ir::Expr>> {
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let is_bytes = |e: &ast::Expr| {
            matches!(
                self.c.expr_types.get(&(e.span.start, e.span.end)),
                Some(Ty::Prim(Prim::Bytes))
            )
        };
        if !is_bytes(base) {
            return Ok(None);
        }
        let (bytes, start, end): (&'a ast::Expr, Option<&'a ast::Expr>, Option<&'a ast::Expr>) =
            match (&base.kind, args) {
                // `b[s:e].text()`, with no step and no `#` (spec 2.19).
                (ExprKind::Index { base: b, index }, []) => match &index.kind {
                    ExprKind::Part {
                        start,
                        end,
                        step: None,
                    } if is_bytes(b)
                        && [start, end]
                            .into_iter()
                            .flatten()
                            .all(|x| !matches!(x.kind, ExprKind::FromEnd(_))) =>
                    {
                        (b, start.as_deref(), end.as_deref())
                    }
                    _ => return Ok(None),
                },
                (_, []) => return Ok(None),
                // `b.text(start, end)`, by position or by name.
                _ => {
                    let (mut start, mut end) = (None, None);
                    for (i, a) in args.iter().enumerate() {
                        match (a.name.as_ref().map(|n| n.name.as_str()), i) {
                            (Some("start"), _) | (None, 0) => start = Some(&a.value),
                            (Some("end"), _) | (None, _) => end = Some(&a.value),
                            _ => {}
                        }
                    }
                    (base, start, end)
                }
            };
        let mut values = vec![self.expr(bytes)?];
        for (e, default) in [(start, 0), (end, i64::MAX as i128)] {
            values.push(match e {
                Some(e) => self.expr(e)?,
                None => at(K::Int(default), Type::I64),
            });
        }
        Ok(Some(at(K::Rt(RtOp::BytesTextRange, values), ty)))
    }
}
