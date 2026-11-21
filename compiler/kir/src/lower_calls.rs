// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Lowering calls: functions, built-ins, and methods of records, text and
//! lists.

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use crate::types::prim_type;
use kavach_ast::{self as ast, ExprKind, StrPart};
use kavach_checker::{Prim, Ty};
use kavach_resolver::{DefId, DefKind};
use std::collections::HashMap;

impl<'a> Lowerer<'a> {
    pub(crate) fn call(
        &mut self,
        e: &'a ast::Expr,
        callee: &'a ast::Expr,
        args: &'a [ast::Arg],
        ty: Type,
    ) -> Lowered<ir::Expr> {
        // `std.reflect`'s answers, worked out while compiling (14.32).
        if let Some(answer) = self.reflected(e, ty) {
            return answer;
        }
        if let Some(schema) = self.schema_call(e, callee, ty) {
            return schema;
        }
        if let Some(described) = self.describe_call(e, callee, args, ty) {
            return described;
        }
        let line = self.line(e.span);
        let res = &self.c.resolution;
        match &callee.kind {
            ExprKind::Name(id) => {
                let Some(&def) = res.uses.get(&id.span.start) else {
                    return Err(self.unsupported("this call", e.span));
                };
                match res.defs[def].kind {
                    DefKind::Fn if self.generic_fns.contains_key(&def) => {
                        self.generic_call(def, e, args, ty, line)
                    }
                    DefKind::Fn => {
                        let fid = self.fn_ids[&def];
                        if self.decl_of(fid).c_library.is_some() {
                            return self.c_call(fid, args, ty, line);
                        }
                        let values = self.arguments(self.decl_of(fid), args, e.span)?;
                        let call = ir::Expr {
                            kind: K::Call(fid, values),
                            ty,
                            line,
                        };
                        self.write_back(fid, call, args, 0, ty)
                    }
                    DefKind::Type => self.construct(def, None, args, ty, line, e.span),
                    DefKind::Variant { of } => {
                        self.construct(of, Some(&id.name), args, ty, line, e.span)
                    }
                    DefKind::Builtin => self.builtin_call(&id.name, args, ty, line, e.span),
                    // `T(x)`: `x` converted to this copy's `T` (spec 2.9).
                    DefKind::GenericParam => match args {
                        [a] => {
                            let v = self.expr(&a.value)?;
                            Ok(ir::Expr {
                                kind: K::Convert(Box::new(v), true),
                                ty,
                                line,
                            })
                        }
                        _ => Err(self.unsupported("this conversion", e.span)),
                    },
                    DefKind::Local { .. } | DefKind::Param => {
                        self.call_through(callee, args, e.span)
                    }
                    _ => Err(self.unsupported("calling a function value", e.span)),
                }
            }
            // `(0..n).to_list()`: the range, built as a list (spec 2.2).
            ExprKind::Field { base, name, .. }
                if name.name == "to_list"
                    && args.is_empty()
                    && matches!(
                        base.kind,
                        ExprKind::Binary(ast::BinaryOp::Range | ast::BinaryOp::RangeInclusive, ..)
                    ) =>
            {
                self.expr(base)
            }
            ExprKind::Field {
                base,
                name,
                optional: false,
            } => self.method_call(base, name, args, ty, line, e),
            _ => self.call_through(callee, args, e.span),
        }
    }

    /// Calls whatever function value `callee` gives (spec 3.4).
    fn call_through(
        &mut self,
        callee: &'a ast::Expr,
        args: &'a [ast::Arg],
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let f = self.expr(callee)?;
        let values = args
            .iter()
            .map(|a| self.expr(&a.value))
            .collect::<Lowered<Vec<_>>>()?;
        self.call_value(f, values, span)
    }

    /// Arguments in parameter order, converted to the parameter types.
    pub(crate) fn arguments(
        &mut self,
        f: &'a ast::FnDecl,
        args: &'a [ast::Arg],
        span: ast::Span,
    ) -> Lowered<Vec<ir::Expr>> {
        let values = self.argument_values(f, args, span, &HashMap::new())?;
        let params = f.params.iter().filter_map(|p| p.ty.as_ref());
        let mut out = Vec::new();
        for (value, ty) in values.into_iter().zip(params) {
            let want = self.written_type(ty)?;
            out.push(self.coerce(value, want)?);
        }
        Ok(out)
    }

