// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A table's column, `t["amount"]`, becomes the call it stands for, once
//! the checker has typed it (spec 14.34,
//! `compiler/checker/src/std_table.rs`): `t.column(.amount)`. So the
//! interpreter and native code run it as any method call. The nodes made
//! here get their types too; each has a span no other node has: the
//! name's own for the lambda, and that span the other way round, or one
//! past it, for the field and the lambda's value.

use kavach_ast::visit::{VisitMut, walk_expr};
use kavach_ast::*;
use kavach_checker::{Checked, Ty};
use std::collections::HashMap;

struct Calls<'c> {
    checked: &'c Checked,
    made: HashMap<(u32, u32), Ty>,
}

fn key(span: Span) -> (u32, u32) {
    (span.start, span.end)
}

impl VisitMut for Calls<'_> {
    fn expr(&mut self, e: &mut Expr) {
        walk_expr(self, e);
        let ExprKind::Index { base, index } = &e.kind else {
            return;
        };
        let at = (base.span.start, index.span.end);
        let Some((name, row, field)) = self.checked.table_columns.get(&at).cloned() else {
            return;
        };
        let span = e.span;
        let ExprKind::Index { base, index } = std::mem::replace(&mut e.kind, ExprKind::None) else {
            return;
        };
        let (s, end) = (index.span.start as usize, index.span.end as usize);
        let (it, read) = (Span::new(end + 1, s), Span::new(end, s));
        self.made.insert(key(it), row.clone());
        self.made.insert(key(read), field.clone());
        self.made
            .insert(key(index.span), Ty::Fn(vec![row], Box::new(field), None));
        let value = Expr {
            kind: ExprKind::Field {
                base: Box::new(Expr {
                    kind: ExprKind::It,
                    span: it,
                }),
                name: Ident { name, span: read },
                optional: false,
            },
            span: read,
        };
        let lambda = Expr {
            kind: ExprKind::DotLambda(Box::new(value)),
            span: index.span,
        };
        let at = base.span.end as usize;
        let callee = Expr {
            kind: ExprKind::Field {
                base,
                name: Ident {
                    name: "column".to_string(),
                    span: Span::new(at, at),
                },
                optional: false,
            },
            span,
        };
        *e = Expr {
            kind: ExprKind::Call {
                callee: Box::new(callee),
                generics: Vec::new(),
                args: vec![Arg {
                    name: None,
                    value: lambda,
                }],
            },
            span,
        };
    }
}

/// Turns the columns the checker noted into method calls, and gives the
/// nodes it makes their types.
pub(crate) fn to_calls(file: &mut File, checked: &mut Checked) {
    if checked.table_columns.is_empty() {
        return;
    }
    let mut calls = Calls {
        checked,
        made: HashMap::new(),
    };
    kavach_ast::visit::walk_file(&mut calls, file);
    let made = calls.made;
    checked.expr_types.extend(made);
}
