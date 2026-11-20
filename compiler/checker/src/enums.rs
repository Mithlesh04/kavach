// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Enums (spec 2.6): a union whose variants carry no fields. Every union
//! value has `name()`; an enum has `Type.all()` and `Type.parse(text)`,
//! and, when its variants have values (`NotFound = 404`, `Get = "GET"`),
//! `value()` and `Type.from(value)`. A function or method the type declares itself with one
//! of these names is used instead (the callers look for those first).

use crate::builtins::MethodSig;
use crate::checker::{Checker, Shape, VariantInfo};
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, Ident, Span, TypeDecl, VariantValue};
use kavach_resolver::DefId;

impl Checker<'_> {
    /// The variants of union `def`, if it is one.
    fn variants_of(&self, def: DefId) -> Option<&Vec<VariantInfo>> {
        match self.types.get(&def).map(|t| &t.shape) {
            Some(Shape::Union(vs)) => Some(vs),
            _ => None,
        }
    }

    /// The type of the values of variants `vs`: `String` when they are
    /// text, else `Int`.
    fn value_prim(vs: &[VariantInfo]) -> Prim {
        match vs.first().and_then(|v| v.value.as_ref()) {
            Some(VariantValue::Text(_)) => Prim::String,
            _ => Prim::Int,
        }
    }

    /// `Type.all()`, `Type.parse(text)` and `Type.from(value)` on union
    /// `def`; `None` when `name` is none of them.
    pub(crate) fn enum_call(
        &mut self,
        def: DefId,
        name: &Ident,
        args: &[Arg],
        span: Span,
    ) -> Option<Ty> {
        if !matches!(name.name.as_str(), "all" | "parse" | "from") {
            return None;
        }
        let vs = self.variants_of(def)?.clone();
        let (param, ret_list) = match name.name.as_str() {
            "all" => (None, true),
            "parse" => (Some(("text", Prim::String)), false),
            _ => match Self::value_prim(&vs) {
                Prim::String => (Some(("text", Prim::String)), false),
                p => (Some(("n", p)), false),
            },
        };
        let type_name = self.res.defs[def].name.clone();
        let generic = self.types.get(&def).is_some_and(|t| !t.generics.is_empty());
        if generic || vs.iter().any(|v| !v.fields.is_empty()) {
            self.error(
                name.span,
                format!("`{type_name}.{}` is for enums: unions whose variants carry no fields (spec 2.6)", name.name),
            );
        } else if name.name == "from" && vs.iter().any(|v| v.value.is_none()) {
            self.error(
                name.span,
                format!("`{type_name}.from` needs values on the variants: `Ok = 200` or `Get = \"GET\"` (spec 2.6)"),
            );
        }
        let params: Vec<(String, Ty, bool)> = param
            .map(|(n, p)| vec![(n.to_string(), Ty::Prim(p), false)])
            .unwrap_or_default();
        let what = format!("`{}`", name.name);
        self.args(args, &params, span, &what, "argument");
        let this = Ty::Adt(def, Vec::new());
        Some(if ret_list {
            Ty::List(Box::new(this))
        } else {
            Ty::Optional(Box::new(this))
        })
    }

    /// `v.name()` on any union value, and `v.value()` on an enum whose
    /// variants have values.
    pub(crate) fn enum_method(&self, recv: &Ty, name: &str) -> Option<MethodSig> {
        let Ty::Adt(def, _) = recv else { return None };
        let vs = self.variants_of(*def)?;
        let ret = match name {
            "name" => Ty::Prim(Prim::String),
            "value" if vs.iter().all(|v| v.value.is_some()) => Ty::Prim(Self::value_prim(vs)),
            _ => return None,
        };
        Some(MethodSig {
            params: Vec::new(),
            ret,
            named: Vec::new(),
        })
    }

    /// The rules for variant values: only on enums, on every variant or
    /// none, all numbers or all text, and no two the same.
    pub(crate) fn check_enum_values(&mut self, t: &TypeDecl, vs: &[VariantInfo]) {
        let valued = vs.iter().filter(|v| v.value.is_some()).count();
        if valued == 0 {
            return;
        }
        let name = &t.name.name;
        let texts = vs
            .iter()
            .filter(|v| matches!(v.value, Some(VariantValue::Text(_))))
            .count();
        if vs.iter().any(|v| !v.fields.is_empty()) {
            self.error(t.name.span, format!("`{name}` has variants with fields: only an enum's variants have values (spec 2.6)"));
        } else if let Some(bare) = vs.iter().find(|v| v.value.is_none()) {
            self.error(
                t.name.span,
                format!(
                    "`{}` has no value: give every variant of `{name}` a value, or none (spec 2.6)",
                    bare.name
                ),
            );
        } else if texts != 0 && texts != valued {
            self.error(
                t.name.span,
                format!("`{name}` mixes numbers and text: its variants' values are all numbers or all text (spec 2.6)"),
            );
        }
        let mut seen: Vec<&VariantValue> = Vec::new();
        for v in vs {
            if let Some(value) = &v.value {
                if seen.contains(&value) {
                    self.error(
                        t.name.span,
                        format!("two variants of `{name}` have the value {}: each needs its own (spec 2.6)", value.shown()),
                    );
                }
                seen.push(value);
            }
        }
    }
}
