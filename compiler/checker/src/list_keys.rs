// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Indexing a program's own type that has a key of any length, or more
//! than one `Index` (spec 2.16, decision 0136). A list key, `[Part]`,
//! takes any number of places: `t[1, #1]`, `t[0:2, :, 3]`. Each place
//! becomes one `Part`, a number one with `single: true`; the driver
//! builds the list (`compiler/driver/src/indexing.rs`). Notes:
//!
//! - `"index[]"`: a key of numbers only, as a list, to `index`;
//! - `"slice[]"`: a key with a slice, as a list, to `slice`;
//! - `"set[]"`: `x[…] = v` with a list key, to `index_set`, the key
//!   numbers or slices.
//!
//! Another key, such as a mask, chooses its `index` by its type.

use crate::checker::Checker;
use crate::op_impls::Impl;
use crate::ty::{Prim, Ty};
use crate::user_index::at;
use kavach_ast::*;
use kavach_resolver::DefId;
use std::collections::HashMap;

const INT: Ty = Ty::Prim(Prim::Int);

/// Where the list a key becomes is typed: the commas' own span, or for one
/// place an empty span where it starts, which nothing else has.
pub(crate) fn list_span(index: &Expr) -> Span {
    match index.kind {
        ExprKind::Dims(_) => index.span,
        _ => Span::new(index.span.start as usize, index.span.start as usize),
    }
}

