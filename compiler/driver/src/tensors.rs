// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What the checker found of `std.tensor`'s own rules becomes plain
//! code (spec 14.33, `compiler/checker/src/std_tensor.rs`):
//! `Tensor<U>(t)` is `t.converted<U>()`, and a number `x` given where a
//! tensor is asked for is `Tensor(shape: [], items: [x])`. The nodes made
//! here get their types too; each has a span no other node has: the
//! number's own, the other way round, or one past it.

use kavach_ast::visit::{VisitMut, walk_expr};
use kavach_ast::*;
use kavach_checker::{Checked, Prim, Ty};
use kavach_resolver::{DefId, Resolution};
use std::collections::HashMap;

struct Calls<'c> {
    checked: &'c Checked,
    tensor: Option<DefId>,
    made: HashMap<(u32, u32), Ty>,
    uses: Vec<(u32, DefId)>,
}

fn key(span: Span) -> (u32, u32) {
    (span.start, span.end)
}

fn named(name: &str, value: Expr) -> Arg {
    let at = value.span.start as usize;
    Arg {
        name: Some(Ident {
            name: name.to_string(),
            span: Span::new(at, at),
        }),
        value,
    }
}

impl Calls<'_> {
    /// `Tensor(shape: [], items: [x])`, for the number `x`.
    fn wrapped(&mut self, x: Expr, ty: Ty) -> Expr {
        let (s, e) = (x.span.start as usize, x.span.end as usize);
        let item = match &ty {
            Ty::Adt(_, args) => args.first().cloned().unwrap_or(Ty::Error),
            _ => Ty::Error,
        };
        let (shape_at, items_at, call) =
            (Span::new(e + 1, s), Span::new(e + 2, s), Span::new(e, s));
        self.made
            .insert(key(shape_at), Ty::List(Box::new(Ty::Prim(Prim::Int))));
        self.made.insert(key(items_at), Ty::List(Box::new(item)));
        self.made.insert(key(call), ty);
        if let Some(def) = self.tensor {
            self.uses.push((e as u32, def));
        }
        let list = |items: Vec<Expr>, span: Span| Expr {
            kind: ExprKind::List(items),
            span,
        };
        let callee = Expr {
            kind: ExprKind::Name(Ident {
                name: "Tensor".to_string(),
                span: Span::new(e, e),
            }),
            span: Span::new(e, e),
        };
        let args = vec![
            named("shape", list(Vec::new(), shape_at)),
            named("items", list(vec![x], items_at)),
        ];
        Expr {
            kind: ExprKind::Call {
                callee: Box::new(callee),
                generics: Vec::new(),
                args,
            },
            span: call,
        }
    }
}

impl VisitMut for Calls<'_> {
    fn expr(&mut self, e: &mut Expr) {
        walk_expr(self, e);
        if let Some(ty) = self.checked.as_tensors.get(&key(e.span)) {
            let x = std::mem::replace(
                e,
                Expr {
                    kind: ExprKind::None,
                    span: e.span,
                },
            );
            *e = self.wrapped(x, ty.clone());
            return;
        }
        if !self.checked.tensor_casts.contains(&key(e.span)) {
            return;
        }
        // `Tensor<U>(t)` is `t.converted<U>()`.
        let ExprKind::Call { generics, args, .. } = &mut e.kind else {
            return;
        };
        let (Some(arg), generics) = (args.pop(), std::mem::take(generics)) else {
            return;
        };
        let base = arg.value;
        let at = base.span.end as usize;
        let callee = Expr {
            kind: ExprKind::Field {
                base: Box::new(base),
                name: Ident {
                    name: "converted".to_string(),
                    span: Span::new(at, at),
                },
                optional: false,
            },
            span: e.span,
        };
        e.kind = ExprKind::Call {
            callee: Box::new(callee),
            generics,
            args: Vec::new(),
        };
    }
}

/// Turns tensor conversions and numbers given as tensors into the calls
/// they stand for.
pub(crate) fn to_calls(file: &mut File, checked: &mut Checked, resolution: &mut Resolution) {
    if checked.tensor_casts.is_empty() && checked.as_tensors.is_empty() {
        return;
    }
    let mut calls = Calls {
        checked,
        tensor: resolution.std_type("Tensor"),
        made: HashMap::new(),
        uses: Vec::new(),
    };
    kavach_ast::visit::walk_file(&mut calls, file);
    let (made, uses) = (calls.made, calls.uses);
    checked.expr_types.extend(made);
    resolution.uses.extend(uses);
}
