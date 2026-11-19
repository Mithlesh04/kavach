// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Number bounds (spec 2.9, decision 0115): `T: Number` and `T: Floating`
//! let generic code do arithmetic on `T`. A literal may be a `T`
//! (`infer.rs`), `T(x)` converts a number to `T`, `std.math` takes a
//! `Floating` `T`, and each call is checked to give a number type.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};

impl Checker<'_> {
    /// Notes that type parameter `def` has a number bound, if `g` has one.
    pub(crate) fn note_number_bound(&mut self, g: &GenericParam, def: DefId) {
        // `width: Int` is a number parameter, not a bound (spec 2.9).
        if g.is_number() {
            return;
        }
        self.note_op_bounds(g, def);
        for b in &g.bounds {
            let TypeKind::Named { path, args } = &b.kind else {
                continue;
            };
            self.check_bound_is_interface(path, b.span);
            match path.segments.last().map(|s| s.name.as_str()) {
                Some("Number") if args.is_empty() && path.segments.len() == 1 => {
                    self.inf.numbers.entry(def).or_insert(false);
                }
                Some("Floating") if args.is_empty() && path.segments.len() == 1 => {
                    self.inf.numbers.insert(def, true);
                }
                _ => {}
            }
        }
    }

    /// A bound names an interface (2.9): `<T: Float>` or `<T: User>` names
    /// a type, which nothing could meet but itself.
    fn check_bound_is_interface(&mut self, path: &Path, span: Span) {
        let Some(last) = path.segments.last() else {
            return;
        };
        let Some(&def) = self.res.uses.get(&last.span.start) else {
            return;
        };
        let d = &self.res.defs[def];
        let builtin_type = d.kind == DefKind::Builtin
            && (Prim::from_name(&d.name).is_some()
                || matches!(d.name.as_str(), "List" | "Dict" | "Set" | "Ring" | "Json"));
        if d.kind != DefKind::Type && !builtin_type {
            return;
        }
        let hint = match d.name.as_str() {
            "Float" | "Float32" => ": for floating-point numbers the bound is `Floating`",
            n if Prim::from_name(n).is_some() => ": for any number the bound is `Number`",
            _ => "",
        };
        let message = format!(
            "`{}` is a type, and a bound names an interface{hint} (spec 2.9)",
            d.name
        );
        if !self
            .errors
            .iter()
            .any(|e| e.span == span && e.message == message)
        {
            self.error(span, message);
        }
    }

    /// Whether `t` is a type parameter with a number bound; with
    /// `floating`, a `Floating` one.
    pub(crate) fn number_param(&self, t: &Ty, floating: bool) -> bool {
        match self.inf.resolve(t) {
            Ty::Param(p) => self.inf.numbers.get(&p).is_some_and(|f| *f || !floating),
            _ => false,
        }
    }

    /// Whether `t` meets the bound of `param`, a type parameter with a
    /// number bound; `None` when `param` has none, or `t` isn't known.
    pub(crate) fn meets_number_bound(&self, param: DefId, t: &Ty) -> Option<bool> {
        let floating = *self.inf.numbers.get(&param)?;
        Some(match self.inf.resolve(t) {
            Ty::Prim(p) if floating => matches!(p, Prim::Float | Prim::Float32),
            Ty::Prim(p) => p.is_numeric(),
            Ty::Param(_) => self.number_param(t, floating),
            Ty::Var(_) | Ty::Error | Ty::Never => return None,
            _ => false,
        })
    }

    /// The message for `t`, which doesn't meet `param`'s number bound.
    pub(crate) fn not_a_number(&self, param: DefId, t: &Ty) -> String {
        let (name, shown) = (&self.res.defs[param].name, self.show(t));
        match self.inf.numbers.get(&param) {
            Some(true) => format!(
                "`{name}` must be `Float` or `Float32` (`{name}: Floating`), and this is `{shown}` (spec 2.9)"
            ),
            _ => format!(
                "`{name}` must be a number type (`{name}: Number`), and this is `{shown}` (spec 2.9)"
            ),
        }
    }

    /// `T(x)`, with `T` a type parameter with a number bound: `x` as a `T`.
    pub(crate) fn number_conversion(&mut self, callee: &Expr, args: &[Arg]) -> Option<Ty> {
        let ExprKind::Name(id) = &callee.kind else {
            return None;
        };
        let &def = self.res.uses.get(&id.span.start)?;
        if !self.inf.numbers.contains_key(&def) {
            return None;
        }
        match args {
            [a] if a.name.is_none() => {
                let t = self.value(&a.value, None);
                self.require_number(&a.value, &t, false);
            }
            _ => {
                let what = format!("`{}(x)` takes one number", id.name);
                self.error(
                    callee.span,
                    format!("{what}, which it gives as a `{}` (spec 2.9)", id.name),
                );
            }
        }
        Some(Ty::Param(def))
    }
}

