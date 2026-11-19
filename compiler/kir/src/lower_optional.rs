// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
        if crate::lower_body::leaves(r) {
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

impl Lowerer<'_> {
    /// `get`, a `MapGet` of type `ty`: as it is when `ty` is an optional
    /// (`map[key]`, spec 2.2); else the value under the key, for a change
    /// to it (`d[k].push(x)`, `d[k] += [x]`), and a missing key panics,
    /// naming it.
    pub(crate) fn dict_entry(&mut self, get: ir::Expr) -> ir::Expr {
        let (ty, line) = (get.ty, get.line);
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let is_optional = matches!(self.shapes.shape(ty), Some(Shape::Optional(_)));
        let K::Rt(RtOp::MapGet, mut args) = get.kind else {
            return get;
        };
        if is_optional || args.len() != 2 {
            return at(K::Rt(RtOp::MapGet, args), ty);
        }
        let key = args.pop().expect("a key");
        let map = args.pop().expect("a map");
        let opt = self.optional_type(ty);
        let (k, v) = (self.new_local(key.ty), self.new_local(opt));
        let text = self.str_type();
        let message = at(
            K::Format(vec![
                at(K::Str("the dict has no key `".into()), text),
                self.local_expr(k, line),
                at(K::Str("`".into()), text),
            ]),
            text,
        );
        let missing = at(
            K::Unary(
                ir::UnOp::Not,
                Box::new(at(
                    K::IsSome(Box::new(self.local_expr(v, line))),
                    Type::Bool,
                )),
            ),
            Type::Bool,
        );
        let panic = Stmt::Expr(at(K::Rt(RtOp::PanicText, vec![message]), Type::Unit));
        let value = at(K::Unwrap(Box::new(self.local_expr(v, line))), ty);
        let check = at(
            K::If(
                Box::new(missing),
                Block {
                    stmts: vec![panic],
                    value: None,
                },
                Block::default(),
            ),
            Type::Unit,
        );
        at(
            K::Block(Block {
                stmts: vec![
                    Stmt::Let(k, key),
                    Stmt::Let(
                        v,
                        at(
                            K::Rt(RtOp::MapGet, vec![map, self.local_expr(k, line)]),
                            opt,
                        ),
                    ),
                    Stmt::Expr(check),
                ],
                value: Some(Box::new(value)),
            }),
            ty,
        )
    }
}
