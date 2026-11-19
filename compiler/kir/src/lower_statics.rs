// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Calls through a built-in type's name: `Set.of(…)`, `List.capacity(n)`,
//! `Map.capacity(n)`, `Set.capacity(n)` (spec 2.2), `Ptr.from_int(n)`,
//! `Bytes.of<T>(…)` (spec 2.13) and `Float.from_bits(n)`.

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;
use kavach_resolver::{DefId, DefKind};

impl<'a> Lowerer<'a> {
    /// The call `owner.name(args)` when `owner` is a built-in type with
    /// such a static function, otherwise `None`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn builtin_static(
        &mut self,
        def: DefId,
        owner: &str,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Option<Lowered<ir::Expr>> {
        let builtin = self.c.resolution.defs[def].kind == DefKind::Builtin;
        Some(match (owner, name) {
            ("Set", "of") if builtin => self.set_of(args, ty, line),
            ("List", "filled") if builtin => self.list_filled(args, ty, line, span),
            ("Dict" | "Set" | "List" | "Ring", "capacity") if builtin => {
                self.capacity(owner, args, ty, line, span)
            }
            ("Ptr", "from_int") => self
                .first_arg(args, "`Ptr.from_int()`", span)
                .map(|n| ir::Expr { ty, ..n }),
            ("Bytes", "of") => self.bytes_of(args, ty, line),
            ("Bytes", "of_list") => self.bytes_of_list(args, ty, line),
            // `TextBuilder.capacity(n)` (spec 2.18): empty text, which
            // grows as it is pushed onto.
            ("TextBuilder", "capacity") if builtin => Ok(ir::Expr {
                kind: K::Str(String::new()),
                ty,
                line,
            }),
            ("Float", "from_bits") => self.expr(&args[0].value).map(|bits| ir::Expr {
                kind: K::Rt(RtOp::FloatFromBits, vec![bits]),
                ty,
                line,
            }),
            _ => return None,
        })
    }

    /// `List.capacity(n, first)` and the others: the first entries are 0
    /// when not given.
    fn capacity(
        &mut self,
        owner: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let op = match owner {
            "List" => RtOp::ListCapacity,
            "Dict" => RtOp::MapCapacity,
            "Ring" => RtOp::RingCapacity,
            _ => RtOp::SetCapacity,
        };
        let n = self.first_arg(args, "`capacity()`", span)?;
        let first = match args.get(1) {
            Some(a) => {
                let first = self.expr(&a.value)?;
                // The first entries of a list or map are of its type.
                if matches!(op, RtOp::SetCapacity | RtOp::RingCapacity) {
                    first
                } else {
                    self.coerce(first, ty)?
                }
            }
            None => self.at(K::Int(0), Type::I64, line),
        };
        Ok(ir::Expr {
            kind: K::Rt(op, vec![n, first]),
            ty,
            line,
        })
    }

    /// The first argument, lowered.
    fn first_arg(
        &mut self,
        args: &'a [ast::Arg],
        what: &str,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let n = args.first().ok_or_else(|| self.unsupported(what, span))?;
        self.expr(&n.value)
    }
}
