// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Whole numbers with their own limits in native code (spec 2.1):
//! `Int<bits: 24>`, `UInt<bits: 12>` and `Int<range: 1..=65535>`. Their
//! values are `I64`s; each operator, conversion and method checks the
//! type's limits through `RtOp::IntWhole`, which panics with the
//! interpreter's words. A small type adds, subtracts and multiplies
//! inline, calling the runtime only when the result is out of range.

use crate::ir::{self, BinOp, Block, ExprKind as K, Shape, Stmt, Type, WHOLE_OPS, type_code};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;
use kavach_checker::{Prim, Ty};

/// The place of `op` in `WHOLE_OPS`.
fn op_code(op: &str) -> i128 {
    WHOLE_OPS.iter().position(|o| *o == op).unwrap_or(0) as i128
}

impl<'a> Lowerer<'a> {
    /// The type of `e` when it is a whole number with its own limits.
    pub(crate) fn whole_of(&self, e: &ast::Expr) -> Option<Prim> {
        match self.c.expr_types.get(&(e.span.start, e.span.end))? {
            Ty::Prim(p) if p.is_narrow() => Some(*p),
            _ => None,
        }
    }

    /// The KIR type of a written `Int<bits: n>`, `UInt<bits: n>` or
    /// `Int<range: …>`: `Int8` … `UInt64` for their widths, else `Int`.
    pub(crate) fn written_int(&self, name: &str, args: &[ast::TypeExpr]) -> Type {
        let bits = args.iter().find_map(|a| match &a.kind {
            ast::TypeKind::Num {
                name: Some(n),
                value,
            } if n.name == "bits" => match &value.kind {
                ast::ExprKind::Int(v) => i64::try_from(*v).ok(),
                ast::ExprKind::Name(id) => (self.c.resolution.uses.get(&id.span.start))
                    .and_then(|d| self.c.numbers.consts.get(d).copied()),
                _ => None,
            },
            _ => None,
        });
        match bits.and_then(|b| u16::try_from(b).ok()) {
            Some(b) => crate::types::prim_type(kavach_checker::bits_type(b, name == "Int"))
                .unwrap_or(Type::I64),
            None => Type::I64,
        }
    }

    /// The whole-number type with its own limits the checker gave `span`.
    pub(crate) fn whole_at(&self, span: ast::Span) -> Option<Prim> {
        match self.c.expr_types.get(&(span.start, span.end))? {
            Ty::Prim(p) if p.is_narrow() => Some(*p),
            _ => None,
        }
    }

    /// A generic function or method given a whole number with its own
    /// limits (or a value holding one): each copy of a generic function
    /// is made for the `Int` its values are kept in, so its operators
    /// would not check the limits. Not built yet in native code.
    pub(crate) fn no_whole_in_generic(&self, call: &ast::Expr, args: &[ast::Arg]) -> Lowered<()> {
        let mut parts: Vec<&ast::Expr> = args.iter().map(|a| &a.value).collect();
        parts.push(call);
        if let ast::ExprKind::Call { callee, .. } = &call.kind
            && let ast::ExprKind::Field { base, .. } = &callee.kind
        {
            parts.push(base);
        }
        for e in parts {
            let Some(t) = self.c.expr_types.get(&(e.span.start, e.span.end)) else {
                continue;
            };
            if self.holds_whole(t, false, &mut std::collections::HashSet::new()) {
                return Err(self.unsupported(
                    "a generic function given an `Int<bits: n>` or an `Int<range: …>`",
                    call.span,
                ));
            }
        }
        Ok(())
    }

    /// Whether the call at `span` decodes JSON into a type that holds a
    /// whole number with its own limits, at any depth: its limits are
    /// not checked by the native decoder yet.
    pub(crate) fn decodes_whole(&self, span: ast::Span) -> bool {
        let Some(t) = self.c.expr_types.get(&(span.start, span.end)) else {
            return false;
        };
        let mut seen = std::collections::HashSet::new();
        self.holds_whole(t, true, &mut seen)
    }

    fn holds_whole(
        &self,
        t: &Ty,
        fields: bool,
        seen: &mut std::collections::HashSet<kavach_resolver::DefId>,
    ) -> bool {
        match t {
            Ty::Prim(p) => p.is_narrow(),
            Ty::List(x) | Ty::Set(x) | Ty::Optional(x) => self.holds_whole(x, fields, seen),
            Ty::Map(k, v) | Ty::Result(k, v) => {
                self.holds_whole(k, fields, seen) || self.holds_whole(v, fields, seen)
            }
            Ty::Tuple(ts) | Ty::Builtin(_, ts) => {
                ts.iter().any(|t| self.holds_whole(t, fields, seen))
            }
            Ty::Anon(fs) => fs.iter().any(|(_, t)| self.holds_whole(t, fields, seen)),
            Ty::Sized(x, _, _) | Ty::Frozen(x, _, _) => self.holds_whole(x, fields, seen),
            Ty::Adt(def, args) => {
                if !seen.insert(*def) {
                    return false;
                }
                let defs = &self.c.resolution.defs;
                let of_def = |d: kavach_resolver::DefId| {
                    d == *def
                        || matches!(defs[d].kind, kavach_resolver::DefKind::Variant { of } if of == *def)
                };
                args.iter().any(|a| self.holds_whole(a, fields, seen))
                    || (fields
                        && (self.c.field_types.iter())
                            .any(|((d, _), t)| of_def(*d) && self.holds_whole(t, true, seen)))
            }
            _ => false,
        }
    }

