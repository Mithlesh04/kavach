// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Indexing and slicing for a program's own types (spec 2.16, decision
//! 0134): `Index<Key, Out>` gives `x[k]`, `IndexSet<Key, Value>` gives
//! `x[k] = v`, and `Slice` gives `x[a:b:step]`, the slice as a `Part`
//! (`std.collections`). Each use is noted in `user_index`, by
//! `(base start, index end)`, and the driver turns it into the method
//! call (`compiler/driver/src/indexing.rs`):
//!
//! - `"index"`: `x[k]` is `x.index(k)`; with commas, a tuple key whose
//!   slices are `Part`s;
//! - `"index#"`: `x[#k]` is `x.index(x.len() - k)`;
//! - `"slice"`: `x[a:b:step]` is `x.slice(Part(…))`, and with commas a
//!   tuple of `Part`s;
//! - `"set"`: `x[k] = v` is `x = x.index_set(k, v)`;
//! - `"set?"`: a read of a type with `IndexSet` but no `Index`, an error
//!   unless it is the target of `=`.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;

const INT: Ty = Ty::Prim(Prim::Int);

/// The interfaces, and the method each gives.
const INDEXERS: &[(&str, &str)] = &[
    ("Index", "index"),
    ("IndexSet", "index_set"),
    ("Slice", "slice"),
];

/// Where a use of `base[index]` is noted.
pub(crate) fn at(base: &Expr, index: &Expr) -> (u32, u32) {
    (base.span.start, index.span.end)
}

