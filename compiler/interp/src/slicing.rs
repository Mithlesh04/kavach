// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Indexing and slicing (spec 2.19): `x[#k]`, `x[start:end:step]`,
//! a text's characters, and setting a slice. The rules themselves are in
//! `kavach_runtime_core::slices`, shared with native code.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{Expr, ExprKind, Span};
use kavach_checker::Prim;
use kavach_runtime_core::slices::{self, Part};
use std::rc::Rc;

/// How many positions `v` has: items, characters or bytes.
pub(crate) fn positions(v: &Value<'_>) -> Option<usize> {
    Some(match v {
        Value::List(items) => items.len(),
        Value::Str(s) => s.chars().count(),
        Value::Bytes(b) => b.len(),
        _ => return None,
    })
}

/// `xs[part] = value` on the list in `slot`: as many items as the slice
/// has, each set in place (spec 2.19).
pub(crate) fn set_slice<'a>(
    slot: &mut Value<'a>,
    part: &Part,
    value: Value<'a>,
) -> Result<(), String> {
    let (Value::List(items), Value::List(given)) = (slot, value) else {
        return Err("can't assign here".to_string());
    };
    let at = part.set_positions(items.len(), given.len())?;
    let items = Rc::make_mut(items);
    for (i, v) in at.into_iter().zip(given.iter()) {
        items[i] = v.clone();
    }
    Ok(())
}

impl<'a> Interp<'a> {
    /// `base[index]` when `index` is `#k` or a slice; `None` otherwise.
    pub(crate) fn index_form(
        &mut self,
        base: &'a Expr,
        index: &'a Expr,
        span: Span,
    ) -> Option<Eval<'a>> {
        match &index.kind {
            ExprKind::FromEnd(k) => Some(self.counted_from_end(base, k, span)),
            ExprKind::Part { .. } => Some(self.slice(base, index, span)),
            _ => None,
        }
    }

    fn counted_from_end(&mut self, base: &'a Expr, k: &'a Expr, span: Span) -> Eval<'a> {
        let b = self.eval(base)?;
        let k = self.int_of(k)?;
        let n = positions(&b).unwrap_or(0);
        self.index(b, Value::Int(slices::from_end(n, k), Prim::Int), span)
    }

    fn slice(&mut self, base: &'a Expr, index: &'a Expr, span: Span) -> Eval<'a> {
        let b = self.eval(base)?;
        let part = self.part(index)?;
        let cut = match &b {
            Value::List(items) => {
                slices::pick(items, &part).map(|v| Value::List(Rc::new(v.into())))
            }
            Value::Str(s) => slices::text(s, &part).map(|t| Value::Str(Rc::new(t))),
            Value::Bytes(bytes) => slices::pick(bytes, &part).map(|v| Value::Bytes(v.into())),
            _ => return Err(self.panic("this can't be sliced", span)),
        };
        cut.map_err(|m| self.panic(m, span))
    }

    /// The numbers of a slice as written.
    pub(crate) fn part(&mut self, index: &'a Expr) -> Result<Part, Flow<'a>> {
        let ExprKind::Part { start, end, step } = &index.kind else {
            return Ok(Part::default());
        };
        let mut part = Part::default();
        if let Some(x) = start {
            let (v, from_end) = self.bound(x)?;
            part.start = Some(v);
            part.start_from_end = from_end;
        }
        if let Some(x) = end {
            let (v, from_end) = self.bound(x)?;
            part.end = Some(v);
            part.end_from_end = from_end;
        }
        if let Some(x) = step {
            part.step = Some(self.int_of(x)?);
        }
        Ok(part)
    }

    /// An end of a slice: its number, and whether it was written `#k`.
    fn bound(&mut self, x: &'a Expr) -> Result<(i64, bool), Flow<'a>> {
        match &x.kind {
            ExprKind::FromEnd(k) => Ok((self.int_of(k)?, true)),
            _ => Ok((self.int_of(x)?, false)),
        }
    }

    pub(crate) fn int_of(&mut self, x: &'a Expr) -> Result<i64, Flow<'a>> {
        match self.eval(x)? {
            Value::Int(n, _) => Ok(n.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64),
            _ => Err(self.panic("a position must be a whole number", x.span)),
        }
    }
}
