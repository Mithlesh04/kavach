// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Lowering the standard library (spec 14): calls to `std` modules, their
//! constants, more methods of built-in types, and sets.

use crate::ir::{self, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, ItemKind};
use kavach_checker::Ty;
use kavach_resolver::DefId;
use std::collections::HashMap;

/// Names brought in by `use std.<module>`, and the module.
pub(crate) fn std_modules(
    c: &kavach_driver::Compiled,
    decl: &HashMap<u32, DefId>,
) -> HashMap<DefId, String> {
    let mut out = HashMap::new();
    for item in &c.file.items {
        let ItemKind::Use(u) = &item.kind else {
            continue;
        };
        let segs = &u.path.segments;
        if segs.len() == 2 && segs[0].name == "std" && u.names.is_empty() {
            let name = u.alias.as_ref().unwrap_or(&segs[1]);
            if let Some(&def) = decl.get(&name.span.start) {
                out.insert(def, segs[1].name.clone());
            }
        }
    }
    out
}

impl<'a> Lowerer<'a> {
    pub(crate) fn at(&self, kind: K, ty: Type, line: u32) -> ir::Expr {
        ir::Expr { kind, ty, line }
    }

    /// `math.pi`.
    pub(crate) fn std_constant(
        &self,
        module: &str,
        name: &str,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        match (module, name) {
            ("math", "pi") => Ok(self.at(K::Float(std::f64::consts::PI), Type::F64, line)),
            ("math", "e") => Ok(self.at(K::Float(std::f64::consts::E), Type::F64, line)),
            _ => Err(self.unsupported(format!("`{module}.{name}`"), span)),
        }
    }

    /// `fs.read_text(path)` and the other module functions (spec 14.3–14.6).
    pub(crate) fn std_call(
        &mut self,
        module: &str,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        // The functions that are one op each come first, as they came
        // before the others' wildcards (`crypto`).
        let simple = crate::std_ops::std_op(module, name);
        let op = match (module, name) {
            _ if simple.is_some() => simple.expect("just checked"),
            ("net", "connect_tls" | "listen_tls") => {
                return self.tls_call(name, args, ty, line, span);
            }
            ("crypto", "rsa_verify") => return self.rsa_verify(args, ty, line, span),
            ("crypto", _) | ("net", "websocket_accept" | "websocket_key" | "websocket_mask") => {
                return self.selected_call(module, name, args, ty, line);
            }
            ("base64", _) => return self.base64_call(name, args, ty, line, span),
            ("gpu", "pipeline" | "draw_id") => return self.draw_call(name, args, ty, line, span),
            // `text.nfkc(s)` and the other normal forms (spec 14.9).
            ("text", "nfc" | "nfd" | "nfkc" | "nfkd") => {
                let form = kavach_runtime_core::normalize::form_of(name).unwrap_or(0);
                let s = args
                    .first()
                    .ok_or_else(|| self.unsupported("`text.nfc()`", span))?;
                let s = self.expr(&s.value)?;
                let form = self.at(K::Int(i128::from(form)), Type::I64, line);
                return Ok(self.at(K::Rt(RtOp::NormalizeText, vec![s, form]), ty, line));
            }
            ("json", _) => return self.json_call(name, args, ty, line, span),
            ("time", "after") => return self.time_after(args, ty, line, span),
            // A `Duration` is its milliseconds (spec 14.4).
            ("time", "sleep" | "every") => {
                let d = args
                    .first()
                    .ok_or_else(|| self.unsupported(format!("`time.{name}()`"), span))?;
                let d = self.expr(&d.value)?;
                let ms = self.at(K::GetField(Box::new(d), 0), Type::I64, line);
                if name == "every" {
                    return Ok(ms);
                }
                return Ok(self.at(K::Rt(RtOp::TimeSleepMs, vec![ms]), Type::Unit, line));
            }
            // An interval is its period; `for` over it waits (lower_stmt.rs).
            ("time", "every_ms") => {
                let ms = args
                    .first()
                    .ok_or_else(|| self.unsupported("`every_ms()`", span))?;
                return self.expr(&ms.value);
            }
            _ => return Err(self.unsupported(format!("`{module}.{name}`"), span)),
        };
        let mut values = Vec::new();
        // Approvals in code (spec 14.16): the project's agents folder, or
        // empty text outside a project.
        if matches!(op, RtOp::AgentApprovals | RtOp::AgentAnswer) {
            let folder = self.c.agents_folder(false).unwrap_or_default();
            let text = self.str_type();
            values.push(self.at(K::Str(folder), text, line));
        }
        for a in args {
            values.push(self.expr(&a.value)?);
        }
        // An op needs at least one operand; a missing one is a 0 slot.
        if values.is_empty() {
            values.push(self.at(K::Int(0), Type::I64, line));
        }
        // `math.sqrt` of a `Float32` (spec 14.5): worked out as a `Float`,
        // then rounded once, as the interpreter does.
        if module == "math" && ty == Type::F32 {
            let wide = values
                .into_iter()
                .map(|v| self.at(K::Convert(Box::new(v), false), Type::F64, line))
                .collect();
            let r = self.at(K::Rt(op, wide), Type::F64, line);
            return Ok(self.at(K::Convert(Box::new(r), false), Type::F32, line));
        }
        Ok(self.at(K::Rt(op, values), ty, line))
    }

