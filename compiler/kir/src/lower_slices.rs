// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Lowering indexing and slicing (spec 2.19): `x[#k]`, `x[a:b:step]`,
//! a text's characters, and setting a slice. The runtime works the
//! slice out (`rt_slices.rs`), from the four words of
//! `kavach_runtime_core::slices::Part`.

use crate::ir::{self, BinOp, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, ExprKind};
use kavach_runtime_core::slices::Part;

impl<'a> Lowerer<'a> {
    /// `base[index]` when `index` is `#k` or a slice; `None` otherwise.
    pub(crate) fn index_form(
        &mut self,
        base: &'a ast::Expr,
        index: &'a ast::Expr,
        ty: Type,
        line: u32,
    ) -> Option<Lowered<ir::Expr>> {
        match &index.kind {
            ExprKind::FromEnd(k) => Some(self.counted_from_end(base, k, ty, line)),
            ExprKind::Part { .. } => Some(self.slice(base, index, ty, line)),
            _ => None,
        }
    }

    /// `x[#k]`: `x[x.len() - k]`, with `x` worked out once.
    fn counted_from_end(
        &mut self,
        base: &'a ast::Expr,
        k: &'a ast::Expr,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let b = self.expr(base)?;
        let held = self.new_local(b.ty);
        let at = self.len_minus(held, b.ty, k, line, base.span)?;
        let whole = self.local_expr(held, line);
        let item = self.item_at(whole, at, ty, line, base.span)?;
        let block = Block {
            stmts: vec![Stmt::Let(held, b)],
            value: Some(Box::new(item)),
        };
        Ok(self.at(K::Block(block), ty, line))
    }

