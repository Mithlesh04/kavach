// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Names declared and never used (spec 1.4, 10.1): a warning, so code
//! being written still runs. A name starting with `_` says it is unused
//! on purpose. Only the program's own modules are looked at, not the
//! standard library's or a dependency's.

use crate::resolver::Resolver;
use crate::{DefKind, Diagnostic};
use std::collections::HashSet;

impl Resolver {
    pub(crate) fn warn_unused(&mut self) {
        // A misspelled use (`totl`) leaves its name unused: the error says
        // it all, so no warning joins it.
        if !self.res.errors.is_empty() {
            return;
        }
        let used: HashSet<usize> = self.res.uses.values().copied().collect();
        // Each module's own functions, types and constants (10.1).
        let top: HashSet<usize> = self
            .modules
            .own
            .iter()
            .flat_map(|o| o.values().copied())
            .collect();
        // An entry file without `main` is a file of functions `kavach
        // test` checks (spec 11): they are what it is for.
        let tested = !self.res.modules.first().is_some_and(|m| m.library)
            && self
                .modules
                .own
                .first()
                .is_some_and(|o| !o.contains_key("main"));
        let mut found = Vec::new();
        for (id, d) in self.res.defs.iter().enumerate() {
            let declared = matches!(d.kind, DefKind::Fn | DefKind::Type) && top.contains(&id);
            if !(matches!(d.kind, DefKind::Local { .. }) || declared)
                || d.name.starts_with('_')
                || used.contains(&id)
                || self.modules.out.contains(&id)
                || (d.kind == DefKind::Fn && d.name == "main")
            {
                continue;
            }
            let Some(span) = d.span else { continue };
            // An `inout` parameter is a variable, but a parameter's own
            // rule decides (`unused_params.rs`).
            if self.param_spans.contains(&span.start) {
                continue;
            }
            let at = self.res.module_at(span.start);
            let module = &self.res.modules[at];
            if module.name.starts_with("std.")
                || !module.package.is_empty()
                || (tested && at == 0 && d.kind == DefKind::Fn)
            {
                continue;
            }
            let message = if self.modules.public.contains(&id) {
                // `pub`: used by none of the package's modules.
                let out = match module.library {
                    true => ", or make it `pub(out)` if other packages use it",
                    false => "",
                };
                format!(
                    "`{}` is `pub`, but nothing in this package uses it: remove it{out} (spec 10.1)",
                    d.name
                )
            } else if self.hides.contains_key(&id) {
                format!(
                    "`{0}` is never used: this makes a new `{0}`, hiding the one declared above. To change that one, write `{0} = …` without `let` (spec 1.4)",
                    d.name
                )
            } else {
                format!(
                    "`{}` is never used: remove it, or start its name with `_` if that is on purpose",
                    d.name
                )
            };
            found.push(Diagnostic { message, span });
        }
        self.res.warnings.extend(found);
    }
}
