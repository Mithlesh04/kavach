// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A `for` variable over a list of text kept as a view of each item, its
//! address, instead of a text object made for every item (`stmt.rs`).
//! The body may ask the item's length or whether it is empty, and push
//! it onto another list of text, which copies its two words: so
//! `words.filter(w => not w.is_empty())`, a loop of exactly that, makes
//! no object for any word (Sakshi's S8: about 60 ns a word).

use crate::func::FnEmit;
use kavach_kir::{Block, Expr, ExprKind, Local, RtOp, walk};

impl FnEmit<'_> {
    /// Whether `for` variable `var` over a list of text can be a view: it
    /// never changes, and each mention of it is the text whose length is
    /// asked, or whether it is empty, or what a local list of text is
    /// given by `push`.
    pub(crate) fn text_view_ok(&self, body: &Block, var: Local) -> bool {
        let is_var =
            |e: &Expr| matches!(e.kind, ExprKind::Local(l) | ExprKind::Move(l) if l == var);
        let (mut all, mut fine, mut set) = (0, 0, false);
        walk::any_in_block(body, &mut |e| {
            match &e.kind {
                ExprKind::Local(l) | ExprKind::Move(l) if *l == var => all += 1,
                ExprKind::Assign(l, _) if *l == var => set = true,
                ExprKind::Rt(RtOp::StrLen | RtOp::StrIsEmpty, args) if is_var(&args[0]) => {
                    fine += 1
                }
                // `acc = acc.push(var)`, as `expr.rs` stores a push.
                ExprKind::Assign(l, v) => {
                    if let ExprKind::Rt(RtOp::ListPush, args) = &v.kind
                        && args.len() == 2
                        && matches!(args[0].kind, ExprKind::Local(x) | ExprKind::Move(x) if x == *l)
                        && is_var(&args[1])
                        && self.text_list(v.ty)
                    {
                        fine += 1;
                    }
                }
                _ => {}
            }
            false
        });
        !set && all == fine
    }

    /// Whether `e` is a `for` variable that is a view (`text_view_ok`).
    pub(crate) fn is_text_view(&self, e: &Expr) -> bool {
        matches!(e.kind, ExprKind::Local(l) | ExprKind::Move(l) if self.text_views[l as usize])
    }

    /// `text.is_empty()`: from the item's tag for a text view.
    pub(crate) fn view_is_empty(&mut self, arg: &Expr) -> Option<String> {
        let addr = self.text_view(arg)?;
        let n = self.text_len(&addr);
        Some(self.val(format!("icmp eq i64 {n}, 0")))
    }
}
