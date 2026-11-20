// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `a ** b` in native code (spec 1.8): floats through the op `math.pow`
//! uses, whole numbers through `IntPow`, which panics when the result
//! doesn't fit the type, as `*` does.

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::Lowerer;
use crate::rt_op::RtOp;

impl Lowerer<'_> {
    pub(crate) fn power(&self, l: ir::Expr, r: ir::Expr, line: u32) -> ir::Expr {
        let ty = l.ty;
        let at = |kind, ty| ir::Expr { kind, ty, line };
        // A `Float32` power is worked out as a `Float`, then rounded once:
        // the interpreter does the same.
        if ty == Type::F32 {
            let wide = |x: ir::Expr| at(K::Convert(Box::new(x), false), Type::F64);
            let pow = at(K::Rt(RtOp::MathPow, vec![wide(l), wide(r)]), Type::F64);
            return at(K::Convert(Box::new(pow), false), Type::F32);
        }
        let op = if ty == Type::F64 {
            RtOp::MathPow
        } else {
            RtOp::IntPow
        };
        at(K::Rt(op, vec![l, r]), ty)
    }
}

impl Lowerer<'_> {
    /// `x.fract()` (spec 14.2): `x - floor(x)`, as WGSL's `fract`, worked
    /// out as a `Float` (exact for a `Float32` too) and given back as `x`'s
    /// type.
    pub(crate) fn fract(&mut self, x: ir::Expr, line: u32) -> ir::Expr {
        let ty = x.ty;
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let wide = at(K::Convert(Box::new(x), false), Type::F64);
        let held = self.new_local(Type::F64);
        let get = |s: &Self| s.local_expr(held, line);
        let floor = at(K::Rt(RtOp::MathFloor, vec![get(self)]), Type::F64);
        let part = at(
            K::Binary(ir::BinOp::Sub, Box::new(get(self)), Box::new(floor)),
            Type::F64,
        );
        let value = at(K::Convert(Box::new(part), false), ty);
        at(
            K::Block(ir::Block {
                stmts: vec![ir::Stmt::Let(held, wide)],
                value: Some(Box::new(value)),
            }),
            ty,
        )
    }

    /// `x.is_nan()`, `x.is_finite()` and `x.is_infinite()` (spec 14.2), as
    /// compares with no call: `NaN` is the one value not equal to itself,
    /// and `x - x` is `NaN` exactly when `x` is `NaN` or an infinity.
    pub(crate) fn float_check(&mut self, name: &str, x: ir::Expr, line: u32) -> ir::Expr {
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let wide = at(K::Convert(Box::new(x), false), Type::F64);
        let held = self.new_local(Type::F64);
        let get = |s: &Self| s.local_expr(held, line);
        let cmp =
            |op, a: ir::Expr, b: ir::Expr| at(K::Binary(op, Box::new(a), Box::new(b)), Type::Bool);
        let diff = |s: &Self| {
            at(
                K::Binary(ir::BinOp::Sub, Box::new(get(s)), Box::new(get(s))),
                Type::F64,
            )
        };
        let value = match name {
            "is_nan" => cmp(ir::BinOp::Ne, get(self), get(self)),
            "is_finite" => cmp(ir::BinOp::Eq, diff(self), diff(self)),
            _ => {
                let number = cmp(ir::BinOp::Eq, get(self), get(self));
                let endless = cmp(ir::BinOp::Ne, diff(self), diff(self));
                at(K::And(Box::new(number), Box::new(endless)), Type::Bool)
            }
        };
        at(
            K::Block(ir::Block {
                stmts: vec![ir::Stmt::Let(held, wide)],
                value: Some(Box::new(value)),
            }),
            Type::Bool,
        )
    }
}