    /// `Set.of(a, b, …)`: stored like a map with no values.
    pub(crate) fn set_of(
        &mut self,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let Some(Shape::Set(item)) = self.shapes.shape(ty).cloned() else {
            return Err(self.unsupported("this set", ast::Span::new(0, 0)));
        };
        let mut pairs = Vec::new();
        for a in args {
            let v = self.expr(&a.value)?;
            pairs.push((self.coerce(v, item)?, self.nothing(line)));
        }
        Ok(self.at(K::Map(pairs), ty, line))
    }

    /// An expression with no value: the `Unit` "values" of a set.
    pub(crate) fn nothing(&self, line: u32) -> ir::Expr {
        self.at(K::Block(Block::default()), Type::Unit, line)
    }

    /// More methods of built-in types (spec 14.2), or `None` if `name`
    /// isn't one of them. `values` holds the receiver, then the arguments.
    pub(crate) fn more_method(
        &mut self,
        name: &str,
        mut values: Vec<ir::Expr>,
        ty: Type,
        line: u32,
    ) -> Option<Lowered<ir::Expr>> {
        let recv = values[0].ty;
        let shape = self.shapes.shape(recv).cloned();
        if matches!(shape, Some(Shape::Str))
            && let Some(e) = self.text_method(name, values.clone(), ty, line)
        {
            return Some(Ok(e));
        }
        let numeric = recv.is_int()
            || recv.is_float()
            || matches!(shape, Some(Shape::Decimal | Shape::BigInt));
        // The operator interfaces' methods (spec 2.16).
        if (numeric || matches!(shape, Some(Shape::Str)))
            && let Some(e) = self.op_method(name, values.clone(), ty, line)
        {
            return Some(Ok(e));
        }
        if numeric && matches!(name, "abs" | "min" | "max" | "clamp") {
            return Some(Ok(self.number_method(name, values, line)));
        }
        if recv.is_float() && name == "near" {
            // Passed as `Float`s (a `Float32` widens exactly), with a third
            // operand saying which tolerance (spec 14.2).
            let mut wide: Vec<ir::Expr> = values
                .into_iter()
                .map(|v| self.at(K::Convert(Box::new(v), false), Type::F64, line))
                .collect();
            wide.push(self.at(K::Int(i128::from(recv == Type::F32)), Type::I64, line));
            return Some(Ok(self.at(K::Rt(RtOp::FloatNear, wide), ty, line)));
        }
        if recv.is_int() && matches!(name, "div_floor" | "mod") {
            return Some(Ok(self.floor_method(name, values, line)));
        }
        if matches!(shape, Some(Shape::Decimal)) && matches!(name, "round" | "round_even") {
            values.push(self.at(K::Int(i128::from(name == "round_even")), Type::I64, line));
            return Some(Ok(self.at(K::Rt(RtOp::DecRound, values), ty, line)));
        }
        if recv.is_float() && matches!(name, "is_nan" | "is_finite" | "is_infinite") {
            let x = values.pop().expect("the receiver");
            return Some(Ok(self.float_check(name, x, line)));
        }
        if recv.is_float() && name == "fract" {
            let x = values.pop().expect("the receiver");
            return Some(Ok(self.fract(x, line)));
        }
        if recv == Type::F64 && name == "bits" {
            return Some(Ok(self.at(K::Rt(RtOp::FloatBits, values), ty, line)));
        }
        let op = match (shape, name) {
            (Some(Shape::Str), "replace") => RtOp::StrReplace,
            (Some(Shape::Str), "lines") => RtOp::StrLines,
            (Some(Shape::Str), "split") => RtOp::StrSplit,
            (Some(Shape::Str), "chars") => RtOp::StrChars,
            (Some(Shape::Str), "index_of") => RtOp::StrIndexOf,
            (Some(Shape::List(_)), "join") => RtOp::ListJoin,
            (Some(Shape::List(_)), "sort") => RtOp::ListSort,
            (Some(Shape::List(_)), "index_of") => RtOp::ListIndexOf,
            (Some(Shape::List(_)), "min") => RtOp::ListMin,
            (Some(Shape::List(_)), "max") => RtOp::ListMax,
            (Some(Shape::Regex), "is_match") => RtOp::RegexIsMatch,
            (Some(Shape::Regex), "find") => RtOp::RegexFind,
            (Some(Shape::Regex), "find_all") => RtOp::RegexFindAll,
            (Some(Shape::Regex), "captures") => RtOp::RegexCaptures,
            (Some(Shape::Regex), "named_captures") => RtOp::RegexNamedCaptures,
            (Some(Shape::Regex), "replace") => RtOp::RegexReplace,
            (Some(Shape::Regex), "replace_all") => RtOp::RegexReplaceAll,
            (Some(Shape::Regex), "split") => RtOp::RegexSplit,
            (Some(Shape::Set(_)), "add") => {
                values.push(self.nothing(line));
                RtOp::MapInsert
            }
            (Some(Shape::Set(_)), "remove") => RtOp::MapRemove,
            (Some(Shape::Set(_)), "contains") => RtOp::MapContains,
            (Some(Shape::Set(_)), "len") => RtOp::MapLen,
            (Some(Shape::Set(_)), "is_empty") => RtOp::MapIsEmpty,
            (Some(Shape::Set(_)), "is_full") => RtOp::MapIsFull,
            (Some(Shape::Set(_)), "to_list") => RtOp::MapKeys,
            _ => return None,
        };
        // A method that changes the set gives back the changed set.
        let ty = if op.changes_receiver() { recv } else { ty };
        Some(Ok(self.at(K::Rt(op, values), ty, line)))
    }

