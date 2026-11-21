// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `Json` in the checker (spec 2.12): literals that become a `Json` where
//! one is expected, and which types can be read back from JSON. Every
//! value can be written as JSON (decision 0132).

use crate::checker::{Checker, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::{Expr, ExprKind, StrPart, UnaryOp};
use kavach_resolver::DefId;

const STRING: Ty = Ty::Prim(Prim::String);

/// Whether `e` is written as a literal that can become a `Json`.
pub fn json_literal_kind(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::None
        | ExprKind::List(_)
        | ExprKind::Map(_) => true,
        ExprKind::Unary(UnaryOp::Neg, x) => matches!(x.kind, ExprKind::Int(_) | ExprKind::Float(_)),
        _ => false,
    }
}

impl Checker<'_> {
    /// Whether `t` is the built-in `Json` type.
    pub(crate) fn is_json(&self, t: &Ty) -> bool {
        let json = self.builtin_ty("Json");
        json != Ty::Error && self.inf.resolve(t) == json
    }

    /// `e`, a literal where a `Json` is expected: it becomes one.
    pub(crate) fn json_literal(&mut self, e: &Expr, json: &Ty) -> Ty {
        match &e.kind {
            ExprKind::List(items) => {
                for item in items {
                    self.json_item(item, json);
                }
            }
            ExprKind::Map(pairs) => {
                for (key, value) in pairs {
                    self.check_expect(key, &STRING);
                    self.json_item(value, json);
                }
            }
            ExprKind::Str(parts) => {
                for part in parts {
                    if let StrPart::Expr(x) = part {
                        self.value(x, None);
                    }
                }
            }
            // A number keeps its digits, whatever its size (decision 0071).
            _ => {}
        }
        json.clone()
    }

    /// An item or field value inside a `Json` literal: another literal,
    /// or any value, which `json.of` converts (decision 0132).
    pub(crate) fn json_item(&mut self, e: &Expr, json: &Ty) {
        if json_literal_kind(e) {
            self.infer(e, Some(json));
            return;
        }
        self.value(e, None);
    }

    /// Why values of type `t` can't be read from JSON, or `None` when they
    /// can (spec 2.12): a function, task, channel, file or socket is
    /// written as its printed form, one way only (decision 0132).
    pub(crate) fn not_decodable(&self, t: &Ty) -> Option<String> {
        self.decodable_in(t, &mut Vec::new())
    }

    fn decodable_in(&self, t: &Ty, seen: &mut Vec<DefId>) -> Option<String> {
        let t = self.inf.resolve(t);
        match &t {
            Ty::Prim(_) | Ty::Param(_) | Ty::Error | Ty::Var(_) => None,
            Ty::List(x) | Ty::Set(x) | Ty::Optional(x) => self.decodable_in(x, seen),
            Ty::Tuple(ts) => ts.iter().find_map(|t| self.decodable_in(t, seen)),
            // Keys are JSON text, read back as the key type (decision 0132).
            Ty::Map(k, v) => self
                .decodable_in(k, seen)
                .or_else(|| self.decodable_in(v, seen)),
            Ty::Adt(def, _) if seen.contains(def) || self.is_json(&t) => None,
            Ty::Adt(def, _) => {
                seen.push(*def);
                let name = self.res.defs[*def].name.clone();
                let fields = match self.types.get(def).map(|i| &i.shape) {
                    Some(Shape::Record(fields)) => fields.clone(),
                    Some(Shape::Union(variants)) => {
                        variants.iter().flat_map(|v| v.fields.clone()).collect()
                    }
                    _ => return Some(format!("`{name}` can't be read from JSON (spec 2.12)")),
                };
                fields.iter().find_map(|f| self.decodable_in(&f.ty, seen))
            }
            other => Some(format!(
                "`{}` can't be read from JSON: it is written as its printed form, one way only (spec 2.12)",
                self.show(other)
            )),
        }
    }

    /// Why `t` has no JSON Schema (spec 2.12), looking inside records,
    /// unions, lists, optionals and maps; `None` when it has one.
    pub(crate) fn no_schema(&self, t: &Ty) -> Option<String> {
        self.schema_in(t, &mut Vec::new())
    }

    fn schema_in(&self, t: &Ty, seen: &mut Vec<DefId>) -> Option<String> {
        let t = self.inf.resolve(t);
        match &t {
            Ty::Prim(_) | Ty::Param(_) | Ty::Error | Ty::Var(_) => None,
            Ty::List(x) | Ty::Optional(x) => self.schema_in(x, seen),
            Ty::Map(k, v) if self.inf.resolve(k) == STRING => self.schema_in(v, seen),
            Ty::Anon(fs) => fs.iter().find_map(|(_, x)| self.schema_in(x, seen)),
            Ty::Adt(def, _) if seen.contains(def) || self.is_json(&t) => None,
            Ty::Adt(def, _) => {
                seen.push(*def);
                let fields = match self.types.get(def).map(|i| &i.shape) {
                    Some(Shape::Record(fields)) => fields.clone(),
                    Some(Shape::Union(variants)) => {
                        variants.iter().flat_map(|v| v.fields.clone()).collect()
                    }
                    _ => return None,
                };
                fields.iter().find_map(|f| self.schema_in(&f.ty, seen))
            }
            other => Some(format!(
                "`{}` has no JSON Schema: dicts without `String` keys, sets, tuples and functions have none, even inside a record (spec 2.12)",
                self.show(other)
            )),
        }
    }
}