impl Checker<'_> {
    /// `x * 0.5` with `x` of a `T` that is only a `Number`: says why
    /// (spec 2.9). Whether it did.
    pub(crate) fn fraction_for_number(&mut self, lt: &Ty, r: &Expr) -> bool {
        let fraction = matches!(r.kind, ExprKind::Float(_));
        if !fraction || !self.number_param(lt, false) || self.number_param(lt, true) {
            return false;
        }
        let name = self.show(lt);
        self.error(r.span, format!("a number with a fraction is a `{name}` only when `{name}` is `Floating`: write `{name}: Floating`, or a whole number (spec 2.9)"));
        true
    }
}

impl Checker<'_> {
    /// Checks that `t`, given for type parameter `g`, meets `g`'s number
    /// bound; a literal given for a `Floating` one is a `Float` (`half(3)`).
    pub(crate) fn check_number_bound(&mut self, g: DefId, t: &Ty, span: Span) {
        if self.inf.numbers.contains_key(&g) || self.param_ops.contains_key(&g) {
            self.number_checks.push((g, t.clone(), span));
        }
    }

    /// The number bounds noted while checking, now each literal's type is
    /// known from where it went.
    pub(crate) fn check_numbers_later(&mut self) {
        for (g, t, span) in std::mem::take(&mut self.number_checks) {
            self.number_bound_now(g, &t, span);
            self.op_bounds_now(g, &t, span);
        }
        self.wheres_now();
        self.filled_now();
    }

    /// A type argument nothing gave a type: no field holds it, it wasn't
    /// written, and no use of the value fixed it (2.9).
    fn filled_now(&mut self) {
        let mut told = std::collections::HashSet::new();
        for (g, t, span) in std::mem::take(&mut self.filled_checks) {
            let Ty::Var(v) = self.inf.resolve(&t) else {
                continue;
            };
            if self.inf.kind(v) != crate::infer::VarKind::Any || !told.insert((span.start, g)) {
                continue;
            }
            let name = self.res.defs[g].name.clone();
            let owner = (self.res.defs.iter().zip(0..))
                .find(|(_, d)| self.types.get(d).is_some_and(|t| t.generics.contains(&g)))
                .map_or(String::new(), |(d, _)| d.name.clone());
            let example = match self.is_num_param(g) {
                true => format!("`{owner}<…, {name}: 3>(…)`"),
                false => format!("`{owner}<Int>(…)`"),
            };
            self.error(
                span,
                format!("nothing here says what `{name}` of `{owner}` is: write it, as {example} (spec 2.9)"),
            );
        }
    }

    fn number_bound_now(&mut self, g: DefId, t: &Ty, span: Span) {
        let mut ty = self.inf.resolve(t);
        if let Ty::Var(v) = ty
            && self.inf.numbers.get(&g) == Some(&true)
            && self.inf.kind(v) != crate::infer::VarKind::Any
        {
            self.inf.unify(&ty, &Ty::Prim(Prim::Float));
            ty = self.inf.resolve(&ty);
        }
        if self.meets_number_bound(g, &ty) == Some(false) {
            let why = self.not_a_number(g, &ty);
            self.error(span, why);
        }
    }
}

impl Checker<'_> {
    /// A float literal that became a `Float32` must fit one (2.1): a
    /// `1e300` there would be `inf`.
    pub(crate) fn check_float32_literals(&mut self) {
        let float32 = Ty::Prim(Prim::Float32);
        for (span, text) in std::mem::take(&mut self.float_texts) {
            let ty = self.literals.iter().find(|(s, _)| *s == span.start);
            let Some((_, ty)) = ty else { continue };
            let wide: f64 = text.replace('_', "").parse().unwrap_or(0.0);
            if self.inf.resolve(ty) == float32 && wide.is_finite() && (wide as f32).is_infinite() {
                self.error(
                    span,
                    format!("`{text}` doesn't fit `Float32`, whose largest is about 3.4e38: it would be `inf` (spec 2.1)"),
                );
            }
        }
    }
}