    /// Arguments in parameter order: positional, named, then defaults. A
    /// default is lowered with the callee's type arguments `callee` known
    /// so far: `mean: T = 0.0` is a `Float32` for a `Float32` `T`.
    pub(crate) fn argument_values(
        &mut self,
        f: &'a ast::FnDecl,
        args: &'a [ast::Arg],
        span: ast::Span,
        callee: &HashMap<DefId, Type>,
    ) -> Lowered<Vec<ir::Expr>> {
        let params: Vec<&ast::Param> = f.params.iter().filter(|p| p.ty.is_some()).collect();
        let given_at = |i: usize, p: &ast::Param| {
            (args.iter())
                .find(|a| a.name.as_ref().is_some_and(|n| n.name == p.name.name))
                .or_else(|| args.get(i).filter(|a| a.name.is_none()))
        };
        // A default may read the parameters before it (spec 3.2): each
        // value is then kept in a local the default's names stand for.
        let defaulted = (params.iter().enumerate()).any(|(i, p)| given_at(i, p).is_none());
        let mut bound = Vec::new();
        let mut values = Vec::new();
        for (i, p) in params.iter().enumerate() {
            let mut value = match (given_at(i, p), &p.default) {
                (Some(a), _) => self.expr(&a.value)?,
                (None, Some(d)) => {
                    let outer = self.subst.clone();
                    self.subst.extend(callee.iter().map(|(k, v)| (*k, *v)));
                    let value = self.expr(d);
                    self.subst = outer;
                    value?
                }
                (None, None) => return Err(self.unsupported("this call", span)),
            };
            if defaulted {
                let line = value.line;
                let held = self.new_local(value.ty);
                let read = self.local_expr(held, line);
                value = ir::Expr {
                    ty: value.ty,
                    line,
                    kind: K::Block(crate::ir::Block {
                        stmts: vec![crate::ir::Stmt::Let(held, value)],
                        value: Some(Box::new(read)),
                    }),
                };
                if let Some(&def) = self.decl.get(&p.name.span.start) {
                    bound.push((def, self.local_ids.get(&def).copied()));
                }
                self.bind(&p.name, held);
            }
            values.push(value);
        }
        for (def, before) in bound {
            match before {
                Some(l) => self.local_ids.insert(def, l),
                None => self.local_ids.remove(&def),
            };
        }
        Ok(values)
    }

    fn builtin_call(
        &mut self,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        if let Some(made) = self.sized_builtin(name, args, ty, line, span) {
            return made;
        }
        if let ("log_write", [level, text, event]) = (name, args) {
            return self.log_write_call(level, text, event, line);
        }
        if name == "random_bits" {
            return Ok(ir::Expr {
                kind: K::Rt(RtOp::RandomBits, Vec::new()),
                ty,
                line,
            });
        }
        if name == "random_ints" {
            let args = args
                .iter()
                .map(|a| self.expr(&a.value))
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(ir::Expr {
                kind: K::Rt(RtOp::RandomInts, args),
                ty,
                line,
            });
        }
        // `TextBuilder()`, `TextBuilder(text)` (spec 2.18): text here.
        if name == "TextBuilder" {
            return match args.first() {
                Some(a) => self.expr(&a.value),
                None => Ok(ir::Expr {
                    kind: K::Str(String::new()),
                    ty,
                    line,
                }),
            };
        }
        let arg = args
            .first()
            .map(|a| &a.value)
            .ok_or_else(|| self.unsupported(format!("`{name}()`"), span))?;
        let kind = match name {
            "panic" => match &arg.kind {
                ExprKind::Str(parts) if parts.iter().all(|p| matches!(p, StrPart::Text(_))) => {
                    K::Panic(
                        parts
                            .iter()
                            .map(|p| {
                                if let StrPart::Text(t) = p {
                                    t.as_str()
                                } else {
                                    ""
                                }
                            })
                            .collect(),
                    )
                }
                _ => K::Rt(RtOp::PanicText, vec![self.expr(arg)?]),
            },
            "alloc" | "free" => return self.ptr_builtin(name, arg, line, span),
            "Shared" => K::Rt(RtOp::SharedNew, vec![self.expr(arg)?]),
            "Channel" => K::Rt(RtOp::ChanNew, vec![self.expr(arg)?]),
            "Bytes" => K::Rt(RtOp::BytesNew, vec![self.expr(arg)?]),
            "env" => K::Rt(RtOp::EnvGet, vec![self.expr(arg)?]),
            // `Error("disk full")`: an error that is only its message (spec 5.5).
            "Error" => {
                let message = self.expr(arg)?;
                return self.coerce(message, ty);
            }
            // `reveal` only lets a secret through the compiler's checks
            // (spec 8.7); at run time it is the value itself.
            "reveal" => return self.expr(arg),
            "ask" => return self.ask_call(span, arg, line),
            "sum" => match self.rule_sum(arg, ty, line)? {
                Some(total) => return Ok(total),
                None => return Err(self.unsupported("`sum` outside a `limit` rule", span)),
            },
            // `String(x)` (spec 2.1): its text, as `"{x}"` gives it.
            "String" => K::Format(vec![self.expr(arg)?]),
            // `Int(text)` (spec 2.1): read, so it can fail.
            _ if name != "String" && (Prim::from_name(name).is_some()) && self.is_text(arg) => {
                return self.convert_text(arg, ty, line, span);
            }
            "Char" if self.ty_of(arg)?.is_int() => return self.char_of(arg, ty, line),
            // `UInt<bits: 12>(x)`, and `try Int8(x)` (spec 2.1).
            _ if let Some((p, checked)) = self.whole_target(name, span) => {
                return self.whole_convert(arg, p, ty, line, checked);
            }
            _ if Prim::from_name(name).and_then(prim_type).is_some()
                || matches!(name, "Decimal" | "BigInt") =>
            {
                K::Convert(Box::new(self.expr(arg)?), true)
            }
            _ => return Err(self.unsupported(format!("`{name}`"), span)),
        };
        Ok(ir::Expr { kind, ty, line })
    }

