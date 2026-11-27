// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Making records and variants (spec 2.4): named arguments, defaults,
//! type arguments and the type's number bounds.

use crate::checker::{Checker, Shape};
use crate::ty::Ty;
use kavach_ast::*;
use kavach_resolver::DefId;

impl Checker<'_> {
    /// `User(name: …)` or `TooLarge(value: …)`: records and variants are
    /// created with named arguments (spec 2.4).
    pub(crate) fn construct(
        &mut self,
        def: DefId,
        variant: Option<DefId>,
        args: &[Arg],
        span: Span,
        name: &str,
        written: &[kavach_ast::TypeExpr],
    ) -> Ty {
        let (instance, subst) = self.written_instance(def, written, span);
        let fields = match (variant, self.types.get(&def).map(|t| t.shape.clone())) {
            (Some(v), _) => self.variant_fields(def, v).unwrap_or_default(),
            (None, Some(Shape::Record(fields))) => fields,
            (None, Some(Shape::Union(_))) => {
                self.error(
                    span,
                    format!("`{name}` is a union: create one of its variants instead"),
                );
                return Ty::Error;
            }
            _ => return Ty::Error,
        };
        if variant.is_none() {
            self.note_drop(def, span);
            self.check_construct_access(def, &fields, span, name);
        }
        // A distinct type is made from its value alone (spec 2.21).
        let distinct = variant.is_none() && self.distinct.contains(&def);
        if distinct && (args.len() != 1 || args[0].name.is_some()) {
            self.error(
                span,
                format!("a distinct type is made from the value it holds alone: write `{name}(value)` (spec 2.21)"),
            );
            for a in args {
                self.value(&a.value, None);
            }
            return instance;
        }
        if !distinct && args.iter().any(|a| a.name.is_none()) && !fields.is_empty() {
            let example = fields
                .iter()
                .map(|f| format!("{}: …", f.name))
                .collect::<Vec<_>>()
                .join(", ");
            self.error(
                span,
                format!("records are created with named arguments: write `{name}({example})`"),
            );
            for a in args {
                self.value(&a.value, None);
            }
            return instance;
        }
        let params: Vec<_> = fields
            .iter()
            .map(|f| (f.name.clone(), self.inf.subst(&f.ty, &subst), f.has_default))
            .collect();
        let secret: Vec<bool> = fields.iter().map(|f| f.secret).collect();
        self.args_with(args, &params, &secret, span, &format!("`{name}`"), "field");
        self.defaults_filled(variant.unwrap_or(def), &fields, args, span);
        // A type's number bounds (spec 2.9), now its fields are known.
        for (g, t) in &subst {
            self.check_number_bound(*g, t, span);
            self.filled_checks.push((*g, t.clone(), span));
        }
        instance
    }
}
