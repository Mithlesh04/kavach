//! `pub(out)` (spec 10.1, decision 0127): the names a library offers to
//! other packages. Only a library may write it, and another package may
//! use only those names; the standard library offers all its `pub` ones.

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