    pub(crate) fn is_mutating_call(&self, callee: &ast::Expr) -> bool {
        let ExprKind::Field { base, name, .. } = &callee.kind else {
            return false;
        };
        matches!(
            (
                name.name.as_str(),
                self.c.expr_types.get(&(base.span.start, base.span.end))
            ),
            ("push", Some(Ty::List(_) | Ty::Builtin("Ring", _)))
                | ("swap", Some(Ty::List(_)))
                | ("remove", Some(Ty::Map(..) | Ty::Set(_)))
                | ("add", Some(Ty::Set(_)))
                | (
                    "push" | "push_char" | "insert" | "replace" | "remove" | "clear",
                    Some(Ty::Builtin("TextBuilder", _))
                )
        )
    }
}

/// The name `extend` uses for a scalar type.
pub(crate) fn scalar_name(t: Type) -> &'static str {
    match t {
        Type::F64 => "Float",
        Type::F32 => "Float32",
        Type::Bool => "Bool",
        Type::Char => "Char",
        t => t.kavach_name(),
    }
}

impl<'a> Lowerer<'a> {
    /// `Char(n)` of a whole number (spec 2.1): a panic unless `n` is a
    /// character's code point, `0..=0x10FFFF` without the surrogates.
    fn char_of(&mut self, arg: &'a ast::Expr, ty: Type, line: u32) -> Lowered<ir::Expr> {
        use crate::ir::{BinOp, Block, Stmt};
        let n = self.expr(arg)?;
        let held = self.new_local(Type::I64);
        let wide = ir::Expr {
            kind: K::Convert(Box::new(n), false),
            ty: Type::I64,
            line,
        };
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let get = |s: &Self| s.local_expr(held, line);
        let cmp = |s: &Self, op, v: i128| {
            at(
                K::Binary(op, Box::new(get(s)), Box::new(at(K::Int(v), Type::I64))),
                Type::Bool,
            )
        };
        let low = at(
            K::And(
                Box::new(cmp(self, BinOp::Ge, 0)),
                Box::new(cmp(self, BinOp::Lt, 0xD800)),
            ),
            Type::Bool,
        );
        let high = at(
            K::And(
                Box::new(cmp(self, BinOp::Gt, 0xDFFF)),
                Box::new(cmp(self, BinOp::Le, 0x10FFFF)),
            ),
            Type::Bool,
        );
        let cond = at(K::Or(Box::new(low), Box::new(high)), Type::Bool);
        let value = at(K::Convert(Box::new(get(self)), false), ty);
        // The same words as the interpreter's: `55296 is not a …`.
        let text = self.str_type();
        let why = at(
            K::Format(vec![
                get(self),
                at(K::Str(" is not a character's code point".into()), text),
            ]),
            text,
        );
        let stop = Block {
            stmts: vec![Stmt::Expr(at(
                K::Rt(crate::rt_op::RtOp::PanicText, vec![why]),
                Type::Unit,
            ))],
            value: None,
        };
        let bad = at(K::Unary(crate::ir::UnOp::Not, Box::new(cond)), Type::Bool);
        let check = at(K::If(Box::new(bad), stop, Block::default()), Type::Unit);
        Ok(at(
            K::Block(Block {
                stmts: vec![Stmt::Let(held, wide), Stmt::Expr(check)],
                value: Some(Box::new(value)),
            }),
            ty,
        ))
    }
}