    /// `items.sort_by(key)`: the keys from an inlined loop, then a stable
    /// sort of the items by them. `min_by` and `max_by` (`op`) pick an
    /// item by the keys the same way (spec 14.2).
    pub(crate) fn sort_by(
        &mut self,
        op: RtOp,
        list: ir::Expr,
        f: &'a ast::Expr,
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let key = match self.c.expr_types.get(&(f.span.start, f.span.end)) {
            Some(Ty::Fn(_, k, _)) => self.kir_type(&k.clone(), span)?,
            _ => return Err(self.unsupported("a key here", span)),
        };
        let keys_ty = self.list_type(key);
        let tmp = self.new_local(list.ty);
        let src = self.local_expr(tmp, line);
        let keys = self.list_loop("map", src, f, keys_ty, span)?;
        let held = self.new_local(keys_ty);
        let mut sorted = self.at(
            K::Rt(
                op,
                vec![self.local_expr(tmp, line), self.local_expr(held, line)],
            ),
            ty,
            line,
        );
        // A key that fails ends the loop early, with fewer keys than
        // items: the call fails then (spec 5.2), and nothing is sorted.
        if let Some(error) = self.pending_error {
            let unsorted = match op {
                RtOp::ListSortBy => self.local_expr(tmp, line),
                _ => self.at(K::None, ty, line),
            };
            sorted = self.at(
                K::If(
                    Box::new(self.failed(error, line)),
                    wrap_value(unsorted),
                    wrap_value(sorted),
                ),
                ty,
                line,
            );
        }
        Ok(self.at(
            K::Block(Block {
                stmts: vec![Stmt::Let(tmp, list), Stmt::Let(held, keys)],
                value: Some(Box::new(sorted)),
            }),
            ty,
            line,
        ))
    }
}

/// The op of a list method that works by keys from a lambda (spec 14.2).
pub(crate) fn by_keys(method: &str) -> Option<RtOp> {
    match method {
        "sort_by" => Some(RtOp::ListSortBy),
        "min_by" => Some(RtOp::ListMinBy),
        "max_by" => Some(RtOp::ListMaxBy),
        _ => None,
    }
}

pub(crate) fn wrap_value(e: ir::Expr) -> Block {
    Block {
        stmts: Vec::new(),
        value: Some(Box::new(e)),
    }
}
