// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Enums (spec 2.6): `Type.all()`, `Type.parse(text)`, `Type.from(value)`,
//! and `v.name()` and `v.value()` on a union value. The checker has made
//! sure each is used only where it may be.

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::{TypeBody, VariantValue};
use kavach_checker::Prim;
use kavach_resolver::DefId;
use kavach_runtime_core::json::Tree;
use std::collections::HashMap;
use std::rc::Rc;

/// The program's unions, by definition.
pub(crate) type Unions<'u> = HashMap<DefId, &'u kavach_ast::TypeDecl>;

/// The value variant `name` of union `def` carries, if any (spec 2.6).
pub(crate) fn carried<'u>(unions: &Unions<'u>, def: DefId, name: &str) -> Option<&'u VariantValue> {
    let TypeBody::Union(variants) = &unions.get(&def)?.body else {
        return None;
    };
    variants
        .iter()
        .find(|v| v.name.name == name)?
        .value
        .as_ref()
}

/// A carried value as JSON writes it (spec 2.12).
pub(crate) fn carried_tree(value: &VariantValue) -> Tree {
    match value {
        VariantValue::Int(n) => i64::try_from(*n).map_or(Tree::Number(n.to_string()), Tree::Int),
        VariantValue::Text(t) => Tree::Text(t.clone()),
    }
}

/// Whether JSON value `t` is the carried value `value`.
pub(crate) fn is_carried(t: &Tree, value: &VariantValue) -> bool {
    match (value, t) {
        (VariantValue::Text(x), Tree::Text(y)) => x == y,
        (VariantValue::Int(n), Tree::Int(m)) => *n == i128::from(*m),
        (VariantValue::Int(n), Tree::Number(d)) => d.parse::<i128>().ok() == Some(*n),
        _ => false,
    }
}

impl<'a> Interp<'a> {
    /// `Type.all()`, `Type.parse(text)` or `Type.from(value)` on union `def`;
    /// `None` when `name` is none of them.
    pub(crate) fn enum_call(
        &mut self,
        def: DefId,
        name: &str,
        arg: Option<&Value<'a>>,
    ) -> Option<Eval<'a>> {
        let decl = self.unions.get(&def)?;
        let TypeBody::Union(variants) = &decl.body else {
            return None;
        };
        // A variant of that name is a variant.
        if variants.iter().any(|v| *v.name.name == *name) {
            return None;
        }
        let variant = |v: &kavach_ast::Variant| {
            let name = Rc::from(v.name.name.as_str());
            let life = self.variant_life(def, &name, &[]);
            Value::Variant {
                union: def,
                name,
                fields: Vec::new(),
                life,
            }
        };
        let found = match (name, arg) {
            ("all", _) => {
                return Some(Ok(Value::List(Rc::new(
                    variants.iter().map(variant).collect(),
                ))));
            }
            ("parse", Some(Value::Str(s))) => variants.iter().find(|v| *v.name.name == **s),
            ("from", Some(Value::Int(n, _))) => variants
                .iter()
                .find(|v| v.value == Some(VariantValue::Int(*n))),
            ("from", Some(Value::Str(s))) => variants
                .iter()
                .find(|v| matches!(&v.value, Some(VariantValue::Text(t)) if *t == **s)),
            _ => return None,
        };
        Some(Ok(found.map_or(Value::None, variant)))
    }

    /// `v.name()` and `v.value()` on a union value.
    pub(crate) fn enum_method(&self, v: &Value<'a>, name: &str) -> Option<Value<'a>> {
        let Value::Variant {
            union,
            name: variant,
            ..
        } = v
        else {
            return None;
        };
        match name {
            "name" => Some(self.str_value(variant)),
            "value" => {
                let decl = self.unions.get(union)?;
                let TypeBody::Union(variants) = &decl.body else {
                    return None;
                };
                match variants
                    .iter()
                    .find(|x| *x.name.name == **variant)?
                    .value
                    .as_ref()?
                {
                    VariantValue::Int(n) => Some(Value::Int(*n, Prim::Int)),
                    VariantValue::Text(t) => Some(self.str_value(t)),
                }
            }
            _ => None,
        }
    }
}
