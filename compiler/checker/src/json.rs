//! `Json` in the checker (spec 2.12): literals that become a `Json` where
//! one is expected, and which types can be encoded as JSON.

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
    /// or any value whose type can be encoded.
    fn json_item(&mut self, e: &Expr, json: &Ty) {
        if json_literal_kind(e) {
            self.infer(e, Some(json));
            return;
        }
        let t = self.value(e, None);
        if !self.is_json(&t)
            && let Some(why) = self.not_encodable(&t)
        {
            self.error(e.span, why);
        }
    }

    /// Why values of type `t` can't be encoded as JSON, or `None` when
    /// they can (spec 2.12).
    pub(crate) fn not_encodable(&self, t: &Ty) -> Option<String> {
        self.encodable_in(t, &mut Vec::new(), false)
    }

    /// Why values of type `t` can't be decoded from JSON, or `None`: the
    /// same types as encoding, and records with `secret` fields too.
    pub(crate) fn not_decodable(&self, t: &Ty) -> Option<String> {
        self.encodable_in(t, &mut Vec::new(), true)
            .map(|why| why.replace("written as JSON", "read from JSON"))
    }

    fn encodable_in(&self, t: &Ty, seen: &mut Vec<DefId>, decoding: bool) -> Option<String> {
        let t = self.inf.resolve(t);
        match &t {
            Ty::Prim(_) | Ty::Param(_) | Ty::Error | Ty::Var(_) => None,
            Ty::List(x) | Ty::Set(x) | Ty::Optional(x) => self.encodable_in(x, seen, decoding),
            Ty::Tuple(ts) => ts.iter().find_map(|t| self.encodable_in(t, seen, decoding)),
            Ty::Map(k, v) => {
                if self.inf.resolve(k) != STRING {
                    let shown = self.show(&t);
                    return Some(format!(
                        "`{shown}` can't be written as JSON: only maps with `String` keys become objects"
                    ));
                }
                self.encodable_in(v, seen, decoding)
            }
            Ty::Adt(def, _) if seen.contains(def) || self.is_json(&t) => None,
            Ty::Adt(def, _) => {
                seen.push(*def);
                let name = self.res.defs[*def].name.clone();
                let fields = match self.types.get(def).map(|i| &i.shape) {
                    Some(Shape::Record(fields)) => fields.clone(),
                    Some(Shape::Union(variants)) => {
                        variants.iter().flat_map(|v| v.fields.clone()).collect()
                    }
                    _ => return Some(format!("`{name}` can't be written as JSON")),
                };
                if let Some(f) = fields.iter().find(|f| f.secret && !decoding) {
                    return Some(format!(
                        "`{name}` has the secret field `{}`, so it can't be written as JSON (spec 2.11)",
                        f.name
                    ));
                }
                fields
                    .iter()
                    .find_map(|f| self.encodable_in(&f.ty, seen, decoding))
            }
            other => Some(format!("`{}` can't be written as JSON", self.show(other))),
        }
    }
}
