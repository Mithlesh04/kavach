// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Numbers in types (spec 2.2, 2.9), as the interpreter's `sizes.rs`
//! runs them: a value made where its type has a `max` gets that limit,
//! as `List.capacity(n)` gives one; `try List<Int, max: 2>(xs)` fails
//! with a `SizeError` when `xs` doesn't fit; and a number parameter is
//! a number of the function's instance (`Shape::Num`).

use crate::ir::{self, BinOp, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, ExprKind};
use kavach_checker::{SizeKind, Ty};
use kavach_resolver::DefId;
use std::collections::HashMap;

impl<'a> Lowerer<'a> {
    /// The number `t` stands for in this instance: a `Num`, or a number
    /// parameter's.
    pub(crate) fn number_of(&self, t: &Ty) -> Option<i64> {
        match t {
            Ty::Num(n) => Some(*n),
            Ty::Param(p) => match self.shapes.shape(*self.subst.get(p)?) {
                Some(Shape::Num(n)) => Some(*n),
                _ => None,
            },
            _ => None,
        }
    }

    /// The type standing for number `t` as a type argument.
    fn number_type(&mut self, t: &Ty, span: ast::Span) -> Lowered<Type> {
        let n = self
            .number_of(t)
            .ok_or_else(|| self.unsupported("this number in a type", span))?;
        Ok(self.shapes.intern(Shape::Num(n)))
    }

    /// The number parameters the checker settled for call `span` (spec
    /// 2.9), added to `subst`.
    pub(crate) fn call_numbers(
        &mut self,
        span: ast::Span,
        subst: &mut HashMap<DefId, Type>,
    ) -> Lowered<()> {
        let Some(nums) = self.c.numbers.calls.get(&(span.start, span.end)) else {
            return Ok(());
        };
        for (g, t) in nums.clone() {
            let ty = self.number_type(&t, span)?;
            subst.insert(g, ty);
        }
        Ok(())
    }

    /// A number parameter's value, when `def` is one (spec 2.9).
    pub(crate) fn number_param(&self, def: DefId, line: u32) -> Option<ir::Expr> {
        if !self.c.numbers.params.contains(&def) {
            return None;
        }
        let n = self.number_of(&Ty::Param(def))?;
        Some(self.at(K::Int(i128::from(n)), Type::I64, line))
    }

    /// The limit value `e` gets from its type, if any.
    pub(crate) fn limit_of(&self, e: &ast::Expr) -> Option<Ty> {
        let limits = &self.c.numbers.limits;
        if limits.is_empty()
            || !matches!(
                e.kind,
                ExprKind::List(_) | ExprKind::Map(_) | ExprKind::Call { .. }
            )
        {
            return None;
        }
        // A built-in type called by name makes its own (`sized_builtin`).
        if let ExprKind::Call { callee, .. } = &e.kind
            && let ExprKind::Name(id) = &callee.kind
            && matches!(id.name.as_str(), "List" | "Dict" | "Set" | "Bytes" | "Ring")
        {
            return None;
        }
        limits.get(&(e.span.start, e.span.end)).cloned()
    }

    /// `made` with limit `n` (spec 2.2), as `capacity(n, made)` gives it.
    pub(crate) fn with_limit(
        &mut self,
        made: ir::Expr,
        n: &Ty,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let n = self
            .number_of(n)
            .ok_or_else(|| self.unsupported("this limit", span))?;
        let op = match self.shapes.shape(made.ty) {
            Some(Shape::List(_)) => RtOp::ListCapacity,
            // A set is kept as a map: the same call limits both.
            Some(Shape::Map(..) | Shape::Set(_)) => RtOp::MapCapacity,
            _ => return Ok(made),
        };
        let (ty, line) = (made.ty, made.line);
        let n = self.at(K::Int(i128::from(n)), Type::I64, line);
        Ok(self.at(K::Rt(op, vec![n, made]), ty, line))
    }

