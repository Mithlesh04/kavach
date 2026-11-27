// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Values made from outside data only where the code could make them
//! itself (spec 2.12, 10.1): `json.decode<T>`, `json.decode_texts<T>`,
//! `cli.parse<T>` and `ask<T>` build a `T` field by field, so outside
//! `T`'s module they can't build one with a private or `pub(read)`
//! field, such as the key of a sealed value (2.20).

use crate::checker::{Checker, Shape};
use crate::ty::Ty;
use kavach_ast::Span;
use kavach_resolver::DefId;

impl Checker<'_> {
    /// Reports when `call`, at `span`, would make a value of a type in
    /// `t` that only its own module may make.
    pub(crate) fn check_decode_access(&mut self, t: &Ty, span: Span, call: &str) {
        if let Some(why) = self.made_elsewhere(t, span, &mut Vec::new()) {
            self.error(span, format!("{call} can't make {why}"));
        }
    }

    /// Why a value of `t`, or of a type inside it, can't be made at `at`.
    fn made_elsewhere(&self, t: &Ty, at: Span, seen: &mut Vec<DefId>) -> Option<String> {
        let t = self.inf.resolve(t);
        match &t {
            Ty::List(x) | Ty::Set(x) | Ty::Optional(x) => self.made_elsewhere(x, at, seen),
            Ty::Tuple(ts) => ts.iter().find_map(|x| self.made_elsewhere(x, at, seen)),
            Ty::Map(k, v) => {
                (self.made_elsewhere(k, at, seen)).or_else(|| self.made_elsewhere(v, at, seen))
            }
            Ty::Adt(def, args) if !seen.contains(def) && !self.is_json(&t) => {
                seen.push(*def);
                if let Some(why) = self.private_part(*def, at) {
                    return Some(why);
                }
                let fields = match self.types.get(def).map(|i| &i.shape) {
                    Some(Shape::Record(fields)) => fields.clone(),
                    Some(Shape::Union(variants)) => {
                        variants.iter().flat_map(|v| v.fields.clone()).collect()
                    }
                    _ => Vec::new(),
                };
                (args.iter())
                    .chain(fields.iter().map(|f| &f.ty))
                    .find_map(|x| self.made_elsewhere(x, at, seen))
            }
            _ => None,
        }
    }

    /// Why only `def`'s own module may make one: the type is private
    /// there, or a record field of it is private or `pub(read)`.
    fn private_part(&self, def: DefId, at: Span) -> Option<String> {
        let decl = self.res.defs[def].span?;
        let home = self.res.module_at(decl.start);
        if home == self.res.module_at(at.start) || self.extends_own(def, at) {
            return None;
        }
        let module = &self.res.modules[home].name;
        let name = self.def_name(def);
        if !self.res.public.contains(&def) {
            return Some(format!(
                "a value of `{name}` here: the type is private to module `{module}` (spec 10.1, 2.12)"
            ));
        }
        let Some(Shape::Record(fields)) = self.types.get(&def).map(|i| &i.shape) else {
            return None;
        };
        let field = fields.iter().find(|f| !f.is_pub || f.read_only)?;
        let kind = if field.is_pub {
            "`pub(read)`"
        } else {
            "private"
        };
        Some(format!(
            "a value of `{name}` here: its field `{}` is {kind}, so only module `{module}` can make one (spec 10.1, 2.12)",
            field.name
        ))
    }
}
