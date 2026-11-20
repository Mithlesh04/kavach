// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Methods added to lists, dicts, sets and rings with `extend<T> [T]`
//! (spec 2.5): kept by the kind of collection, for every item type.

use crate::checker::{Checker, FnSig};
use crate::ty::Ty;
use kavach_ast::*;
use kavach_resolver::DefId;
use std::collections::HashMap;

/// `List`, `Map`, `Set` or `Ring`: the kind of collection `t` is, which
/// methods added with `extend<T> [T]` are kept by (spec 2.5).
pub(crate) fn collection_kind(t: &Ty) -> Option<&'static str> {
    match t {
        Ty::List(_) => Some("List"),
        Ty::Map(..) => Some("Dict"),
        Ty::Set(_) => Some("Set"),
        Ty::Builtin("Ring", _) => Some("Ring"),
        _ => None,
    }
}

/// Records what each parameter of `written` (`[T]`) stands for in
/// `actual` (`[Int]`).
fn bind(written: &Ty, actual: &Ty, out: &mut HashMap<DefId, Ty>) {
    match (written, actual) {
        (Ty::Param(p), t) => {
            out.entry(*p).or_insert_with(|| t.clone());
        }
        (Ty::List(w), Ty::List(a)) | (Ty::Set(w), Ty::Set(a)) => bind(w, a, out),
        (Ty::Map(wk, wv), Ty::Map(ak, av)) => {
            bind(wk, ak, out);
            bind(wv, av, out);
        }
        (Ty::Builtin(_, ws), Ty::Builtin(_, xs)) => {
            ws.iter().zip(xs).for_each(|(w, a)| bind(w, a, out));
        }
        _ => {}
    }
}

impl Checker<'_> {
    /// A method added to the lists, maps, sets or rings `recv` is one of,
    /// with what the extend's parameters stand for here.
    pub(crate) fn collection_ext(
        &self,
        recv: &Ty,
        name: &str,
    ) -> Option<(FnSig, HashMap<DefId, Ty>)> {
        let kind = collection_kind(recv)?;
        let sig = self.ext_methods.get(kind)?.get(name)?.clone();
        let mut subst = HashMap::new();
        if let Some(written) = self.ext_targets.get(&(kind.to_string(), name.to_string())) {
            bind(written, recv, &mut subst);
        }
        Some((sig, subst))
    }

    /// `extend<T> [T]` (spec 2.5): its methods, kept for every list, map,
    /// set or ring; a name the collection already has is an error.
    pub(crate) fn collect_collection_ext(
        &mut self,
        target: &Ty,
        sigs: Vec<(String, FnSig)>,
        methods: &[Item],
    ) {
        let Some(kind) = collection_kind(target) else {
            return;
        };
        let own = crate::builtin_methods(target, self.res);
        for (name, sig) in sigs {
            if own.contains(&name.as_str()) {
                let span = methods
                    .iter()
                    .find_map(|m| match &m.kind {
                        ItemKind::Fn(f) if f.name.name == name => Some(f.name.span),
                        _ => None,
                    })
                    .unwrap_or(Span { start: 0, end: 0 });
                self.error(
                    span,
                    format!(
                        "`{}` already has a method `{name}`: give yours another name (spec 2.5)",
                        self.show(target)
                    ),
                );
                continue;
            }
            if self.added_twice(kind, &name, &sig, target) {
                continue;
            }
            self.ext_targets
                .insert((kind.to_string(), name.clone()), target.clone());
            self.ext_methods
                .entry(kind.to_string())
                .or_default()
                .insert(name, sig);
        }
    }
}
