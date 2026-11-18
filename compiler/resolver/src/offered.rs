//! `pub(out)` (spec 10.1, decision 0127): the names a library offers to
//! other packages. Only a library may write it, and another package may
//! use only those names; the standard library offers all its `pub` ones.

use crate::DefId;
use crate::resolver::Resolver;
use kavach_ast::{Item, ItemKind};

impl Resolver {
    /// `pub(out)` in an application is an error: no package uses one.
    pub(crate) fn no_out_in_applications(&mut self, ranges: &[&[Item]]) {
        for (m, items) in ranges.iter().enumerate() {
            if self.res.modules[m].library {
                continue;
            }
            for item in items.iter().filter(|i| i.is_out) {
                let span = match &item.kind {
                    ItemKind::Fn(f) => f.name.span,
                    ItemKind::Type(t) => t.name.span,
                    _ => item.span,
                };
                self.error(
                    span,
                    "`pub(out)` offers a name to other packages, but an application is used by no other package: write `pub` (spec 10.1)",
                );
            }
        }
    }

    /// Why `name`, found in module `m` of another package, can't be used
    /// here: where to write `pub(out)`, at its declaration when `m` only
    /// re-exports it.
    pub(crate) fn not_offered(&self, m: usize, def: DefId, name: &str) -> String {
        let package = &self.res.modules[m].package;
        let declared = (self.res.defs[def].span).map(|s| self.res.module_at(s.start));
        match declared.filter(|d| *d != m) {
            Some(_) if self.modules.out.contains(&def) => format!(
                "`{name}` is re-exported by module `{}` of package `{package}` with `pub use`, which shares it only inside that package: write `pub(out) use` there (spec 10.2)",
                self.res.modules[m].name
            ),
            Some(d) => format!(
                "`{name}` is re-exported by module `{}` of package `{package}`, but declared `pub` in module `{}`: other packages may use only `pub(out)` names, so mark it `pub(out)` there (spec 10.1)",
                self.res.modules[m].name, self.res.modules[d].name
            ),
            None => format!(
                "`{name}` is `pub` in package `{package}`, which offers other packages only its `pub(out)` names: mark it `pub(out)` there to use it here (spec 10.1)"
            ),
        }
    }

    /// Whether module `m` is in another package than the one being
    /// resolved, so only its `pub(out)` names may be used here.
    pub(crate) fn other_package(&self, m: usize) -> bool {
        let (there, here) = (
            &self.res.modules[m],
            &self.res.modules[self.modules.current],
        );
        there.package != here.package && !there.name.starts_with("std.")
    }
}
