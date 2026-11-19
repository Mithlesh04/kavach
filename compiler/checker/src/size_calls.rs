// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Making values whose types have numbers (spec 2.2, 2.9): `try
//! List<Int, max: 2>(xs)`, which fails with a `SizeError`, and `Ring<T,
//! max: n>(items)`; and settling the numbers each call was given, for
//! code generation.

use crate::checker::Checker;
use crate::ty::{Prim, SizeKind, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;
use std::collections::HashMap;

impl Checker<'_> {
    /// `List<Int, max: 2>(xs)`, `Dict<…>(d)`, `Set<…>(s)`, `Bytes<len:
    /// 32>(b)`, and `Ring<T, max: n>(items)` or `Ring()`. `None` for any
    /// other call.
    pub(crate) fn sized_call(
        &mut self,
        name: &str,
        generics: &[TypeExpr],
        args: &[Arg],
        span: Span,
        expected: Option<&Ty>,
    ) -> Option<Ty> {
        if !matches!(name, "List" | "Dict" | "Set" | "Bytes" | "Ring") {
            return None;
        }
        let nums: Vec<&TypeExpr> = (generics.iter())
            .filter(|g| matches!(g.kind, TypeKind::Num { .. }))
            .collect();
        if name == "Ring" {
            return Some(self.ring_call(generics, &nums, args, span, expected));
        }
        if nums.is_empty() {
            return None;
        }
        let mut types: Vec<Ty> = (generics.iter())
            .filter(|g| !matches!(g.kind, TypeKind::Num { .. }))
            .map(|g| self.lower(g, None))
            .collect();
        if types.is_empty() {
            types = match name {
                "Dict" => vec![self.fresh(), self.fresh()],
                _ => vec![self.fresh()],
            };
        }
        let base = crate::collect::builtin_type(name, types, None);
        let target = self.sized_type(name, base.clone(), &nums);
        let Ty::Sized(_, kind, n) = &target else {
            for a in args {
                self.value(&a.value, None);
            }
            return Some(Ty::Error);
        };
        let (kind, n) = (*kind, (**n).clone());
        let [arg] = args else {
            self.error(
                span,
                format!("`{}` takes one value: the one to check", self.show(&target)),
            );
            for a in args {
                self.value(&a.value, None);
            }
            return Some(target);
        };
        // A literal is checked while compiling (spec 2.9).
        if self.literal_count(&arg.value).is_some() {
            self.check_expect(&arg.value, &target);
            return Some(target);
        }
        self.check_expect(&arg.value, &base);
        let key = (span.start, span.end);
        let shown = self.conversion_name(&Ty::Sized(Box::new(base), kind, Box::new(Ty::Error)));
        let shown = shown.replace(": ?>", ": {n}>");
        self.sizes.checks.insert(key, (kind, n.clone(), shown));
        if kind == SizeKind::Max {
            self.sizes.limits.insert(key, n);
        }
        Some(Ty::Result(
            Box::new(target),
            Box::new(Ty::Builtin("SizeError", Vec::new())),
        ))
    }

    /// `Ring<Float, max: 3>()`, `Ring<String, max: 2>(items)`, or `Ring()`
    /// where the wanted type says its size (spec 2.2). Given more items
    /// than it holds, it keeps the last ones, so it never fails.
    fn ring_call(
        &mut self,
        generics: &[TypeExpr],
        nums: &[&TypeExpr],
        args: &[Arg],
        span: Span,
        expected: Option<&Ty>,
    ) -> Ty {
        let target = if nums.is_empty() {
            match expected.map(|t| self.inf.shallow_keep(t)) {
                Some(t @ Ty::Sized(..)) => t,
                _ => {
                    self.error(span, "a ring's type says how many items it keeps: `Ring<Float, max: 100>()`, or `Ring.capacity(n)` for a number known only while running (spec 2.2)");
                    for a in args {
                        self.value(&a.value, None);
                    }
                    return Ty::Error;
                }
            }
        } else {
            let item = match generics
                .iter()
                .find(|g| !matches!(g.kind, TypeKind::Num { .. }))
            {
                Some(g) => self.lower(g, None),
                None => self.fresh(),
            };
            self.sized_type("Ring", Ty::Builtin("Ring", vec![item]), nums)
        };
        let Ty::Sized(base, _, n) = &target else {
            return Ty::Error;
        };
        let item = match self.inf.shallow(base) {
            Ty::Builtin("Ring", ts) => ts.first().cloned().unwrap_or(Ty::Error),
            _ => Ty::Error,
        };
        match args {
            [] => {}
            [a] if a.name.is_none() => {
                self.check_expect(&a.value, &Ty::List(Box::new(item)));
            }
            _ => {
                self.error(
                    span,
                    "a ring takes its first items as one list: `Ring<T, max: n>(items)` (spec 2.2)",
                );
                for a in args {
                    self.value(&a.value, None);
                }
            }
        }
        self.sizes
            .limits
            .insert((span.start, span.end), (**n).clone());
        target
    }

    /// `Bytes([1, 2, 3])`: bytes from a list literal of `n` numbers are a
    /// `Bytes<len: n>` (spec 2.13).
    pub(crate) fn bytes_of_literal(&self, args: &[Arg]) -> Ty {
        let bytes = Ty::Prim(Prim::Bytes);
        match args {
            [a] if a.name.is_none() => match &a.value.kind {
                ExprKind::List(items) => {
                    let n = Box::new(Ty::Num(items.len() as i64));
                    Ty::Sized(Box::new(bytes), SizeKind::Len, n)
                }
                _ => bytes,
            },
            _ => bytes,
        }
    }

    /// The number parameters given at each call, the limits and the
    /// checks, settled now every type is known: a number nothing gave
    /// takes its default, or is an error.
    pub(crate) fn settle_numbers(&mut self) -> Settled {
        let mut calls = HashMap::new();
        for (span, called, given) in std::mem::take(&mut self.sizes.calls) {
            let mut out = Vec::new();
            for (g, t) in given {
                let t = match self.inf.resolve_keep(&t) {
                    Ty::Var(_) => match self.sizes.params.get(&g).cloned().flatten() {
                        Some(d) => {
                            let v = self.num_arg(&d);
                            self.inf.unify(&t, &v);
                            v
                        }
                        None => {
                            let name = self.def_name(g).to_string();
                            self.error(span, format!("nothing here says what the number `{name}` is: write it at the call, as `{called}<{name}: 4>(…)` (spec 2.9)"));
                            Ty::Error
                        }
                    },
                    t => t,
                };
                out.push((g, t));
            }
            calls.insert((span.start, span.end), out);
        }
        let limits = (self.sizes.limits.iter())
            .map(|(k, t)| (*k, self.inf.resolve_keep(t)))
            .collect();
        let checks = (self.sizes.checks.iter())
            .map(|(k, (kind, t, s))| (*k, (*kind, self.inf.resolve_keep(t), s.clone())))
            .collect();
        let defaults = (self.sizes.params.iter())
            .filter_map(|(g, d)| Some((*g, self.const_num(d.as_ref()?)?)))
            .collect();
        Settled {
            calls,
            limits,
            checks,
            params: self.sizes.params.keys().copied().collect(),
            consts: self.sizes.consts.clone(),
            defaults,
        }
    }
}

/// The numbers code generation needs (spec 2.9).
#[derive(Clone, Debug, Default)]
pub struct Settled {
    /// The number arguments of each call of a function with number
    /// parameters, by its span: a `Num`, or the `Param` of the caller's.
    pub calls: HashMap<(u32, u32), Vec<(DefId, Ty)>>,
    /// Values made with a `max` limit, by span: a `Num` or a `Param`.
    pub limits: HashMap<(u32, u32), Ty>,
    /// Conversions checked while running, by span, with the type as it
    /// is shown, `{n}` standing for its number.
    pub checks: HashMap<(u32, u32), (SizeKind, Ty, String)>,
    /// Every number parameter.
    pub params: std::collections::HashSet<DefId>,
    /// The constants known while compiling, and number parameters'
    /// defaults.
    pub consts: HashMap<DefId, i64>,
    pub defaults: HashMap<DefId, i64>,
}