    /// `List<Int, max: 2>(xs)`, `Dict`, `Set`, `Bytes` with numbers, and
    /// `Ring<T, max: n>(items)` (spec 2.2, 2.9). `None` for other calls.
    pub(crate) fn sized_builtin(
        &mut self,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Option<Lowered<ir::Expr>> {
        if !matches!(name, "List" | "Dict" | "Set" | "Bytes" | "Ring") {
            return None;
        }
        let key = (span.start, span.end);
        if name == "Ring" {
            return Some(self.ring_made(args, ty, line, span));
        }
        let check = self.c.numbers.checks.get(&key).cloned();
        if check.is_none() && name == "Bytes" {
            return None;
        }
        let arg = args.first()?;
        let Some((kind, n, shown)) = check else {
            // A literal, checked while compiling.
            return Some(self.expr(&arg.value));
        };
        Some(self.size_checked(&arg.value, kind, &n, &shown, ty, line, span))
    }

    /// `List<T, len: n>.filled(v)`: a loop pushes `v` `n` times (spec
    /// 2.2), `v` worked out once.
    pub(crate) fn list_filled(
        &mut self,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let n = (self.c.numbers.filled.get(&(span.start, span.end)).cloned())
            .and_then(|t| self.number_of(&t))
            .ok_or_else(|| self.unsupported("this `filled`", span))?;
        let Some(Shape::List(item_ty)) = self.shapes.shape(ty).cloned() else {
            return Err(self.unsupported("this `filled`", span));
        };
        let arg = args
            .first()
            .ok_or_else(|| self.unsupported("`filled()`", span))?;
        let value = self.expr(&arg.value)?;
        let value = self.coerce(value, item_ty)?;
        let (acc, item) = (self.new_local(ty), self.new_local(item_ty));
        let var = self.new_local(Type::I64);
        let empty = self.at(K::List(Vec::new()), ty, line);
        let body = self.push_to(acc, self.local_expr(item, line));
        let fill = Stmt::ForRange {
            var,
            start: self.at(K::Int(0), Type::I64, line),
            end: self.at(K::Int(i128::from(n)), Type::I64, line),
            inclusive: false,
            body,
        };
        let stmts = vec![Stmt::Let(item, value), Stmt::Let(acc, empty), fill];
        let value = Some(Box::new(self.local_expr(acc, line)));
        Ok(self.at(K::Block(Block { stmts, value }), ty, line))
    }

    /// `Ring<T, max: n>(items)` or `Ring()`: the last `n` items.
    fn ring_made(
        &mut self,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let limit = self.c.numbers.limits.get(&(span.start, span.end)).cloned();
        let n = limit
            .and_then(|t| self.number_of(&t))
            .ok_or_else(|| self.unsupported("this ring", span))?;
        let first = match args.first() {
            Some(a) => self.expr(&a.value)?,
            None => self.at(K::Int(0), Type::I64, line),
        };
        let n = self.at(K::Int(i128::from(n)), Type::I64, line);
        Ok(self.at(K::Rt(RtOp::RingCapacity, vec![n, first]), ty, line))
    }

    /// `value` checked against its size: when it doesn't fit, the error
    /// `outcome` reads (`pending_error`) is set to the `SizeError`'s text.
    #[allow(clippy::too_many_arguments)]
    fn size_checked(
        &mut self,
        value: &'a ast::Expr,
        kind: SizeKind,
        n: &Ty,
        shown: &str,
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let n = self
            .number_of(n)
            .ok_or_else(|| self.unsupported("this size", span))?;
        let made = self.expr(value)?;
        let (len_op, what) = match self.shapes.shape(made.ty) {
            Some(Shape::List(_)) => (RtOp::ListLen, "list"),
            Some(Shape::Map(..)) => (RtOp::MapLen, "dict"),
            Some(Shape::Set(_)) => (RtOp::MapLen, "set"),
            Some(Shape::Bytes) => (RtOp::BytesLen, "bytes"),
            _ => return Err(self.unsupported("this conversion", span)),
        };
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let (err_ty, text_ty) = (self.error_type(), self.str_type());
        let (tmp, count, error) = (
            self.new_local(made.ty),
            self.new_local(Type::I64),
            self.new_local(err_ty),
        );
        let read = |l| at(K::Local(l), made.ty);
        let len = at(K::Rt(len_op, vec![read(tmp)]), Type::I64);
        let mut stmts = vec![
            Stmt::Let(tmp, made.clone()),
            Stmt::Let(count, len),
            Stmt::Let(error, at(K::None, err_ty)),
        ];
        let count_of = || at(K::Local(count), Type::I64);
        let limit = || at(K::Int(i128::from(n)), Type::I64);
        let op = match kind {
            SizeKind::Max => BinOp::Gt,
            SizeKind::Len => BinOp::Ne,
        };
        let bad = at(
            K::Binary(op, Box::new(count_of()), Box::new(limit())),
            Type::Bool,
        );
        // The same text the interpreter gives (`size_error`).
        let shown = shown.replace("{n}", &n.to_string());
        let one = |count: usize| kavach_runtime_core::size_error(what, count, &shown);
        let (single, many) = (one(1), one(2));
        let (head, tail) = many.split_once('2').unwrap_or((&many, ""));
        let plural = at(
            K::Format(vec![
                at(K::Str(head.to_string()), text_ty),
                count_of(),
                at(K::Str(tail.to_string()), text_ty),
            ]),
            text_ty,
        );
        let is_one = at(
            K::Binary(
                BinOp::Eq,
                Box::new(count_of()),
                Box::new(at(K::Int(1), Type::I64)),
            ),
            Type::Bool,
        );
        let text = at(
            K::If(
                Box::new(is_one),
                crate::lower_expr::wrap(at(K::Str(single), text_ty), true),
                crate::lower_expr::wrap(plural, true),
            ),
            text_ty,
        );
        let set = at(K::Assign(error, Box::new(text)), Type::Unit);
        stmts.push(Stmt::Expr(at(
            K::If(
                Box::new(bad.clone()),
                Block {
                    stmts: vec![Stmt::Expr(set)],
                    value: None,
                },
                Block::default(),
            ),
            Type::Unit,
        )));
        // A value that fits `max` keeps the limit (spec 2.2).
        let result = match (kind, len_op) {
            (SizeKind::Max, RtOp::ListLen | RtOp::MapLen) => {
                let capacity = if len_op == RtOp::ListLen {
                    RtOp::ListCapacity
                } else {
                    RtOp::MapCapacity
                };
                let limited = at(K::Rt(capacity, vec![limit(), read(tmp)]), made.ty);
                at(
                    K::If(
                        Box::new(bad),
                        crate::lower_expr::wrap(read(tmp), true),
                        crate::lower_expr::wrap(limited, true),
                    ),
                    made.ty,
                )
            }
            _ => read(tmp),
        };
        self.pending_error = Some(error);
        Ok(at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(ir::Expr { ty, ..result })),
            }),
            ty,
        ))
    }
}

