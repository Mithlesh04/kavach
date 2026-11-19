// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Text views and builders (spec 2.18). Both are text in native code for
//! now: a view is a copy of its part, and a builder a `String` that
//! `push` grows in place while it has one holder (`kv_concat`, spec 6.1).
//! `insert`, `replace` and `remove` join the parts around the change.

use crate::ir::{self, BinOp, Block, ExprKind as K, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;
use kavach_checker::{Prim, Ty};

impl<'a> Lowerer<'a> {
    /// `s.view(…)`, `v.find(t)`, `v.to_string()` and a builder's methods,
    /// on `values` (the receiver first); `None` for any other call.
    pub(crate) fn view_method(
        &mut self,
        base: &ast::Expr,
        name: &str,
        values: &mut Vec<ir::Expr>,
        ty: Type,
        line: u32,
    ) -> Lowered<Option<ir::Expr>> {
        let kind = match self.c.expr_types.get(&(base.span.start, base.span.end)) {
            Some(Ty::Prim(Prim::String)) => "String",
            Some(Ty::Builtin("StrView", _)) => "StrView",
            Some(Ty::Builtin("TextBuilder", _)) => "TextBuilder",
            _ => return Ok(None),
        };
        let text = self.str_type();
        let mut v = std::mem::take(values);
        let made = match (kind, name, v.len()) {
            (_, "view", 1) | ("StrView" | "TextBuilder", "to_string", 1) => v.pop(),
            ("StrView", "find", 2) => Some(ir::Expr {
                kind: K::Rt(RtOp::StrIndexOf, v),
                ty,
                line,
            }),
            ("TextBuilder", "push", 2) => {
                let part = v.pop().expect("a part");
                let b = v.pop().expect("the builder");
                Some(self.joined(b, part, line))
            }
            ("TextBuilder", "push_char", 2) => {
                let c = v.pop().expect("a character");
                let part = self.at(K::Format(vec![c]), text, line);
                let b = v.pop().expect("the builder");
                Some(self.joined(b, part, line))
            }
            ("TextBuilder", "insert" | "replace" | "remove" | "clear", _) => {
                Some(self.spliced(name, v, line))
            }
            _ => {
                *values = v;
                None
            }
        };
        Ok(made)
    }

    /// `a + b` on text: `a` grows in place when nothing else holds it.
    fn joined(&mut self, a: ir::Expr, b: ir::Expr, line: u32) -> ir::Expr {
        let text = self.str_type();
        self.at(K::Binary(BinOp::Add, Box::new(a), Box::new(b)), text, line)
    }

    /// `b.insert(at, t)`, `b.replace(start, end, t)`, `b.remove(start,
    /// end)`, `b.clear()`: the text before `start`, then `t`, then the
    /// text from `end`. A `start` after `end` panics (a contract).
    fn spliced(&mut self, name: &str, v: Vec<ir::Expr>, line: u32) -> ir::Expr {
        let text = self.str_type();
        let int = |n: i128| ir::Expr {
            kind: K::Int(n),
            ty: Type::I64,
            line,
        };
        let empty = || ir::Expr {
            kind: K::Str(String::new()),
            ty: text,
            line,
        };
        let mut v = v.into_iter();
        let b = v.next().expect("the builder");
        let (start, end, part) = match name {
            "insert" => {
                let at = v.next().expect("a position");
                (at, None, v.next().expect("a part"))
            }
            "replace" => {
                let start = v.next().expect("a start");
                let end = v.next().expect("an end");
                (start, Some(end), v.next().expect("a part"))
            }
            "remove" => {
                let start = v.next().expect("a start");
                (start, Some(v.next().expect("an end")), empty())
            }
            _ => (int(0), Some(int(i128::from(i64::MAX))), empty()),
        };
        let mut stmts = Vec::new();
        let mut hold = |e: ir::Expr, stmts: &mut Vec<Stmt>| {
            let l = self.new_local(e.ty);
            stmts.push(Stmt::Let(l, e));
            l
        };
        let b = hold(b, &mut stmts);
        let start = hold(start, &mut stmts);
        let end = match end {
            Some(e) => hold(e, &mut stmts),
            None => start,
        };
        let read = |s: &Self, l| s.local_expr(l, line);
        let cond = ir::Expr {
            kind: K::Binary(
                BinOp::Le,
                Box::new(read(self, start)),
                Box::new(read(self, end)),
            ),
            ty: Type::Bool,
            line,
        };
        stmts.push(Stmt::Check {
            cond,
            message: format!("contract failed: `{name}` starts after its end"),
            line,
        });
        let slice = |s: &Self, from, to| ir::Expr {
            kind: K::Rt(RtOp::StrSlice, vec![read(s, b), from, to]),
            ty: text,
            line,
        };
        let before = slice(self, int(0), read(self, start));
        let after = slice(self, read(self, end), int(i128::from(i64::MAX)));
        let with_part = self.joined(before, part, line);
        let value = self.joined(with_part, after, line);
        self.at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(value)),
            }),
            text,
            line,
        )
    }
}
