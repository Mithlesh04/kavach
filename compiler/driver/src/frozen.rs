// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Frozen values (spec 2.20) become plain code, once the checker has
//! typed them (`compiler/checker/src/frozen.rs`). A value frozen without
//! a key is the plain value: `x.freeze()` and `f.unfreeze()` become `x`
//! and `f`. A sealed value is `(value, key)`: `x.freeze(key: k)` becomes
//! `(x, k)`, and where it is read as its value it becomes `f.0`. Only
//! `f.unfreeze(key: k)`, which checks the key, is left for the
//! interpreter and native code (`Compiled::sealed_unfreezes`).

use kavach_ast::visit::{VisitMut, walk_expr};
use kavach_ast::*;
use kavach_checker::{Checked, FrozenCall, Ty};
use std::collections::HashMap;

struct Lower<'c> {
    checked: &'c Checked,
    /// Types for the nodes made here, by span.
    made: HashMap<(u32, u32), Ty>,
}

fn key(span: Span) -> (u32, u32) {
    (span.start, span.end)
}

fn hole(span: Span) -> Expr {
    Expr {
        kind: ExprKind::Error,
        span,
    }
}

impl VisitMut for Lower<'_> {
    fn expr(&mut self, e: &mut Expr) {
        walk_expr(self, e);
        let at = key(e.span);
        let call = match self.checked.frozen_calls.get(&at) {
            // Left for the interpreter and native code: the key is checked.
            Some(FrozenCall::Unfreeze(true)) | None => None,
            Some(call) => Some(*call),
        };
        if let Some(call) = call
            && let ExprKind::Call { callee, args, .. } = &mut e.kind
            && let ExprKind::Field { base, .. } = &mut callee.kind
        {
            let base = std::mem::replace(&mut **base, hole(e.span));
            let sealed = args
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|n| n.name == "key"));
            e.kind = match (call, sealed) {
                (FrozenCall::Freeze(true), Some(k)) => ExprKind::Tuple(vec![base, k.value.clone()]),
                _ => ExprKind::Block(Block {
                    span: e.span,
                    stmts: vec![Stmt {
                        span: base.span,
                        kind: StmtKind::Expr(base),
                    }],
                }),
            };
        }
        // A sealed value read as its value: `f.0`.
        if self.checked.frozen_reads.contains(&at)
            && let Some(Ty::Tuple(ts)) = self.checked.expr_types.get(&at)
            && let Some(value) = ts.first()
        {
            let inner = Span::new(e.span.end as usize, e.span.end as usize);
            self.made.insert(key(inner), Ty::Tuple(ts.clone()));
            self.made.insert(at, value.clone());
            let mut base = std::mem::replace(e, hole(e.span));
            let span = base.span;
            base.span = inner;
            *e = Expr {
                kind: ExprKind::TupleField {
                    base: Box::new(base),
                    index: 0,
                },
                span,
            };
        }
    }
}

/// Turns frozen values into the plain code they stand for, and gives the
/// nodes it makes their types.
pub(crate) fn lower(file: &mut File, checked: &mut Checked) {
    if checked.frozen_calls.is_empty() && checked.frozen_reads.is_empty() {
        return;
    }
    let mut lower = Lower {
        checked,
        made: HashMap::new(),
    };
    kavach_ast::visit::walk_file(&mut lower, file);
    let made = lower.made;
    checked.expr_types.extend(made);
}
