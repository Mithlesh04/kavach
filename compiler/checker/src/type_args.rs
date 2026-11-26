// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Type arguments that must be known (spec 2.9): a generic type in a
//! signature is written with them.

use crate::checker::Checker;
use kavach_ast::{TypeExpr, TypeKind};
use kavach_resolver::DefKind;

impl Checker<'_> {
    /// `fn f(p: Pair)`: a generic type in a signature without its type
    /// arguments would fit every instance of it, so it is an error.
    pub(crate) fn bare_generics(&mut self, t: &TypeExpr) {
        match &t.kind {
            TypeKind::Named { path, args } => {
                args.iter().for_each(|a| self.bare_generics(a));
                if args.iter().any(|a| !matches!(a.kind, TypeKind::Num { .. })) {
                    return;
                }
                let Some(named) = path.segments.last() else {
                    return;
                };
                let Some(&def) = self.res.uses.get(&named.span.start) else {
                    return;
                };
                if self.res.defs[def].kind != DefKind::Type || self.aliases.contains_key(&def) {
                    return;
                }
                // Number parameters are checked with the type (`sizes.rs`).
                let missing: Vec<String> = (self.generic_list(def).into_iter())
                    .filter(|g| !self.is_num_param(*g))
                    .map(|g| format!("`{}`", self.def_name(g)))
                    .collect();
                if missing.is_empty() || self.errors.iter().any(|e| e.span == t.span) {
                    return;
                }
                let name = &named.name;
                let (what, list) = match missing.len() {
                    1 => ("argument", missing[0].clone()),
                    _ => ("arguments", missing.join(" and ")),
                };
                let fill = vec!["Int"; missing.len()].join(", ");
                self.error(
                    t.span,
                    format!("`{name}` needs its type {what} {list}: write `{name}<{fill}>`, or the function's own type parameter, as `{name}<T>` (spec 2.9)"),
                );
            }
            TypeKind::List(x) | TypeKind::Optional(x) | TypeKind::WithNums(x, _) => {
                self.bare_generics(x)
            }
            TypeKind::Map(k, v) => {
                self.bare_generics(k);
                self.bare_generics(v);
            }
            TypeKind::Tuple(ts) => ts.iter().for_each(|x| self.bare_generics(x)),
            TypeKind::AnonRecord(fs) => fs.iter().for_each(|(_, x)| self.bare_generics(x)),
            TypeKind::Fn {
                params,
                ret,
                errors,
                ..
            } => {
                params.iter().for_each(|x| self.bare_generics(x));
                ret.iter().chain(errors).for_each(|x| self.bare_generics(x));
            }
            TypeKind::Num { .. } | TypeKind::Error => {}
        }
    }
}
