//! Names declared and never used (spec 1.4): a warning, so code being
//! written still runs. A name starting with `_` says it is unused on
//! purpose. Only the program's own modules are looked at, not the
//! standard library's.

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
        let mut found = Vec::new();
        for (id, d) in self.res.defs.iter().enumerate() {
            if !matches!(d.kind, DefKind::Local { .. })
                || d.name.starts_with('_')
                || used.contains(&id)
            {
                continue;
            }
            let Some(span) = d.span else { continue };
            let module = &self.res.modules[self.res.module_at(span.start)].name;
            if module.starts_with("std.") {
                continue;
            }
            let message = match self.hides.contains_key(&id) {
                true => format!(
                    "`{0}` is never used: this makes a new `{0}`, hiding the one declared above. To change that one, write `{0} = …` without `let` (spec 1.4)",
                    d.name
                ),
                false => format!(
                    "`{}` is never used: remove it, or start its name with `_` if that is on purpose",
                    d.name
                ),
            };
            found.push(Diagnostic { message, span });
        }
        self.res.warnings.extend(found);
    }
}
