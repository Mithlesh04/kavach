// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The GPU numbers a program writes (spec 15.6): `gpu(n) fn` and `on
//! gpu(n) x` with a number, which the program's start checks against the
//! machine's GPUs, in the interpreter and native code alike.

use crate::visit::{VisitMut, walk_expr, walk_file};
use crate::{Expr, ExprKind, File, ItemKind, Span, ValuePlace};

/// Where a GPU number is written.
#[derive(Clone, Debug, PartialEq)]
pub enum GpuNumberAt {
    /// `gpu(n) fn name`.
    Fn(String),
    /// `on gpu(n)`, its number at this span.
    On(Span),
}

/// The highest GPU number `file` writes, and where; `None` when it
/// writes none.
pub fn highest_gpu(file: &File) -> Option<(u64, GpuNumberAt)> {
    struct Find(Vec<(u64, GpuNumberAt)>);
    impl VisitMut for Find {
        fn expr(&mut self, e: &mut Expr) {
            if let ExprKind::On {
                place: ValuePlace::Gpu(n),
                ..
            } = &e.kind
                && let ExprKind::Int(k) = n.kind
            {
                self.0.push((k, GpuNumberAt::On(n.span)));
            }
            walk_expr(self, e);
        }
    }
    let mut find = Find(Vec::new());
    for item in &file.items {
        if let ItemKind::Fn(f) = &item.kind
            && let Some(n) = f.gpu_device
        {
            find.0.push((n, GpuNumberAt::Fn(f.name.name.clone())));
        }
    }
    walk_file(&mut find, &mut file.clone());
    // The first of the highest, in the order written.
    let highest = find.0.iter().map(|(n, _)| *n).max()?;
    find.0.into_iter().find(|(n, _)| *n == highest)
}