    /// `len(held) - k`, an `Int`.
    pub(crate) fn len_minus(
        &mut self,
        held: ir::Local,
        held_ty: Type,
        k: &'a ast::Expr,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let len_op = match self.shapes.shape(held_ty) {
            Some(Shape::List(_)) => RtOp::ListLen,
            Some(Shape::Str) => RtOp::StrLen,
            Some(Shape::Bytes) => RtOp::BytesLen,
            _ => return Err(self.unsupported("`#` on this", span)),
        };
        let len = self.at(
            K::Rt(len_op, vec![self.local_expr(held, line)]),
            Type::I64,
            line,
        );
        let k = self.expr(k)?;
        let k = self.coerce(k, Type::I64)?;
        Ok(self.at(
            K::Binary(BinOp::Sub, Box::new(len), Box::new(k)),
            Type::I64,
            line,
        ))
    }

    /// Item `at` of a list, a text or bytes.
    pub(crate) fn item_at(
        &mut self,
        whole: ir::Expr,
        at: ir::Expr,
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        Ok(match self.shapes.shape(whole.ty) {
            Some(Shape::List(_)) => self.at(K::Index(Box::new(whole), Box::new(at)), ty, line),
            Some(Shape::Str) => self.at(K::Rt(RtOp::StrAt, vec![whole, at]), ty, line),
            Some(Shape::Bytes) => self.at(K::Rt(RtOp::BytesAt, vec![whole, at]), ty, line),
            _ => return Err(self.unsupported("indexing this", span)),
        })
    }

    /// `x[a:b:step]`.
    fn slice(
        &mut self,
        base: &'a ast::Expr,
        index: &'a ast::Expr,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let b = self.expr(base)?;
        // `s[a:b]` with no step and no `#`: the ends alone, no list.
        if let (
            Some(Shape::Str),
            ExprKind::Part {
                start,
                end,
                step: None,
            },
        ) = (self.shapes.shape(b.ty), &index.kind)
            && [start, end].iter().all(|x| {
                !matches!(
                    x.as_deref(),
                    Some(ast::Expr {
                        kind: ExprKind::FromEnd(_),
                        ..
                    })
                )
            })
        {
            let mut ends = Vec::new();
            for (x, missing) in [(start, 0), (end, i64::MAX)] {
                let v = match x.as_deref() {
                    Some(x) => self.expr(x)?,
                    None => self.at(K::Int(i128::from(missing)), Type::I64, line),
                };
                ends.push(self.coerce(v, Type::I64)?);
            }
            let mut args = vec![b];
            args.extend(ends);
            return Ok(self.at(K::Rt(RtOp::StrSliceRange, args), ty, line));
        }
        let op = match self.shapes.shape(b.ty) {
            Some(Shape::List(_)) => RtOp::ListSlicePart,
            Some(Shape::Str) => RtOp::StrSlicePart,
            Some(Shape::Bytes) => RtOp::BytesSlicePart,
            _ => return Err(self.unsupported("slicing this", base.span)),
        };
        let words = self.part_words(index, line)?;
        Ok(self.at(K::Rt(op, vec![b, words]), ty, line))
    }

    /// The slice's four words, as a list of `Int`s: start, end, step and
    /// which of them are written, which is known here.
    pub(crate) fn part_words(&mut self, index: &'a ast::Expr, line: u32) -> Lowered<ir::Expr> {
        let ExprKind::Part { start, end, step } = &index.kind else {
            return Err(self.unsupported("this slice", index.span));
        };
        let mut words = Vec::new();
        let mut from_end = [false, false];
        for (i, x) in [start, end].into_iter().enumerate() {
            let value = match x.as_deref() {
                Some(ast::Expr {
                    kind: ExprKind::FromEnd(k),
                    ..
                }) => {
                    from_end[i] = true;
                    self.expr(k)?
                }
                Some(x) => self.expr(x)?,
                None => self.at(K::Int(0), Type::I64, line),
            };
            words.push(self.coerce(value, Type::I64)?);
        }
        let step_value = match step {
            Some(s) => self.expr(s)?,
            None => self.at(K::Int(1), Type::I64, line),
        };
        words.push(self.coerce(step_value, Type::I64)?);
        let flags = Part::flags(
            start.is_some(),
            from_end[0],
            end.is_some(),
            from_end[1],
            step.is_some(),
        );
        words.push(self.at(K::Int(i128::from(flags)), Type::I64, line));
        let list = self.list_type(Type::I64);
        Ok(self.at(K::List(words), list, line))
    }

    /// `xs[#k] = v` and `xs[a:b:step] = ys` on a list: statements to run
    /// first, and the changed list.
    pub(crate) fn set_part(
        &mut self,
        base: &'a ast::Expr,
        index: &'a ast::Expr,
        value: ir::Expr,
        line: u32,
    ) -> Lowered<(Vec<Stmt>, ir::Expr)> {
        let mut first = Vec::new();
        let changed = match &index.kind {
            ExprKind::FromEnd(k) => {
                // The position, from the length of the list as it is now,
                // read from the place, which has no effects of its own.
                let now = self.expr(base)?;
                if !matches!(self.shapes.shape(now.ty), Some(Shape::List(_))) {
                    return Err(self.unsupported("assigning to this", base.span));
                }
                let len = self.at(K::Rt(RtOp::ListLen, vec![now]), Type::I64, line);
                let k = self.expr(k)?;
                let k = self.coerce(k, Type::I64)?;
                let at = self.at(
                    K::Binary(BinOp::Sub, Box::new(len), Box::new(k)),
                    Type::I64,
                    line,
                );
                let held = self.new_local(Type::I64);
                first.push(Stmt::Let(held, at));
                let list = self.expr(base)?;
                let ty = list.ty;
                let Some(Shape::List(item)) = self.shapes.shape(ty).cloned() else {
                    return Err(self.unsupported("assigning to this", base.span));
                };
                let value = self.coerce(value, item)?;
                let at = self.local_expr(held, line);
                self.at(K::Rt(RtOp::ListSet, vec![list, at, value]), ty, line)
            }
            _ => {
                let list = self.expr(base)?;
                let ty = list.ty;
                if !matches!(self.shapes.shape(ty), Some(Shape::List(_))) {
                    return Err(self.unsupported("assigning to this", base.span));
                }
                let words = self.part_words(index, line)?;
                let value = self.coerce(value, ty)?;
                self.at(K::Rt(RtOp::ListSetPart, vec![list, words, value]), ty, line)
            }
        };
        Ok((first, changed))
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::ExprKind;
    use crate::rt_op::RtOp;

    /// The ops a program's functions call.
    fn ops(src: &str) -> Vec<RtOp> {
        let c = kavach_driver::compile(src, "t.kav", Default::default());
        let program = crate::lower(&c).expect("lowers");
        let mut ops = Vec::new();
        for f in &program.functions {
            crate::walk::any_in_block(&f.body, &mut |e| {
                if let ExprKind::Rt(op, _) = &e.kind {
                    ops.push(*op);
                }
                false
            });
        }
        ops
    }

    /// `s[a:b]`, `s[a:]` and `s[:b]` take their ends alone, with no list
    /// of words (S8); a step or a `#` still takes the words.
    #[test]
    fn a_text_range_takes_its_ends_alone() {
        for (slice, range) in [
            ("s[1:2]", true),
            ("s[1:]", true),
            ("s[:n]", true),
            ("s[::2]", false),
            ("s[#2:]", false),
        ] {
            let src = format!(
                "use std.cli\n\nfn main() {{\n    const s = `x{{cli.args().len()}}yz`\n    const n = s.len()\n    print({slice})\n}}\n"
            );
            let ops = ops(&src);
            assert_eq!(ops.contains(&RtOp::StrSliceRange), range, "{slice}");
            assert_eq!(ops.contains(&RtOp::StrSlicePart), !range, "{slice}");
        }
    }
}
