// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Indexing and slicing on a program's own types become the method calls
//! they stand for (spec 2.16), once the checker has typed them
//! (`compiler/checker/src/user_index.rs`): `x[k]` is `x.index(k)`,
//! `x[#k]` is `x.index(x.len() - k)`, `x[a:b:step]` is
//! `x.slice(Part(start: a, end: b, step: step))`, and `x[k] = v` is
//! `x = x.index_set(k, v)`. So the interpreter and native code run them
//! as any method call. The nodes made here get their types too.

use kavach_ast::visit::{VisitMut, walk_expr, walk_stmt};
use kavach_ast::*;
use kavach_checker::{Checked, Prim, Ty};
use kavach_resolver::{DefId, Resolution};
use std::collections::HashMap;

struct Calls<'c> {
    checked: &'c Checked,
    /// `Part` (`std.collections`).
    part: Option<DefId>,
    /// Types for the nodes made here, by span.
    made: HashMap<(u32, u32), Ty>,
    /// The name `Part` written here, by where it starts.
    uses: Vec<(u32, DefId)>,
}

fn key(span: Span) -> (u32, u32) {
    (span.start, span.end)
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

/// An argument given by name.
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
    /// The key a method is given: a `Part` for a slice, a tuple for
    /// parts separated by commas.
    fn key_of(&mut self, index: Expr) -> Expr {
        match index.kind {
            ExprKind::Part { .. } => self.part(index),
            ExprKind::Dims(parts) => Expr {
                kind: ExprKind::Tuple(parts.into_iter().map(|p| self.key_of(p)).collect()),
                span: index.span,
            },
            kind => Expr {
                kind,
                span: index.span,
            },
        }
    }

    /// `Part(start: a, end: b, step: s)`, with `start_from_end: true` for
    /// an end written `#k`. The call has the slice's span, whose type the
    /// checker gave; the name starts where the slice ends, at `,` or `]`,
    /// where no other name starts.
    fn part(&mut self, p: Expr) -> Expr {
        let span = p.span;
        let ExprKind::Part { start, end, step } = p.kind else {
            return p;
        };
        let mut args = Vec::new();
        for (name, x) in [("start", start), ("end", end)] {
            let Some(x) = x else { continue };
            match x.kind {
                ExprKind::FromEnd(k) => {
                    args.push(named(name, *k));
                    let yes = Expr {
                        kind: ExprKind::Bool(true),
                        span: x.span,
                    };
                    self.made.insert(key(x.span), Ty::Prim(Prim::Bool));
                    args.push(named(&format!("{name}_from_end"), yes));
                }
                kind => args.push(named(name, Expr { kind, span: x.span })),
            }
        }
        if let Some(s) = step {
            args.push(named("step", *s));
        }
        let at = span.end as usize;
        if let Some(def) = self.part {
            self.uses.push((span.end, def));
        }
        let callee = Expr {
            kind: ExprKind::Name(Ident {
                name: "Part".to_string(),
                span: Span::new(at, at),
            }),
            span: Span::new(at, at),
        };
        Expr {
            kind: ExprKind::Call {
                callee: Box::new(callee),
                generics: Vec::new(),
                args,
            },
            span,
        }
    }
}

impl VisitMut for Calls<'_> {
    fn expr(&mut self, e: &mut Expr) {
        walk_expr(self, e);
        let ExprKind::Index { base, index } = &e.kind else {
            return;
        };
        let note = match self
            .checked
            .user_index
            .get(&(base.span.start, index.span.end))
        {
            Some(&n @ ("index" | "index#" | "slice")) => n,
            _ => return,
        };
        let span = e.span;
        let ExprKind::Index { base, index } = std::mem::replace(&mut e.kind, ExprKind::None) else {
            return;
        };
        *e = match (note, index.kind) {
            // `x[#k]`: `x.index(x.len() - k)`; `x - k` has the span of
            // `#k`, and `x.len()` that span the other way round.
            ("index#", ExprKind::FromEnd(k)) => {
                let back = Span::new(index.span.end as usize, index.span.start as usize);
                let int = Ty::Prim(Prim::Int);
                self.made.insert(key(back), int.clone());
                self.made.insert(key(index.span), int);
                let len = method_call((*base).clone(), "len", Vec::new(), back);
                let at = Expr {
                    kind: ExprKind::Binary(BinaryOp::Sub, Box::new(len), k),
                    span: index.span,
                };
                method_call(*base, "index", vec![at], span)
            }
            (_, kind) => {
                let index = Expr {
                    kind,
                    span: index.span,
                };
                let method = if note == "slice" { "slice" } else { "index" };
                let k = self.key_of(index);
                method_call(*base, method, vec![k], span)
            }
        };
    }

    fn stmt(&mut self, s: &mut Stmt) {
        walk_stmt(self, s);
        let StmtKind::Assign { target, op, value } = &mut s.kind else {
            return;
        };
        let ExprKind::Index { base, index } = &target.kind else {
            return;
        };
        if self
            .checked
            .user_index
            .get(&(base.span.start, index.span.end))
            != Some(&"set")
        {
            return;
        }
        // `x[k] = v` is `x = x.index_set(k, v)`.
        let span = Span::new(target.span.start as usize, value.span.end as usize);
        if let Some(t) = self
            .checked
            .expr_types
            .get(&(base.span.start, base.span.end))
        {
            self.made.insert(key(span), t.clone());
        }
        let old = std::mem::replace(
            target,
            Expr {
                kind: ExprKind::None,
                span,
            },
        );
        let ExprKind::Index { base, index } = old.kind else {
            return;
        };
        let v = std::mem::replace(
            value,
            Expr {
                kind: ExprKind::None,
                span,
            },
        );
        let k = self.key_of(*index);
        *value = method_call((*base).clone(), "index_set", vec![k, v], span);
        *target = *base;
        *op = AssignOp::Set;
    }
}

/// Turns the indexing the checker noted on a program's own types into
/// method calls, and gives the nodes it makes their types.
pub(crate) fn to_method_calls(file: &mut File, checked: &mut Checked, resolution: &mut Resolution) {
    if checked.user_index.is_empty() {
        return;
    }
    let mut calls = Calls {
        checked,
        part: resolution.std_type("Part"),
        made: HashMap::new(),
        uses: Vec::new(),
    };
    kavach_ast::visit::walk_file(&mut calls, file);
    let (made, uses) = (calls.made, calls.uses);
    checked.expr_types.extend(made);
    resolution.uses.extend(uses);
}
