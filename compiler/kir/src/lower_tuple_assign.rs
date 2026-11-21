// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Tuple assignment, `(a, b) = (b, a)` (spec 1.14): every value on the
//! right goes into a local of its own first, then each place is set from
//! its local, left to right. A tuple written out on the right is never
//! made: its values go straight into their locals.

use crate::ir::{self, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast::{self as ast, ExprKind};

/// Each place with the local that holds its value.
type Held<'a> = Vec<(&'a ast::Expr, Local)>;

impl<'a> Lowerer<'a> {
    pub(crate) fn assign_tuple(
        &mut self,
        target: &'a ast::Expr,
        value: &'a ast::Expr,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        let mut held = Vec::new();
        self.hold_parts(target, value, &mut held, out)?;
        let line = self.line(target.span);
        for (place, local) in held {
            let v = self.local_expr(local, line);
            out.push(self.assign_path(place, v)?);
        }
        Ok(())
    }

    /// Works out `value`, a tuple written out or not, for `target`.
    fn hold_parts(
        &mut self,
        target: &'a ast::Expr,
        value: &'a ast::Expr,
        held: &mut Held<'a>,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        if let (ExprKind::Tuple(places), ExprKind::Tuple(values)) = (&target.kind, &value.kind) {
            for (p, v) in places.iter().zip(values) {
                self.hold_parts(p, v, held, out)?;
            }
            return Ok(());
        }
        let v = self.expr(value)?;
        self.hold_value(target, v, held, out)
    }

    /// Keeps `v` in a local; for a tuple of places, takes its parts.
    fn hold_value(
        &mut self,
        target: &'a ast::Expr,
        v: ir::Expr,
        held: &mut Held<'a>,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        let (ty, line) = (v.ty, v.line);
        let local = self.new_local(ty);
        out.push(Stmt::Let(local, v));
        let ExprKind::Tuple(places) = &target.kind else {
            held.push((target, local));
            return Ok(());
        };
        let Some(Shape::Tuple(items)) = self.shapes.shape(ty).cloned() else {
            return Err(self.unsupported("this tuple assignment", target.span));
        };
        for (i, (p, t)) in places.iter().zip(items).enumerate() {
            let index = ir::Expr {
                kind: ir::ExprKind::Int(i as i128),
                ty: Type::I64,
                line,
            };
            let part = ir::Expr {
                kind: ir::ExprKind::Index(Box::new(self.local_expr(local, line)), Box::new(index)),
                ty: t,
                line,
            };
            self.hold_value(p, part, held, out)?;
        }
        Ok(())
    }

    /// `pair.0 = v`: the tuple with its field `index` changed, stored
    /// back where it was (spec 1.14, 2.3).
    pub(crate) fn set_tuple_field(
        &mut self,
        base: &'a ast::Expr,
        index: u64,
        value: ir::Expr,
    ) -> Lowered<Stmt> {
        let line = value.line;
        let tuple = self.expr(base)?;
        let ty = tuple.ty;
        let Some(Shape::Tuple(items)) = self.shapes.shape(ty).cloned() else {
            return Err(self.unsupported("assigning to this", base.span));
        };
        let Some(&want) = items.get(index as usize) else {
            return Err(self.unsupported("assigning to this", base.span));
        };
        let value = self.coerce(value, want)?;
        let changed = ir::Expr {
            kind: ir::ExprKind::SetField(Box::new(tuple), index as u32, Box::new(value)),
            ty,
            line,
        };
        self.assign_path(base, changed)
    }
}
