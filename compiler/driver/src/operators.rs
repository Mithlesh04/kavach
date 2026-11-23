// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Operators on a program's own types become the method calls they stand
//! for (spec 2.16), once the checker has typed them: `a + b` is
//! `a.add(b)`, `-a` is `a.neg()`, `~a` is `a.bit_not()`, `a < b` is `a.compare(b) < 0`, and
//! `c += a` is `c = c.add(a)`. So the interpreter and native code run
//! them as any method call. The nodes made here get their types too.

use kavach_ast::visit::{VisitMut, walk_expr, walk_stmt};
use kavach_ast::*;
use kavach_checker::{Checked, Prim, Ty};
use std::collections::HashMap;

struct Calls<'c> {
    checked: &'c Checked,
    /// Types for the nodes made here, by span.
    made: HashMap<(u32, u32), Ty>,
}

/// `base.method(args)`, at `span`.
fn method_call(base: Expr, method: &str, args: Vec<Expr>, span: Span) -> Expr {
    let at = base.span.end as usize;
    let callee = Expr {
        kind: ExprKind::Field {
            base: Box::new(base),
            name: Ident {
                name: method.to_string(),
                span: Span::new(at, at),
            },
            optional: false,
        },
        span,
    };
    Expr {
        kind: ExprKind::Call {
            callee: Box::new(callee),
            generics: Vec::new(),
            args: args
                .into_iter()
                .map(|value| Arg { name: None, value })
                .collect(),
        },
        span,
    }
}

fn key(span: Span) -> (u32, u32) {
    (span.start, span.end)
}

impl VisitMut for Calls<'_> {
    fn expr(&mut self, e: &mut Expr) {
        walk_expr(self, e);
        // A method written more than once: the one the checker chose.
        if let ExprKind::Call { callee, .. } = &mut e.kind
            && let ExprKind::Field { name, .. } = &mut callee.kind
            && let Some(chosen) = self.checked.chosen.get(&key(name.span))
        {
            name.name.clone_from(chosen);
        }
        let span = e.span;
        let kind = std::mem::replace(&mut e.kind, ExprKind::None);
        e.kind = match kind {
            ExprKind::Binary(op, l, r) => {
                match self.checked.user_ops.get(&(l.span.start, r.span.end)) {
                    Some(&"compare") => {
                        // `a.compare(b) < 0`: the call spans a little less
                        // than the comparison, so each has its own type.
                        let end = r.span.end;
                        let inner = Span::new(l.span.start as usize, end as usize - 1);
                        let zero = Expr {
                            kind: ExprKind::Int(0),
                            span: Span::new(end as usize, end as usize),
                        };
                        let int = Ty::Prim(Prim::Int);
                        self.made.insert(key(inner), int.clone());
                        self.made.insert(key(zero.span), int);
                        let call = method_call(*l, "compare", vec![*r], inner);
                        ExprKind::Binary(op, Box::new(call), Box::new(zero))
                    }
                    Some(method) => {
                        let at = (l.span.start, r.span.end);
                        let name = self.checked.chosen.get(&at).map_or(*method, |n| n.as_str());
                        method_call(*l, name, vec![*r], span).kind
                    }
                    None => ExprKind::Binary(op, l, r),
                }
            }
            ExprKind::Unary(op @ (UnaryOp::Neg | UnaryOp::BitNot), x)
                if self.checked.user_negs.contains(&key(x.span)) =>
            {
                let method = if op == UnaryOp::Neg { "neg" } else { "bit_not" };
                method_call(*x, method, Vec::new(), span).kind
            }
            other => other,
        };
    }

    fn stmt(&mut self, s: &mut Stmt) {
        walk_stmt(self, s);
        let StmtKind::Assign { target, op, value } = &mut s.kind else {
            return;
        };
        let at = (target.span.start, value.span.end);
        let Some(method) = self.checked.user_assigns.get(&at) else {
            return;
        };
        // `c += a` is `c = c.add(a)`.
        let span = Span::new(at.0 as usize, at.1 as usize);
        if let Some(t) = self.checked.expr_types.get(&key(target.span)) {
            self.made.insert(at, t.clone());
        }
        let v = std::mem::replace(
            value,
            Expr {
                kind: ExprKind::None,
                span,
            },
        );
        let method = self.checked.chosen.get(&at).map_or(*method, |n| n.as_str());
        *value = method_call(target.clone(), method, vec![v], span);
        *op = AssignOp::Set;
    }
}

/// Turns the operators the checker noted into method calls, and gives the
/// nodes it makes their types.
pub(crate) fn to_method_calls(file: &mut File, checked: &mut Checked) {
    if checked.user_ops.is_empty()
        && checked.user_negs.is_empty()
        && checked.user_assigns.is_empty()
        && checked.chosen.is_empty()
    {
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