impl<'a> Lowerer<'a> {
    /// A written type with numbers (spec 2.9): a built-in one is its type
    /// without them; a program's own has its numbers as type arguments,
    /// in the order its parameters are declared.
    pub(crate) fn numbered_type(
        &mut self,
        t: &ast::TypeExpr,
        path: &ast::Path,
        args: &[ast::TypeExpr],
    ) -> Lowered<Type> {
        let last = path.segments.last().unwrap_or(&path.segments[0]);
        // `Int<bits: 24>`, `Int<range: 1..=9>`: kept in an `Int` (2.1),
        // and `Int<bits: 8>` is `Int8`.
        if matches!(last.name.as_str(), "Int" | "UInt") {
            return Ok(self.written_int(&last.name, args));
        }
        let def = self.c.resolution.uses.get(&last.span.start).copied();
        let is_num = |a: &ast::TypeExpr| matches!(a.kind, ast::TypeKind::Num { .. });
        let Some(def) =
            def.filter(|d| self.c.resolution.defs[*d].kind == kavach_resolver::DefKind::Type)
        else {
            let types: Vec<ast::TypeExpr> = args.iter().filter(|a| !is_num(a)).cloned().collect();
            if types.is_empty() && !matches!(last.name.as_str(), "Bytes") {
                return Err(self.unsupported("a type whose item types are left out", t.span));
            }
            let plain = ast::TypeExpr {
                kind: ast::TypeKind::Named {
                    path: path.clone(),
                    args: types,
                },
                span: t.span,
            };
            return self.written_type(&plain);
        };
        let generics = self.type_generics(def);
        let mut out: Vec<Option<Type>> = vec![None; generics.len()];
        let mut types = args.iter().filter(|a| !is_num(a));
        for (i, g) in generics.iter().enumerate() {
            let name = &self.c.resolution.defs[*g].name;
            if !self.c.numbers.params.contains(g) {
                if let Some(w) = types.next() {
                    out[i] = Some(self.written_type(w)?);
                }
                continue;
            }
            let given = args.iter().find_map(|a| match &a.kind {
                ast::TypeKind::Num {
                    name: Some(n),
                    value,
                } if n.name == *name => Some(value),
                _ => None,
            });
            let n = match given {
                Some(value) => self.written_number(value, t.span)?,
                None => (self.c.numbers.defaults.get(g).copied())
                    .ok_or_else(|| self.unsupported("a type whose numbers are left out", t.span))?,
            };
            out[i] = Some(self.shapes.intern(Shape::Num(n)));
        }
        let args = (out.into_iter())
            .map(|a| a.ok_or_else(|| self.unsupported("a type whose types are left out", t.span)))
            .collect::<Lowered<Vec<_>>>()?;
        self.adt_instance(def, args, t.span)
    }

    /// A number written in a type: a literal, a constant or a number
    /// parameter.
    fn written_number(&self, value: &ast::Expr, span: ast::Span) -> Lowered<i64> {
        match &value.kind {
            ExprKind::Int(n) => Ok(*n as i64),
            ExprKind::Unary(ast::UnaryOp::Neg, x) => Ok(-self.written_number(x, span)?),
            ExprKind::Name(id) => {
                let def = self.c.resolution.uses.get(&id.span.start).copied();
                def.and_then(|d| {
                    self.c
                        .numbers
                        .consts
                        .get(&d)
                        .copied()
                        .or_else(|| self.number_of(&Ty::Param(d)))
                })
                .ok_or_else(|| self.unsupported("this number in a type", span))
            }
            _ => Err(self.unsupported("this number in a type", span)),
        }
    }

    /// The type of a `let`'s written type `t`, or, when it leaves parts
    /// out (`List<max: 2>`), the checker's type for its variable.
    pub(crate) fn declared_type(
        &mut self,
        t: &ast::TypeExpr,
        pattern: &ast::Pattern,
    ) -> Lowered<Type> {
        match self.written_type(t) {
            Ok(ty) => Ok(ty),
            Err(e) => {
                let ast::PatternKind::Binding(id) = &pattern.kind else {
                    return Err(e);
                };
                let def = self.decl.get(&id.span.start).copied();
                match def.and_then(|d| self.c.local_types.get(&d)).cloned() {
                    Some(ty) => self.kir_type(&ty, t.span),
                    None => Err(e),
                }
            }
        }
    }
}
