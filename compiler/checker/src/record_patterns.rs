// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Record patterns (spec 4.3): `Customer(name, age)` takes a record of
//! that type apart, binding its fields as a variant pattern does. Every
//! value of the type fits its shape; `coverage.rs` counts that.

use crate::checker::{Checker, Shape};
use crate::ty::Ty;
use kavach_ast::*;
use kavach_resolver::DefId;

impl Checker<'_> {
    /// Gives the names in record pattern `p` their types. `name` is the
    /// type's name as written, `def` the type, and `ty` the value matched
    /// (inside an optional, its present value).
    pub(crate) fn bind_record(
        &mut self,
        p: &Pattern,
        name: &Ident,
        def: DefId,
        fields: Option<&[FieldPattern]>,
        ty: &Ty,
    ) {
        let Some(Shape::Record(infos)) = self.types.get(&def).map(|t| t.shape.clone()) else {
            return;
        };
        let (instance, subst) = self.instance(def);
        if !self.inf.unify(ty, &instance) {
            let shown = self.show(ty);
            self.error(
                p.span,
                format!(
                    "`{0}(…)` matches a `{0}`, but this value is `{shown}`",
                    name.name
                ),
            );
            return;
        }
        for (i, fp) in fields.unwrap_or_default().iter().enumerate() {
            // A bare name takes the field it names, else the one at its
            // place, as in a variant pattern.
            let by_name = |n: &str| infos.iter().find(|f| f.name == n);
            let info = match (&fp.name, &fp.pattern.kind) {
                (Some(n), _) => by_name(&n.name),
                (None, PatternKind::Binding(b)) => by_name(&b.name).or(infos.get(i)),
                (None, _) => infos.get(i),
            };
            let Some(info) = info else {
                let message = match &fp.name {
                    Some(n) => format!("`{}` has no field `{}`", name.name, n.name),
                    None => format!("`{}` has no such field", name.name),
                };
                let at = fp.name.as_ref().map_or(fp.pattern.span, |n| n.span);
                self.error(at, message);
                self.bind(&fp.pattern, &Ty::Error);
                continue;
            };
            // A field private to another module can't be named (10.1),
            // by its name or by its place.
            let named = fp.name.clone().unwrap_or_else(|| Ident {
                name: info.name.clone(),
                span: fp.pattern.span,
            });
            self.check_field_access(def, info, &named);
            let t = self.inf.subst(&info.ty, &subst);
            self.bind(&fp.pattern, &t);
            // Taken from a secret field, a name holds a secret (spec 8.7).
            if info.secret {
                for d in self.bound(&fp.pattern) {
                    self.secret_defs.insert(d);
                }
            }
        }
    }
}