impl Checker<'_> {
    /// `type Grid: Index<(Int, Int), Float>`: notes the key and the
    /// result. `None` when `i` names none of the three interfaces.
    pub(crate) fn note_indexer(
        &mut self,
        owner: Option<DefId>,
        i: &TypeExpr,
        self_ty: &Ty,
    ) -> Option<bool> {
        let TypeKind::Named { path, args } = &i.kind else {
            return None;
        };
        let name = path.segments.last()?.name.as_str();
        let &(iface, method) = INDEXERS.iter().find(|(n, _)| *n == name)?;
        let Some(def) = owner else {
            let shown = self.show(self_ty);
            self.error(i.span, format!("`{shown}` keeps its own indexing: a built-in type can't implement `{iface}` (spec 2.16)"));
            return Some(true);
        };
        let given: Vec<Ty> = args.iter().map(|a| self.lower(a, Some(self_ty))).collect();
        let pair = match (method, given.as_slice()) {
            ("index" | "index_set", [key, out]) => (key.clone(), out.clone()),
            ("slice", []) => (self.builtin_ty("Part"), self_ty.clone()),
            ("slice", [key]) => (key.clone(), self_ty.clone()),
            _ => {
                let want = match method {
                    "slice" => "`Slice` or `Slice<Key>`".to_string(),
                    _ => format!(
                        "`{iface}<Key, {}>`",
                        if method == "index" { "Out" } else { "Value" }
                    ),
                };
                self.error(i.span, format!("`{iface}` is written {want} (spec 2.16)"));
                return Some(true);
            }
        };
        self.note_impl(def, method, (pair.0, pair.1, i.span));
        Some(true)
    }

    /// Each indexing method is there, with the interface's types, pure
    /// and unable to fail (spec 2.16).
    pub(crate) fn check_indexer_methods(&mut self) {
        let mut wanted: Vec<_> = (self.operators.keys())
            .filter(|(_, m)| INDEXERS.iter().any(|(_, x)| x == m))
            .copied()
            .collect();
        wanted.sort();
        for &(def, m) in &wanted {
            if self.impls(def, m).len() > 1 {
                let iface = INDEXERS
                    .iter()
                    .find(|(_, x)| *x == m)
                    .map_or("", |(i, _)| i);
                self.check_impls_differ(def, m, iface, "key");
            }
        }
        let wanted: Vec<_> = (wanted.into_iter())
            .flat_map(|(def, m)| self.impls(def, m).into_iter().map(move |i| (def, m, i)))
            .collect();
        for (def, method, (key, out, span)) in wanted {
            let ty = self.res.defs[def].name.clone();
            let (k, o) = (self.show(&key), self.show(&out));
            let want = match method {
                "index" => format!("fn index(self, key: {k}) -> {o}"),
                "index_set" => format!("fn index_set(self, key: {k}, value: {o}) -> {ty}"),
                _ => format!("fn slice(self, part: {k}) -> {ty}"),
            };
            let iface = INDEXERS
                .iter()
                .find(|(_, m)| *m == method)
                .map_or("", |(i, _)| i);
            let name = self.variant_for(def, method, &key);
            let Some(sig) = self.methods.get(&def).and_then(|m| m.get(&name)).cloned() else {
                self.error(
                    span,
                    format!("`{ty}` says it implements `{iface}`, so it needs the method `{want}`"),
                );
                continue;
            };
            let params: Vec<Ty> = sig.params.iter().map(|p| self.inf.resolve(&p.ty)).collect();
            let ret = self.inf.resolve(&sig.ret);
            let key = self.inf.resolve(&key);
            let out = self.inf.resolve(&out);
            let fits = sig.has_self
                && match method {
                    "index_set" => params.len() == 2 && params[0] == key && params[1] == out,
                    _ => params.len() == 1 && params[0] == key,
                }
                && match method {
                    "index" => ret == out,
                    _ => matches!(&ret, Ty::Adt(d, _) if *d == def),
                };
            if !fits {
                self.error(
                    span,
                    format!("`{ty}.{method}` must be `{want}` for `{iface}` (spec 2.16)"),
                );
                continue;
            }
            let uses = (sig.def.and_then(|d| self.fn_uses.get(&d))).is_some_and(|u| !u.is_empty());
            if sig.errors.is_some() || !sig.uses.is_empty() || uses {
                self.error(span, format!("`{ty}.{method}` gives `[…]`, so it can't use capabilities (`uses`) or fail (`!`), as an operator can't (spec 2.16)"));
            }
        }
        // A read of a type that can only be set (`"set?"`, never a target).
        let mut reads: Vec<(u32, u32)> = (self.user_index.iter())
            .filter(|(_, u)| **u == "set?")
            .map(|(k, _)| *k)
            .collect();
        reads.sort();
        for (start, end) in reads {
            let span = Span::new(start as usize, end as usize);
            self.error(span, "this type has `IndexSet` but no `Index`: `x[k]` can be set but not read (spec 2.16)");
        }
    }

    /// `base[index]` on a program's own type, `t`: its type, or `None`
    /// when `t` has none of the interfaces.
    pub(crate) fn own_index(&mut self, base: &Expr, t: &Ty, index: &Expr) -> Option<Ty> {
        let Ty::Adt(def, targs) = t else {
            return None;
        };
        let def = *def;
        let subst = self.subst_of(def, targs);
        let index_of = self.operators.get(&(def, "index")).cloned();
        let slice_of = self.operators.get(&(def, "slice")).cloned();
        let set_of = self.operators.get(&(def, "index_set")).cloned();
        if index_of.is_none() && slice_of.is_none() && set_of.is_none() {
            return None;
        }
        // A key of any length, or more than one `Index` (`list_keys.rs`).
        if self.has_many_keys(def)
            && let Some(ty) = self.many_index(def, &subst, base, index)
        {
            return Some(ty);
        }
        let sub = |c: &Self, x: Option<(Ty, Ty, Span)>| {
            x.map(|(k, o, s)| (c.inf.subst(&k, &subst), c.inf.subst(&o, &subst), s))
        };
        let (index_of, slice_of, set_of) =
            (sub(self, index_of), sub(self, slice_of), sub(self, set_of));
        let key = at(base, index);
        let all_slices = match &index.kind {
            ExprKind::Part { .. } => true,
            ExprKind::Dims(parts) => parts
                .iter()
                .all(|p| matches!(p.kind, ExprKind::Part { .. })),
            _ => false,
        };
        if all_slices {
            let Some((want, me, _)) = slice_of else {
                self.error(
                    index.span,
                    format!(
                        "`{}` has no `Slice`, so it can't be sliced (spec 2.16)",
                        self.show(t)
                    ),
                );
                return Some(Ty::Error);
            };
            let got = self.key_type(index);
            self.key_fits(index, &got, &want);
            self.user_index.insert(key, "slice");
            return Some(me);
        }
        let (want, out, note) = match (index_of, set_of) {
            (Some((k, o, _)), _) => (k, o, "index"),
            (None, Some((k, v, _))) => (k, v, "set?"),
            (None, None) => {
                self.error(
                    index.span,
                    format!(
                        "`{}` has no `Index`, so it can't be indexed (spec 2.16)",
                        self.show(t)
                    ),
                );
                return Some(Ty::Error);
            }
        };
        if let ExprKind::FromEnd(k) = &index.kind {
            self.check_expect(k, &INT);
            let has_len = (self.methods.get(&def)).is_some_and(|m| m.contains_key("len"));
            if !has_len {
                self.error(index.span, format!("`#` counts from the end, so `{}` needs a method `fn len(self) -> Int` (spec 2.16)", self.show(t)));
            }
            self.key_fits(index, &INT, &want);
            self.user_index
                .insert(key, if note == "index" { "index#" } else { note });
            return Some(out);
        }
        let got = self.key_type(index);
        self.key_fits(index, &got, &want);
        self.user_index.insert(key, note);
        Some(out)
    }

    /// The type of a key as written: a tuple for commas, a `Part` for a
    /// slice. Its parts are typed too.
    fn key_type(&mut self, index: &Expr) -> Ty {
        let part = |c: &mut Self, p: &Expr| -> Ty {
            match &p.kind {
                ExprKind::Part { .. } => {
                    c.part_types(p);
                    let part = c.builtin_ty("Part");
                    c.expr_types.push((p.span, part.clone()));
                    part
                }
                ExprKind::FromEnd(_) => {
                    c.error(
                        p.span,
                        "`#` in a key of many parts is not in the language yet (spec 2.16)",
                    );
                    Ty::Error
                }
                _ => c.value(p, None),
            }
        };
        match &index.kind {
            ExprKind::Dims(parts) => {
                let tys: Vec<Ty> = parts.iter().map(|p| part(self, p)).collect();
                let tuple = Ty::Tuple(tys);
                self.expr_types.push((index.span, tuple.clone()));
                tuple
            }
            _ => part(self, index),
        }
    }

    fn key_fits(&mut self, index: &Expr, got: &Ty, want: &Ty) {
        if !self.inf.unify(got, want) && !matches!(self.inf.resolve(got), Ty::Error) {
            let (g, w) = (self.show(got), self.show(want));
            self.error(
                index.span,
                format!("this key is a `{g}`, but the type is indexed with a `{w}` (spec 2.16)"),
            );
        }
    }

    /// `x[k] = v` on a program's own type with `IndexSet`: checks `v`,
    /// notes the call, and says whether it did.
    pub(crate) fn own_index_set(&mut self, target: &Expr, op: AssignOp, value: &Expr) -> bool {
        let ExprKind::Index { base, index } = &target.kind else {
            return false;
        };
        let key = at(base, index);
        let Some(note) = self.user_index.get(&key).copied() else {
            return false;
        };
        let Some((def, targs)) = self.def_of_base(base) else {
            return false;
        };
        let subst = self.subst_of(def, &targs);
        if self.has_many_keys(def) && matches!(note, "index" | "index[]") {
            if op != AssignOp::Set {
                self.error(target.span, "`x[k] op= v` is not for a program's own type: write `x[k] = x[k] op v` (spec 2.16)");
                return true;
            }
            if self.many_index_set(def, &subst, target, value).is_some() {
                return true;
            }
        }
        let Some((set_key, want, _)) = self.operators.get(&(def, "index_set")).cloned() else {
            self.error(
                target.span,
                "this type has no `IndexSet`, so `x[k]` can't be set (spec 2.16)",
            );
            return true;
        };
        if note == "slice" || matches!(index.kind, ExprKind::FromEnd(_)) {
            self.error(target.span, "`IndexSet` sets one key: a slice or `#k` of a program's own type can't be set (spec 2.16)");
            return true;
        }
        let (set_key, want) = (
            self.inf.subst(&set_key, &subst),
            self.inf.subst(&want, &subst),
        );
        if let Some(got) = self.type_at(index.span) {
            self.key_fits(index, &got, &set_key);
        }
        if op != AssignOp::Set {
            self.error(target.span, "`x[k] op= v` is not for a program's own type: write `x[k] = x[k] op v` (spec 2.16)");
        }
        self.check_expect(value, &want);
        self.check_read_only_target(target, "assigned");
        self.user_index.insert(key, "set");
        true
    }

    fn def_of_base(&self, base: &Expr) -> Option<(DefId, Vec<Ty>)> {
        let found = self.expr_types.iter().rev().find(|(s, _)| *s == base.span);
        match found.map(|(_, t)| self.inf.resolve(t)) {
            Some(Ty::Adt(def, args)) => Some((def, args)),
            _ => None,
        }
    }
}