    /// The code `IntWhole` reads `p`'s limits and name from: its
    /// `Shape::Whole`, or `Int8` … `UInt64`'s own type code.
    fn whole_code(&mut self, p: Prim) -> i128 {
        let ty = match p {
            Prim::Bits(n, _) => {
                let (lo, hi) = p.int_range().unwrap_or((0, 0));
                self.shapes.intern(Shape::Whole {
                    lo: lo as i64,
                    hi: hi as i64,
                    bits: n,
                })
            }
            Prim::Ranged(lo, hi) => self.shapes.intern(Shape::Whole { lo, hi, bits: 0 }),
            p => crate::types::prim_type(p).unwrap_or(Type::I64),
        };
        i128::from(type_code(ty))
    }

    /// `IntWhole` doing `op` on `a` and `b` in type `p`, giving `ty`.
    fn whole_call(
        &mut self,
        op: &str,
        a: ir::Expr,
        b: ir::Expr,
        p: Prim,
        ty: Type,
        line: u32,
    ) -> ir::Expr {
        let c = (op_code(op) << 32) | self.whole_code(p);
        let c = self.at(K::Int(c), Type::I64, line);
        self.at(K::Rt(RtOp::IntWhole, vec![a, b, c]), ty, line)
    }

    /// `l op r` where both are of type `p` (spec 2.1): the result is
    /// checked against `p`'s limits, as overflow is (5.6).
    pub(crate) fn whole_binary(
        &mut self,
        op: ast::BinaryOp,
        l: ir::Expr,
        r: ir::Expr,
        p: Prim,
        line: u32,
    ) -> ir::Expr {
        use ast::BinaryOp as B;
        let symbol = op.as_str();
        let (lo, hi) = p.int_range().unwrap_or((0, 0));
        // `&`, `|` and `^` of two numbers of `n` bits have `n` bits.
        if matches!(p, Prim::Bits(..)) && matches!(op, B::BitAnd | B::BitOr | B::BitXor) {
            let bin = crate::lower_expr::binary_op(op).unwrap_or(BinOp::BitAnd);
            return self.at(K::Binary(bin, Box::new(l), Box::new(r)), Type::I64, line);
        }
        let small = lo >= -(1 << 31) && hi <= 1 << 31;
        let fast = match op {
            B::Add => Some(ir::FastOp::Add),
            B::Sub => Some(ir::FastOp::Sub),
            B::Mul => Some(ir::FastOp::Mul),
            _ => None,
        };
        let Some(fast) = fast.filter(|_| small) else {
            return self.whole_call(symbol, l, r, p, Type::I64, line);
        };
        // Both are within 2³¹, so the exact result fits an `Int`.
        let (x, y, t) = (
            self.new_local(Type::I64),
            self.new_local(Type::I64),
            self.new_local(Type::I64),
        );
        let get = |me: &Self, l| me.local_expr(l, line);
        let int = |me: &Self, n: i128| me.at(K::Int(n), Type::I64, line);
        let sum = self.at(
            K::Fast(fast, Box::new(get(self, x)), Box::new(get(self, y))),
            Type::I64,
            line,
        );
        let above = self.at(
            K::Binary(BinOp::Ge, Box::new(get(self, t)), Box::new(int(self, lo))),
            Type::Bool,
            line,
        );
        let below = self.at(
            K::Binary(BinOp::Le, Box::new(get(self, t)), Box::new(int(self, hi))),
            Type::Bool,
            line,
        );
        let inside = self.at(K::And(Box::new(above), Box::new(below)), Type::Bool, line);
        let (gx, gy) = (get(self, x), get(self, y));
        let checked = self.whole_call(symbol, gx, gy, p, Type::I64, line);
        let value = self.at(
            K::If(
                Box::new(inside),
                crate::lower_expr::wrap(get(self, t), true),
                crate::lower_expr::wrap(checked, true),
            ),
            Type::I64,
            line,
        );
        let stmts = vec![Stmt::Let(x, l), Stmt::Let(y, r), Stmt::Let(t, sum)];
        self.at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(value)),
            }),
            Type::I64,
            line,
        )
    }

    /// `-x` and `~x` of type `p`.
    pub(crate) fn whole_unary(&mut self, op: &str, x: ir::Expr, p: Prim, line: u32) -> ir::Expr {
        let zero = self.at(K::Int(0), Type::I64, line);
        self.whole_call(op, x, zero, p, Type::I64, line)
    }

    /// `x.wrapping_add(y)`, `x.checked_add(y)` and the others on a number
    /// of type `p` (spec 5.6), at its limits. `None` for other methods.
    pub(crate) fn whole_method(
        &mut self,
        name: &str,
        values: &mut Vec<ir::Expr>,
        p: Prim,
        ty: Type,
        line: u32,
    ) -> Option<ir::Expr> {
        if !name.starts_with("wrapping_") && !name.starts_with("checked_") {
            return None;
        }
        if !WHOLE_OPS.contains(&name) || values.len() != 2 {
            return None;
        }
        let b = values.pop()?;
        let a = values.pop()?;
        Some(self.whole_call(name, a, b, p, ty, line))
    }

    /// The type of conversion `name(…)` at `span` when it checks a whole
    /// number's limits itself, and whether it is under `try` or `catch`.
    pub(crate) fn whole_target(&self, name: &str, span: ast::Span) -> Option<(Prim, bool)> {
        let p = Prim::from_name(name).or((name == "UInt").then_some(Prim::UInt64))?;
        if !p.is_int() {
            return None;
        }
        let checked = self.checked_ctor == Some((span.start, span.end));
        let p = match self.c.expr_types.get(&(span.start, span.end))? {
            Ty::Prim(p) => *p,
            Ty::Result(ok, _) => match **ok {
                Ty::Prim(p) => p,
                _ => return None,
            },
            _ => return None,
        };
        (p.is_narrow() || checked).then_some((p, checked))
    }

    /// `T(x)` for a whole-number type `p` with its own limits, or any
    /// whole-number type under `try` or `catch` (spec 2.1): a value that
    /// doesn't fit panics, or there sets the `SizeError` `outcome` reads.
    pub(crate) fn whole_convert(
        &mut self,
        arg: &'a ast::Expr,
        p: Prim,
        ty: Type,
        line: u32,
        checked: bool,
    ) -> Lowered<ir::Expr> {
        let mut value = self.expr(arg)?;
        // A `Float32` is shown as the `Float` it becomes, as the
        // interpreter shows it; a `Decimal` or `BigInt` becomes an `Int`.
        if value.ty == Type::F32 {
            value = self.at(K::Convert(Box::new(value), false), Type::F64, line);
        } else if value.ty.is_ref() {
            value = self.at(K::Convert(Box::new(value), true), Type::I64, line);
        }
        let from = value.ty;
        let held = self.new_local(from);
        let get = |me: &Self| me.local_expr(held, line);
        let code = self.at(K::Int(i128::from(type_code(from))), Type::I64, line);
        let fits = self.whole_call("fits", get(self), code, p, Type::Bool, line);
        let text_ty = self.str_type();
        let why = self.at(
            K::Format(vec![
                get(self),
                self.at(
                    K::Str(format!(" doesn't fit in {}", p.name())),
                    text_ty,
                    line,
                ),
            ]),
            text_ty,
            line,
        );
        let bad = self.at(K::Unary(ir::UnOp::Not, Box::new(fits)), Type::Bool, line);
        let mut stmts = vec![Stmt::Let(held, value)];
        let on_bad = if checked {
            let err_ty = self.error_type();
            let error = self.new_local(err_ty);
            stmts.push(Stmt::Let(error, self.at(K::None, err_ty, line)));
            self.pending_error = Some(error);
            self.at(K::Assign(error, Box::new(why)), Type::Unit, line)
        } else {
            self.at(K::Rt(RtOp::PanicText, vec![why]), Type::Unit, line)
        };
        stmts.push(Stmt::Expr(self.at(
            K::If(
                Box::new(bad.clone()),
                crate::lower_expr::wrap(on_bad, false),
                Block::default(),
            ),
            Type::Unit,
            line,
        )));
        // Proven to fit, or not used: no second check.
        let converted = if from == ty {
            get(self)
        } else {
            self.at(K::Convert(Box::new(get(self)), false), ty, line)
        };
        let result = if checked {
            let zero = match ty {
                Type::F64 | Type::F32 => K::Float(0.0),
                _ => K::Int(0),
            };
            self.at(
                K::If(
                    Box::new(bad),
                    crate::lower_expr::wrap(self.at(zero, ty, line), true),
                    crate::lower_expr::wrap(converted, true),
                ),
                ty,
                line,
            )
        } else {
            converted
        };
        Ok(self.at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(result)),
            }),
            ty,
            line,
        ))
    }
}
