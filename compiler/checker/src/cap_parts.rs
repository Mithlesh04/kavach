// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Capabilities outside a function's own body (spec 8.1, 2.8): an
//! implementation may use only what its interface's method lists
//! (decision 0147).

use crate::checker::Checker;
use kavach_ast::Span;
use kavach_resolver::DefId;
use std::collections::{BTreeSet, HashMap};

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
}

impl Checker<'_> {
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
