//! Enums (spec 2.6): a union whose variants carry no fields. Every union
//! value has `name()`; an enum has `Type.all()` and `Type.parse(text)`,
//! and, when its variants have numbers (`NotFound = 404`), `value()` and
//! `Type.from(n)`. A function or method the type declares itself with one
//! of these names is used instead (the callers look for those first).

use crate::builtins::MethodSig;
use crate::checker::{Checker, Shape, VariantInfo};
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, Ident, Span, TypeDecl};
use kavach_resolver::DefId;

impl Checker<'_> {
    /// The variants of union `def`, if it is one.
    fn variants_of(&self, def: DefId) -> Option<&Vec<VariantInfo>> {
        match self.types.get(&def).map(|t| &t.shape) {
            Some(Shape::Union(vs)) => Some(vs),
            _ => None,
        }
    }

    /// `Type.all()`, `Type.parse(text)` and `Type.from(n)` on union `def`;
    /// `None` when `name` is none of them.
    pub(crate) fn enum_call(
        &mut self,
        def: DefId,
        name: &Ident,
        args: &[Arg],
        span: Span,
    ) -> Option<Ty> {
        let (param, ret_list) = match name.name.as_str() {
            "all" => (None, true),
            "parse" => (Some(("text", Prim::String)), false),
            "from" => (Some(("n", Prim::Int)), false),
            _ => return None,
        };
        let vs = self.variants_of(def)?.clone();
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
                format!("`{type_name}.from` needs numbers on the variants: `Ok = 200` (spec 2.6)"),
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
    /// variants have numbers.
    pub(crate) fn enum_method(&self, recv: &Ty, name: &str) -> Option<MethodSig> {
        let Ty::Adt(def, _) = recv else { return None };
        let vs = self.variants_of(*def)?;
        let ret = match name {
            "name" => Ty::Prim(Prim::String),
            "value" if vs.iter().all(|v| v.value.is_some()) => Ty::Prim(Prim::Int),
            _ => return None,
        };
        Some(MethodSig {
            params: Vec::new(),
            ret,
            named: Vec::new(),
        })
    }

    /// The rules for variant numbers: only on enums, on every variant or
    /// none, and no two the same.
    pub(crate) fn check_enum_values(&mut self, t: &TypeDecl, vs: &[VariantInfo]) {
        let numbered = vs.iter().filter(|v| v.value.is_some()).count();
        if numbered == 0 {
            return;
        }
        let name = &t.name.name;
        if vs.iter().any(|v| !v.fields.is_empty()) {
            self.error(t.name.span, format!("`{name}` has variants with fields: only an enum's variants have numbers (spec 2.6)"));
        } else if numbered != vs.len() {
            self.error(
                t.name.span,
                format!("give every variant of `{name}` a number, or none (spec 2.6)"),
            );
        }
        let mut seen: Vec<i128> = Vec::new();
        for v in vs {
            if let Some(n) = v.value {
                if seen.contains(&n) {
                    self.error(
                        t.name.span,
                        format!("two variants of `{name}` have the number {n}"),
                    );
                }
                seen.push(n);
            }
        }
    }
}
