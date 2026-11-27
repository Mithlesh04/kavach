// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Type arguments that must be known (spec 2.9): a generic type in a
//! signature is written with them, and a generic function's are worked
//! out at each call or written there.

use crate::checker::Checker;
use crate::infer::VarKind;
use crate::ty::Ty;
use kavach_ast::{TypeExpr, TypeKind};
use kavach_resolver::{DefId, DefKind};
use std::collections::HashSet;

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

    /// `print(zero())` with `fn zero<T>() -> T?`: a type argument that
    /// nothing at the call or after it worked out.
    pub(crate) fn open_type_args(&mut self) {
        let mut told = HashSet::new();
        for (_, callee, args, span) in self.generic_calls.clone() {
            for (g, t) in &args {
                let Ty::Var(v) = self.inf.resolve(t) else {
                    continue;
                };
                if self.inf.kind(v) != VarKind::Any
                    || self.is_num_param(*g)
                    || self.is_error_param(callee, *g)
                    || !told.insert((span.start, *g))
                    // One mistake, one message: the call, or something
                    // inside it, was refused.
                    || (self.errors.iter())
                        .any(|e| e.span.start >= span.start && e.span.end <= span.end)
                {
                    continue;
                }
                let (name, f) = (self.def_name(*g).to_string(), self.def_name(callee));
                let message = format!(
                    "nothing here says what `{name}` of `{f}` is: write it at the call, as `{f}<Int>(…)` (spec 2.9)"
                );
                self.error(span, message);
            }
        }
    }

    /// Whether type parameter `g` is only what `callee`'s functions fail
    /// with: `! E` of it, or of a function it takes. Given functions that
    /// can't fail, nothing says what `E` is, and the call can't fail
    /// (spec 3.4).
    fn is_error_param(&self, callee: DefId, g: DefId) -> bool {
        let sig = (self.fns.get(&callee).into_iter())
            .chain(self.methods.values().flat_map(|m| m.values()))
            .chain(self.ext_methods.values().flat_map(|m| m.values()))
            .find(|s| s.def == Some(callee));
        let Some(sig) = sig else {
            return false;
        };
        let e = Ty::Param(g);
        sig.errors.as_ref() == Some(&e) || sig.params.iter().any(|p| fails_with(&p.ty, &e))
    }
}

/// Whether a function in `t` fails with `e`.
fn fails_with(t: &Ty, e: &Ty) -> bool {
    match t {
        Ty::Result(_, x) => **x == *e,
        Ty::Fn(ps, r, _) => fails_with(r, e) || ps.iter().any(|p| fails_with(p, e)),
        Ty::List(x) | Ty::Optional(x) | Ty::Set(x) => fails_with(x, e),
        Ty::Map(k, v) => fails_with(k, e) || fails_with(v, e),
        Ty::Tuple(ts) => ts.iter().any(|x| fails_with(x, e)),
        _ => false,
    }
}
