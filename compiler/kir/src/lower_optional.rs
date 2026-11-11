//! `a ?? b` (spec 2.7): the value inside an optional, or `b`. A map read
//! with a scalar value and a simple fallback becomes one runtime lookup
//! (`MapGetOr`); `x ?? break` leaves instead of giving a value.

use crate::ir::{self, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::lower_expr::wrap;
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// `a ?? b`: the value inside `a`, or `b` when `a` is `none`.
    pub(crate) fn default_value(
        &mut self,
        l: &'a ast::Expr,
        r: &'a ast::Expr,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let value = self.expr(l)?;
        let Some(Shape::Optional(inner)) = self.shapes.shape(value.ty).cloned() else {
            return Err(self.unsupported("`??` here", l.span));
        };
        // `x ?? break`: the `none` branch leaves (`return`, `break`,
        // `continue`) instead of giving a value.
        if crate::lower::leaves(r) {
            let mut leave = Block::default();
            self.expr_stmt(r, &mut leave.stmts)?;
            return self.unwrap_or(value, inner, leave, ty, line);
        }
        let fallback = self.expr(r)?;
        let value = match map_get_or(value, &fallback, inner, ty, line) {
            Ok(one_lookup) => return Ok(one_lookup),
            Err(value) => value,
        };
        let tmp = self.new_local(value.ty);
        let present = ir::Expr {
            kind: K::IsSome(Box::new(self.local_expr(tmp, line))),
            ty: Type::Bool,
            line,
        };
        let inside = ir::Expr {
            kind: K::Unwrap(Box::new(self.local_expr(tmp, line))),
            ty: inner,
            line,
        };
        let (inside, fallback) = (self.coerce(inside, ty)?, self.coerce(fallback, ty)?);
        let choice = ir::Expr {
            kind: K::If(Box::new(present), wrap(inside, true), wrap(fallback, true)),
            ty,
            line,
        };
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts: vec![Stmt::Let(tmp, value)],
                value: Some(Box::new(choice)),
            }),
            ty,
            line,
        })
    }
}

/// `map[key] ?? fallback` as one lookup, when the value is a scalar and
/// the fallback costs nothing to compute (so computing it every time is
/// the same as computing it only when the key is missing). So too
/// `j.int() ?? fallback` and `j.float() ?? fallback` of a `Json`.
fn map_get_or(
    value: ir::Expr,
    fallback: &ir::Expr,
    inner: Type,
    ty: Type,
    line: u32,
) -> Result<ir::Expr, ir::Expr> {
    let simple = matches!(
        fallback.kind,
        K::Int(_) | K::Float(_) | K::Bool(_) | K::Char(_) | K::Local(_)
    );
    let fits = !inner.is_ref() && inner == ty && fallback.ty == ty && simple;
    let (op, or) = match &value.kind {
        K::Rt(RtOp::JsonInt, _) => (RtOp::JsonInt, RtOp::JsonIntOr),
        K::Rt(RtOp::JsonFloat, _) => (RtOp::JsonFloat, RtOp::JsonFloatOr),
        _ => (RtOp::MapGet, RtOp::MapGetOr),
    };
    let K::Rt(found, mut args) = value.kind else {
        return Err(value);
    };
    if found != op {
        return Err(ir::Expr {
            kind: K::Rt(found, args),
            ..value
        });
    }
    if !fits {
        return Err(ir::Expr {
            kind: K::Rt(op, args),
            ..value
        });
    }
    let fallback = ir::Expr {
        kind: match fallback.kind {
            K::Int(n) => K::Int(n),
            K::Float(f) => K::Float(f),
            K::Bool(b) => K::Bool(b),
            K::Char(c) => K::Char(c),
            K::Local(l) => K::Local(l),
            _ => unreachable!("a simple fallback"),
        },
        ty,
        line,
    };
    args.push(fallback);
    Ok(ir::Expr {
        kind: K::Rt(or, args),
        ty,
        line,
    })
}

impl Lowerer<'_> {
    /// `let t = value; if t.is_some() { t.unwrap() } else { otherwise }`.
    fn unwrap_or(
        &mut self,
        value: ir::Expr,
        inner: Type,
        otherwise: Block,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let tmp = self.new_local(value.ty);
        let present = ir::Expr {
            kind: K::IsSome(Box::new(self.local_expr(tmp, line))),
            ty: Type::Bool,
            line,
        };
        let inside = ir::Expr {
            kind: K::Unwrap(Box::new(self.local_expr(tmp, line))),
            ty: inner,
            line,
        };
        let inside = self.coerce(inside, ty)?;
        let choice = ir::Expr {
            kind: K::If(Box::new(present), wrap(inside, true), otherwise),
            ty,
            line,
        };
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts: vec![Stmt::Let(tmp, value)],
                value: Some(Box::new(choice)),
            }),
            ty,
            line,
        })
    }
}
