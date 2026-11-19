// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! An operator or indexing interface implemented more than once (spec
//! 2.16, decision 0136): `Mul<Vec2>` and `Mul<Float>`, each with its own
//! `fn mul`. The driver names the second and later methods written with
//! one name `mul#2`, `mul#3` … (`compiler/driver/src/variants.rs`), so
//! every later stage sees one method per name. Here each implementation
//! finds its method by the type it takes, and each use chooses the
//! implementation by the type of the right operand or the key.

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::{Arg, Span};
use kavach_resolver::DefId;
use std::collections::HashMap;

/// One implementation: the type it takes, the type it gives, and where
/// it is written.
pub(crate) type Impl = (Ty, Ty, Span);

impl Checker<'_> {
    /// Notes one implementation of `method`: the first in `operators`,
    /// the others in `more_impls`.
    pub(crate) fn note_impl(&mut self, def: DefId, method: &'static str, found: Impl) {
        match self.operators.entry((def, method)) {
            std::collections::hash_map::Entry::Occupied(_) => {
                self.more_impls
                    .entry((def, method))
                    .or_default()
                    .push(found);
            }
            std::collections::hash_map::Entry::Vacant(v) => {
                v.insert(found);
            }
        }
    }

    /// Every implementation of `method` that `def` has, in the order
    /// written.
    pub(crate) fn impls(&self, def: DefId, method: &'static str) -> Vec<Impl> {
        let mut out: Vec<Impl> = self
            .operators
            .get(&(def, method))
            .cloned()
            .into_iter()
            .collect();
        out.extend(
            self.more_impls
                .get(&(def, method))
                .cloned()
                .unwrap_or_default(),
        );
        out
    }

    /// The names of the methods written as `method` in `def`: `mul`, then
    /// `mul#2`, `mul#3` … for the later ones.
    pub(crate) fn variants(&self, def: DefId, method: &str) -> Vec<String> {
        let Some(ms) = self.methods.get(&def) else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        if ms.contains_key(method) {
            out.push(method.to_string());
        }
        let mut n = 2;
        while ms.contains_key(&format!("{method}#{n}")) {
            out.push(format!("{method}#{n}"));
            n += 1;
        }
        out
    }

    /// The method that implements `method` for `other`: the variant whose
    /// first parameter is `other`, else the plain name.
    pub(crate) fn variant_for(&self, def: DefId, method: &str, other: &Ty) -> String {
        let want = self.inf.resolve(other);
        for name in self.variants(def, method) {
            let sig = self.methods.get(&def).and_then(|m| m.get(&name));
            if sig.is_some_and(|s| {
                (s.params.first()).is_some_and(|p| self.inf.resolve(&p.ty) == want)
            }) {
                return name;
            }
        }
        method.to_string()
    }

    /// The implementation, of `impls`, that takes `got`, the type of the
    /// right operand or key, with the type's own parameters as `subst`
    /// gives them. Types are unified only with the one chosen.
    pub(crate) fn choose_impl(
        &mut self,
        impls: &[Impl],
        subst: &HashMap<DefId, Ty>,
        got: &Ty,
    ) -> Option<usize> {
        for (i, (other, ..)) in impls.iter().enumerate() {
            let want = self.inf.subst(other, subst);
            let before = self.inf.clone();
            if self.inf.unify(got, &want) {
                return Some(i);
            }
            self.inf = before;
        }
        None
    }

    /// The types `impls` take, for a message: "a `Vec2` or a `Float`".
    pub(crate) fn shown_impls(&self, impls: &[Impl], subst: &HashMap<DefId, Ty>) -> String {
        let shown: Vec<String> = (impls.iter())
            .map(|(o, ..)| a_shown(&self.show(&self.inf.subst(o, subst))))
            .collect();
        match shown.split_last() {
            Some((last, [])) => last.clone(),
            Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
            None => String::new(),
        }
    }

    /// Two implementations of one interface for the same type are an
    /// error (spec 2.16): `what` names the operand, "right operand" or
    /// "key".
    pub(crate) fn check_impls_differ(
        &mut self,
        def: DefId,
        method: &'static str,
        iface: &str,
        what: &str,
    ) {
        let impls = self.impls(def, method);
        for (i, (o, _, span)) in impls.iter().enumerate() {
            let o = self.inf.resolve(o);
            if impls[..i].iter().any(|(p, ..)| self.inf.resolve(p) == o) {
                let (ty, shown) = (self.res.defs[def].name.clone(), self.show(&o));
                self.error(*span, format!("`{ty}` implements `{iface}` twice for a `{shown}` {what}: each implementation takes another type (spec 2.16)"));
            }
        }
    }
}

impl Checker<'_> {
    /// A call of a method the standard library writes more than once,
    /// as `std.tensor`'s `sum()` and `sum(axis:)` (spec 14.33): the
    /// variant whose parameters take the arguments, by their names. `None`
    /// when the method is written once.
    pub(crate) fn call_variant(&self, def: DefId, name: &str, args: &[Arg]) -> Option<String> {
        let found = self.variants(def, name);
        if found.len() < 2 {
            return None;
        }
        let named: Vec<&str> = (args.iter())
            .filter_map(|a| a.name.as_ref().map(|n| n.name.as_str()))
            .collect();
        let given = args.iter().filter(|a| a.name.is_none()).count();
        found.into_iter().find(|v| {
            let Some(sig) = self.methods.get(&def).and_then(|m| m.get(v)) else {
                return false;
            };
            let names: Vec<&str> = sig.params.iter().map(|p| p.name.as_str()).collect();
            named
                .iter()
                .all(|n| names[given.min(names.len())..].contains(n))
                && given <= names.len()
                && (sig.params.iter().enumerate())
                    .all(|(i, p)| p.has_default || i < given || named.contains(&p.name.as_str()))
        })
    }
}

/// `` a `Float` `` or `` an `Int` ``: a type's name in a message, with its
/// article.
pub(crate) fn a_shown(shown: &str) -> String {
    let an = shown.starts_with(['A', 'E', 'I', 'O', 'a', 'e', 'i', 'o']);
    format!("{} `{shown}`", if an { "an" } else { "a" })
}
