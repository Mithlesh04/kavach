// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Capabilities outside a function's own body (spec 8.1, 2.8, 7.5): an
//! implementation may use only what its interface's method lists
//! (decision 0147); each package's default capabilities (decision 0148);
//! a field's default, paid for where a value is made without it; an
//! invariant, which must be pure; and a function value called in a
//! contract, which must be typed `uses ()`.

use crate::capabilities::{Need, NeedAt};
use crate::checker::{Checker, FieldInfo};
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, Expr, Field, Span};
use kavach_resolver::DefId;
use std::collections::{BTreeSet, HashMap};

/// Code outside any function: a record's (or variant's) field default,
/// by field name, or its invariants (`None`).
pub(crate) type Outside = (DefId, Option<String>);

/// A method that implements an interface's method, for comparing their
/// `uses` once capabilities are known.
#[derive(Clone, Debug)]
pub(crate) struct Implements {
    /// The type's method and its name as shown: `User.describe`.
    pub(crate) method: DefId,
    pub(crate) shown: String,
    /// The interface's method and its name as shown: `Describe.describe`.
    pub(crate) wanted: DefId,
    pub(crate) wanted_shown: String,
    pub(crate) span: Span,
}

/// What `capabilities.rs` collects beyond functions' bodies.
#[derive(Default)]
pub(crate) struct CapParts {
    pub(crate) implements: Vec<Implements>,
    /// The code outside a function being checked, and what each uses.
    pub(crate) outside: Option<Outside>,
    pub(crate) outside_needs: HashMap<Outside, Vec<NeedAt>>,
}

/// What `need` uses: a capability, a function's, or a field default's.
pub(crate) fn caps_of(
    need: &Need,
    effective: &HashMap<DefId, BTreeSet<String>>,
    outside: &HashMap<Outside, Vec<NeedAt>>,
    depth: u8,
) -> BTreeSet<String> {
    match need {
        Need::Cap(c) => BTreeSet::from([c.clone()]),
        Need::Fn(d) => effective.get(d).cloned().unwrap_or_default(),
        // A default that makes a value of its own type stops somewhere.
        Need::Default(d, f) if depth < 8 => (outside.get(&(*d, Some(f.clone()))))
            .into_iter()
            .flatten()
            .flat_map(|n| caps_of(&n.need, effective, outside, depth + 1))
            .collect(),
        Need::Default(..) => BTreeSet::new(),
    }
}

/// What a package may use when `kavach.toml` doesn't say (spec 8.1).
const DEFAULT: &str = "Console";

impl Checker<'_> {
    /// The default capabilities of the package of the code at `at`
    /// (spec 8.1, rule 7): what its functions may use without `uses`.
    pub(crate) fn defaults_at(&self, at: u32) -> BTreeSet<String> {
        let module = self.res.modules.get(self.res.module_at(at));
        // The standard library writes every capability it uses, so an
        // interface of its own, as `ai.Provider`, costs only what it lists.
        if module.is_some_and(|m| m.name.starts_with("std.")) {
            return BTreeSet::new();
        }
        let package = module.map_or("", |m| m.package.as_str());
        match self.opts.defaults.iter().find(|(p, _)| p == package) {
            Some((_, caps)) => caps.iter().cloned().collect(),
            None => BTreeSet::from([DEFAULT.to_string()]),
        }
    }

    /// Each default must have its field's type. What it uses is paid for
    /// where a value is made without the field (`defaults_filled`).
    pub(crate) fn field_defaults(&mut self, owner: DefId, fields: &[Field], infos: &[FieldInfo]) {
        for (field, info) in fields.iter().zip(infos) {
            if let Some(default) = &field.default {
                self.caps.outside = Some((owner, Some(info.name.clone())));
                self.check_expect(default, &info.ty);
                self.caps.outside = None;
            }
        }
    }

    /// A record's invariants: contracts, so pure (spec 7.5), also through
    /// the functions they call.
    pub(crate) fn invariants(&mut self, def: DefId, invariants: &[Expr]) {
        self.record = Some(def);
        self.in_contract = true;
        self.caps.outside = Some((def, None));
        for inv in invariants {
            self.check_expect(inv, &Ty::Prim(Prim::Bool));
        }
        self.caps.outside = None;
        self.in_contract = false;
        self.record = None;
    }

    /// `owner(…)` made without some fields: their defaults run here.
    pub(crate) fn defaults_filled(
        &mut self,
        owner: DefId,
        fields: &[FieldInfo],
        args: &[Arg],
        span: Span,
    ) {
        let given = |f: &&FieldInfo| {
            (args.iter()).any(|a| a.name.as_ref().is_some_and(|n| n.name == f.name))
        };
        let left: Vec<String> = (fields.iter())
            .filter(|f| f.has_default && !given(f))
            .map(|f| f.name.clone())
            .collect();
        for name in left {
            self.note(Need::Default(owner, name), span);
        }
    }

    /// A function value called in a contract: only one whose type says
    /// it uses nothing, `uses ()` (spec 7.5, 3.4).
    pub(crate) fn fn_value_in_contract(&mut self, fty: &Ty, callee: &str, span: Span) {
        let Ty::Fn(params, ret, limit) = fty else {
            return;
        };
        if !self.in_contract || limit.as_ref().is_some_and(|l| l.is_empty()) {
            return;
        }
        let pure = self.show(&Ty::Fn(params.clone(), ret.clone(), Some(Vec::new())));
        self.error(
            span,
            format!("{callee} may use capabilities, and contracts must be pure: give it the type `{pure}` (spec 7.5)"),
        );
    }

    /// Notes that `method` implements `wanted` (spec 2.8).
    pub(crate) fn note_implements(&mut self, implements: Implements) {
        self.caps.implements.push(implements);
    }

    /// An implementation that uses more than its interface's method lists
    /// (decision 0147). `allowed` is what each method may use, `effective`
    /// what it does.
    pub(crate) fn implementations_overreach(
        &mut self,
        effective: &HashMap<DefId, BTreeSet<String>>,
        allowed: &HashMap<DefId, BTreeSet<String>>,
    ) -> Vec<(Span, String)> {
        let mut errors = Vec::new();
        for i in std::mem::take(&mut self.caps.implements) {
            let Some(may) = allowed.get(&i.wanted) else {
                continue;
            };
            let used = effective.get(&i.method).cloned().unwrap_or_default();
            let over: Vec<&String> = (used.iter())
                .filter(|c| !may.iter().any(|m| kavach_resolver::cap_covers(m, c)))
                .collect();
            let Some(first) = over.first() else {
                continue;
            };
            let lists = match may.is_empty() {
                true => "uses nothing".to_string(),
                false => {
                    let names: Vec<&str> = may.iter().map(String::as_str).collect();
                    format!("uses only {}", names.join(", "))
                }
            };
            let span = self.res.defs[i.method].span.unwrap_or(i.span);
            errors.push((span, format!(
                "`{}` uses {first}, but `{}` {lists}: an implementation may use only what its interface's method lists, so add `uses {first}` to `{}` or stop using it here (spec 2.8)",
                i.shown, i.wanted_shown, i.wanted_shown
            )));
        }
        errors
    }
}
