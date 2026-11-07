//! Enums (spec 2.6): `Type.all()`, `Type.parse(text)`, `Type.from(n)`,
//! and `v.name()` and `v.value()` on a union value. The checker has made
//! sure each is used only where it may be.

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::TypeBody;
use kavach_checker::Prim;
use kavach_resolver::DefId;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// `Type.all()`, `Type.parse(text)` or `Type.from(n)` on union `def`;
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
        let variant = |v: &kavach_ast::Variant| Value::Variant {
            union: def,
            name: Rc::from(v.name.name.as_str()),
            fields: Vec::new(),
        };
        let found = match (name, arg) {
            ("all", _) => {
                return Some(Ok(Value::List(Rc::new(
                    variants.iter().map(variant).collect(),
                ))));
            }
            ("parse", Some(Value::Str(s))) => variants.iter().find(|v| *v.name.name == **s),
            ("from", Some(Value::Int(n, _))) => variants.iter().find(|v| v.value == Some(*n)),
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
                let n = variants.iter().find(|x| *x.name.name == **variant)?.value?;
                Some(Value::Int(n, Prim::Int))
            }
            _ => None,
        }
    }
}