impl Checker<'_> {
    /// `[Part]`, the key of any length.
    fn list_key(&self) -> Ty {
        Ty::List(Box::new(self.builtin_ty("Part")))
    }

    /// Which of `impls` takes a list key.
    fn takes_list(&self, impls: &[Impl]) -> Option<usize> {
        let list = self.list_key();
        impls.iter().position(|(k, ..)| self.inf.resolve(k) == list)
    }

    /// Whether `def`'s indexing needs this file: a list key, or more than
    /// one implementation of `Index` or `IndexSet`.
    pub(crate) fn has_many_keys(&self, def: DefId) -> bool {
        let (index_of, set_of) = (self.impls(def, "index"), self.impls(def, "index_set"));
        let slice_of = self.impls(def, "slice");
        index_of.len() > 1
            || set_of.len() > 1
            || [&index_of, &slice_of, &set_of]
                .iter()
                .any(|i| self.takes_list(i).is_some())
    }

    /// Types each place of a key that becomes a list.
    fn list_places(&mut self, index: &Expr) {
        let one = |c: &mut Self, p: &Expr| match &p.kind {
            ExprKind::Part { .. } => {
                c.part_types(p);
                let part = c.builtin_ty("Part");
                c.expr_types.push((p.span, part));
            }
            ExprKind::FromEnd(k) => {
                c.check_expect(k, &INT);
            }
            _ => {
                c.check_expect(p, &INT);
            }
        };
        match &index.kind {
            ExprKind::Dims(parts) => parts.iter().for_each(|p| one(self, p)),
            _ => one(self, index),
        }
        let list = self.list_key();
        self.expr_types.push((list_span(index), list));
    }

    /// Notes the method chosen for a use, when it isn't the plain name.
    fn choose(&mut self, key: (u32, u32), def: DefId, method: &str, takes: &Ty) {
        let name = self.variant_for(def, method, takes);
        if name == method {
            self.chosen.remove(&key);
        } else {
            self.chosen.insert(key, name);
        }
    }

    /// `base[index]` on a type with many keys: its type, or `None` to
    /// leave it to the plain rules of `user_index.rs`.
    pub(crate) fn many_index(
        &mut self,
        def: DefId,
        subst: &HashMap<DefId, Ty>,
        base: &Expr,
        index: &Expr,
    ) -> Option<Ty> {
        let key = at(base, index);
        let list = self.list_key();
        let any_slice = match &index.kind {
            ExprKind::Part { .. } => true,
            ExprKind::Dims(parts) => parts
                .iter()
                .any(|p| matches!(p.kind, ExprKind::Part { .. })),
            _ => false,
        };
        if any_slice {
            let slice_of = self.impls(def, "slice");
            let i = self.takes_list(&slice_of)?;
            self.list_places(index);
            self.user_index.insert(key, "slice[]");
            self.choose(key, def, "slice", &list);
            return Some(self.inf.subst(&slice_of[i].1, subst));
        }
        let index_of = self.impls(def, "index");
        if index_of.is_empty() {
            return None;
        }
        let listed = self.takes_list(&index_of);
        if matches!(index.kind, ExprKind::Dims(_) | ExprKind::FromEnd(_)) {
            let i = listed?;
            self.list_places(index);
            self.user_index.insert(key, "index[]");
            self.choose(key, def, "index", &list);
            return Some(self.inf.subst(&index_of[i].1, subst));
        }
        // One key of its own: the implementation that takes its type, or
        // a number as a list of one place.
        let got = self.value(index, None);
        let others: Vec<Impl> = (index_of.iter())
            .filter(|(k, ..)| self.inf.resolve(k) != list)
            .cloned()
            .collect();
        if let Some(j) = self.choose_impl(&others, subst, &got) {
            self.user_index.insert(key, "index");
            self.choose(key, def, "index", &others[j].0.clone());
            return Some(self.inf.subst(&others[j].1, subst));
        }
        if let Some(i) = listed
            && self.inf.unify(&got, &INT)
        {
            let list_ty = self.list_key();
            self.expr_types.push((list_span(index), list_ty));
            self.user_index.insert(key, "index[]");
            self.choose(key, def, "index", &list);
            return Some(self.inf.subst(&index_of[i].1, subst));
        }
        let (ty, shown) = (self.res.defs[def].name.clone(), self.show(&got));
        let mut takes = self.shown_impls(&others, subst);
        if listed.is_some() {
            takes = if takes.is_empty() {
                "numbers".into()
            } else {
                format!("{takes}, or numbers")
            };
        }
        self.error(
            index.span,
            format!(
                "this key is {}, but `{ty}` is indexed with {takes} (spec 2.16)",
                crate::op_impls::a_shown(&shown)
            ),
        );
        Some(Ty::Error)
    }

    /// `x[k] = v` on a type with many keys: checks `v` and notes the
    /// call. `None` to leave it to the plain rules.
    pub(crate) fn many_index_set(
        &mut self,
        def: DefId,
        subst: &HashMap<DefId, Ty>,
        target: &Expr,
        value: &Expr,
    ) -> Option<()> {
        let ExprKind::Index { base, index } = &target.kind else {
            return None;
        };
        let key = at(base, index);
        let note = *self.user_index.get(&key)?;
        let set_of = self.impls(def, "index_set");
        let key_ty = match note {
            "index[]" | "slice[]" => self.list_key(),
            "index" => self.type_at(index.span)?,
            _ => return None,
        };
        let i = self.choose_set(def, &set_of, subst, &key_ty, value)?;
        self.check_read_only_target(target, "assigned");
        self.user_index
            .insert(key, if note == "index" { "set" } else { "set[]" });
        self.choose_set_variant(key, def, &set_of[i]);
        Some(())
    }

    /// Of `impls` of `index_set`, the one for a key of type `key_ty` and
    /// for `value`: the key's type chooses, then the value's (spec 2.16).
    /// Checks `value`. `None` when no implementation takes the key.
    pub(crate) fn choose_set(
        &mut self,
        def: DefId,
        impls: &[Impl],
        subst: &HashMap<DefId, Ty>,
        key_ty: &Ty,
        value: &Expr,
    ) -> Option<usize> {
        let mut fits = Vec::new();
        for (i, (k, ..)) in impls.iter().enumerate() {
            let want = self.inf.subst(k, subst);
            let before = self.inf.clone();
            if self.inf.unify(key_ty, &want) {
                fits.push(i);
            }
            self.inf = before;
        }
        let &first = fits.first()?;
        if let [only] = fits.as_slice() {
            let k = self.inf.subst(&impls[*only].0, subst);
            self.inf.unify(key_ty, &k);
            let want = self.inf.subst(&impls[*only].1, subst);
            self.check_expect(value, &want);
            return Some(*only);
        }
        let got = self.value(value, None);
        for &i in &fits {
            let want = self.inf.subst(&impls[i].1, subst);
            let before = self.inf.clone();
            if self.inf.unify(&got, &want) {
                let k = self.inf.subst(&impls[i].0, subst);
                self.inf.unify(key_ty, &k);
                return Some(i);
            }
            self.inf = before;
        }
        if !matches!(self.inf.resolve(&got), Ty::Error) {
            let chosen: Vec<Impl> = (fits.iter())
                .map(|&i| (impls[i].1.clone(), impls[i].0.clone(), impls[i].2))
                .collect();
            let (ty, shown) = (self.res.defs[def].name.clone(), self.show(&got));
            let takes = self.shown_impls(&chosen, subst);
            self.error(
                value.span,
                format!(
                    "this value is {}, but `{ty}` is set with {takes} for this key (spec 2.16)",
                    crate::op_impls::a_shown(&shown)
                ),
            );
        }
        Some(first)
    }

    /// Notes the `index_set` chosen for a use, when it isn't the plain
    /// name: the one whose key and value are `chosen`'s.
    pub(crate) fn choose_set_variant(&mut self, key: (u32, u32), def: DefId, chosen: &Impl) {
        let name = self.set_variant_for(def, &chosen.0, &chosen.1);
        if name == "index_set" {
            self.chosen.remove(&key);
        } else {
            self.chosen.insert(key, name);
        }
    }
}
