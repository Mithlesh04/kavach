//! `json.decode<T>` and `json.decode_value<T>` (spec 2.12): JSON into a
//! typed value, strict about kinds, with defaults and `none` for missing
//! fields. Native code decodes the same way (`rt_json_decode.rs`), with the
//! same messages (`kavach_runtime_core::json::expected`).

use crate::calls::Args;
use crate::interp::Interp;
use crate::map::MapValue;
use crate::value::{Eval, Field, Flow, Value};
use kavach_ast::TypeBody;
use kavach_checker::{Prim, Ty};
use kavach_runtime_core::Decimal;
use kavach_runtime_core::json::{self, Limits, Tree, expected, path_field, path_item};
use std::rc::Rc;

type Decoded<'a> = Result<Value<'a>, Flow<'a>>;

impl<'a> Interp<'a> {
    /// `json.decode<T>(text, limits…)` or `json.decode_value<T>(j)`; `ty` is
    /// the call's type, `T ! JsonError`.
    pub(crate) fn json_decode(&mut self, name: &str, args: Args<'a>, ty: Option<Ty>) -> Eval<'a> {
        let Some(Ty::Result(t, _)) = ty else {
            return Ok(Value::Unit);
        };
        let a = crate::json::arrange(
            args,
            &["text", "max_depth", "max_bytes", "max_fields", "comments"],
        );
        let tree = if name == "decode" {
            let text = match &a[0] {
                Some(Value::Str(s)) => s.clone(),
                _ => Rc::from(""),
            };
            let d = Limits::default();
            let limits = Limits {
                max_depth: crate::json::int_or(&a[1], d.max_depth),
                max_bytes: crate::json::int_or(&a[2], d.max_bytes),
                max_fields: crate::json::int_or(&a[3], d.max_fields),
                comments: matches!(a[4], Some(Value::Bool(true))),
            };
            json::parse(&text, &limits).map_err(|e| Flow::Fail(self.json_error(e)))?
        } else {
            a[0].as_ref().map_or(Tree::Null, crate::json::json_tree)
        };
        self.decode(&tree, &t, "")
    }

    fn mismatch(&self, path: &str, want: &str) -> Flow<'a> {
        let fields = vec![
            ("path", self.str_value(path)),
            ("expected", self.str_value(want)),
        ];
        Flow::Fail(self.builtin_variant("JsonError", "Mismatch", fields))
    }

    pub(crate) fn decode(&mut self, t: &Tree, ty: &Ty, path: &str) -> Decoded<'a> {
        let wrong = |want: &str| Err(self.mismatch(path, want));
        match (ty, t) {
            (Ty::Prim(Prim::Bool), Tree::Bool(b)) => Ok(Value::Bool(*b)),
            (Ty::Prim(Prim::Bool), _) => wrong(expected::BOOL),
            (Ty::Prim(Prim::String), Tree::Text(s)) => Ok(self.str_value(s)),
            (Ty::Prim(Prim::String), _) => wrong(expected::TEXT),
            (Ty::Prim(Prim::Char), Tree::Text(s)) if s.chars().count() == 1 => {
                Ok(Value::Char(s.chars().next().unwrap_or(' ')))
            }
            (Ty::Prim(Prim::Char), _) => wrong(expected::CHAR),
            (Ty::Prim(Prim::Float32), Tree::Int(n)) => Ok(Value::Float32(*n as f32)),
            (Ty::Prim(Prim::Float32), Tree::Float(f)) => Ok(Value::Float32(*f as f32)),
            (Ty::Prim(Prim::Float), Tree::Int(n)) => Ok(Value::Float(*n as f64)),
            (Ty::Prim(Prim::Float), Tree::Float(f)) => Ok(Value::Float(*f)),
            (Ty::Prim(Prim::Decimal), Tree::Int(n)) => {
                Ok(Value::Decimal(Decimal::from_int(*n as i128)))
            }
            (Ty::Prim(Prim::Decimal), Tree::Float(f)) => Ok(Value::Decimal(
                Decimal::parse(&kavach_runtime_core::format_float(*f))
                    .unwrap_or(Decimal::from_int(0)),
            )),
            // A number kept by its digits (decision 0071): a `Float` takes
            // the nearest, a `Decimal` takes it exactly.
            (Ty::Prim(Prim::Float), Tree::Number(text)) => match text.parse::<f64>() {
                Ok(f) if f.is_finite() => Ok(Value::Float(f)),
                _ => wrong(&expected::float("Float")),
            },
            (Ty::Prim(Prim::Float32), Tree::Number(text)) => match text.parse::<f32>() {
                Ok(f) if f.is_finite() => Ok(Value::Float32(f)),
                _ => wrong(&expected::float("Float32")),
            },
            (Ty::Prim(Prim::Decimal), Tree::Number(text)) if json::exponent_fits(text) => {
                match kavach_runtime_core::AnyDecimal::parse(text) {
                    Some(d) => Ok(crate::bigint::decimal_value(d)),
                    None => wrong(expected::NUMBER),
                }
            }
            (Ty::Prim(Prim::Float | Prim::Float32 | Prim::Decimal), _) => wrong(expected::NUMBER),
            // Any whole number, of any size.
            (Ty::Prim(Prim::BigInt), Tree::Int(n)) => Ok(Value::BigInt(Rc::new(
                kavach_runtime_core::BigInt::from(*n),
            ))),
            (Ty::Prim(Prim::BigInt), Tree::Number(text)) => match text.parse() {
                Ok(n) => Ok(Value::BigInt(Rc::new(n))),
                Err(_) => wrong(expected::WHOLE),
            },
            (Ty::Prim(Prim::BigInt), _) => wrong(expected::WHOLE),
            (Ty::Prim(p), Tree::Int(n)) if p.is_int() => match crate::ops::fits(*n as i128, *p) {
                Some(n) => Ok(Value::Int(n, *p)),
                None => wrong(&expected::integer(p.name())),
            },
            // A `Json` value's number, by its digits (`json.decode_value`).
            (Ty::Prim(p), Tree::Number(text)) if p.is_int() => {
                match text
                    .parse::<i128>()
                    .ok()
                    .and_then(|n| crate::ops::fits(n, *p))
                {
                    Some(n) => Ok(Value::Int(n, *p)),
                    None => wrong(&expected::integer(p.name())),
                }
            }
            (Ty::Prim(p), _) if p.is_int() => wrong(&expected::integer(p.name())),
            (Ty::Optional(_), Tree::Null) => Ok(Value::None),
            (Ty::Optional(x), _) => self.decode(t, x, path),
            (Ty::List(x) | Ty::Set(x), Tree::Array(items)) => {
                let mut out = Vec::with_capacity(items.len());
                for (i, item) in items.iter().enumerate() {
                    out.push(self.decode(item, x, &path_item(path, i))?);
                }
                if matches!(ty, Ty::List(_)) {
                    return Ok(Value::List(Rc::new(out)));
                }
                let mut set = MapValue::default();
                for v in out {
                    set.insert(v, Value::Unit);
                }
                Ok(Value::Set(Rc::new(set)))
            }
            (Ty::Tuple(ts), Tree::Array(items)) if ts.len() == items.len() => {
                let mut out = Vec::with_capacity(items.len());
                for (i, (item, t)) in items.iter().zip(ts).enumerate() {
                    out.push(self.decode(item, t, &path_item(path, i))?);
                }
                Ok(Value::Tuple(out))
            }
            (Ty::Tuple(ts), _) => wrong(&expected::items(ts.len())),
            (Ty::List(_) | Ty::Set(_), _) => wrong(expected::ARRAY),
            (Ty::Map(_, v), Tree::Object(fields)) => {
                let mut map = MapValue::default();
                for (k, value) in fields {
                    let value = self.decode(value, v, &path_field(path, k))?;
                    map.insert(self.str_value(k), value);
                }
                Ok(Value::Map(Rc::new(map)))
            }
            (Ty::Map(..), _) => wrong(expected::OBJECT),
            (Ty::Adt(def, _), _) if crate::json::is_json(*def) => Ok(self.json_value(t.clone())),
            (Ty::Adt(def, _), _) => self.decode_type(t, *def, path),
            _ => wrong(expected::OBJECT),
        }
    }

    /// A record or union value of type `def`.
    fn decode_type(&mut self, t: &Tree, def: usize, path: &str) -> Decoded<'a> {
        let name: Rc<str> = Rc::from(self.c.resolution.defs[def].name.as_str());
        if let Some(decl) = self.records.get(&def).copied() {
            let (Tree::Object(given), TypeBody::Record(r)) = (t, &decl.body) else {
                return Err(self.mismatch(path, expected::OBJECT));
            };
            let fields = self.decode_fields(given, &r.fields, def, path)?;
            return Ok(Value::Record {
                def,
                name,
                fields,
                life: None,
            });
        }
        let Some(TypeBody::Union(variants)) = self.unions.get(&def).map(|d| &d.body) else {
            return Err(self.mismatch(path, expected::OBJECT));
        };
        let found = match t {
            Tree::Text(v) => variants
                .iter()
                .find(|x| x.name.name == *v && x.fields.is_empty())
                .map(|x| (x, Vec::new())),
            Tree::Object(one) if one.len() == 1 => {
                let (v, inner) = &one[0];
                match (variants.iter().find(|x| x.name.name == *v), inner) {
                    (Some(x), Tree::Object(given)) => Some((x, given.clone())),
                    _ => None,
                }
            }
            _ => None,
        };
        let Some((variant, given)) = found else {
            return Err(self.mismatch(path, &expected::variant(&name)));
        };
        let at = path_field(path, &variant.name.name);
        let owner = self
            .decl
            .get(&variant.name.span.start)
            .copied()
            .unwrap_or(def);
        let fields = self.decode_fields(&given, &variant.fields, owner, &at)?;
        Ok(Value::Variant {
            union: def,
            name: Rc::from(variant.name.name.as_str()),
            fields,
        })
    }

    /// The fields of a record or variant: each from `given`, or its
    /// default, or `none` when it is optional.
    fn decode_fields(
        &mut self,
        given: &[(String, Tree)],
        declared: &'a [kavach_ast::Field],
        owner: usize,
        path: &str,
    ) -> Result<Vec<Field<'a>>, Flow<'a>> {
        let mut out = Vec::with_capacity(declared.len());
        for f in declared {
            let name = f.name.name.as_str();
            let ty = self.field_ty(owner, name);
            let at = path_field(path, name);
            let value = match (given.iter().find(|(k, _)| k == name), &f.default, &ty) {
                (Some((_, t)), _, Some(ty)) => self.decode(t, ty, &at)?,
                (None, Some(d), _) => self.eval(d)?,
                (None, None, Some(Ty::Optional(_))) => Value::None,
                _ => return Err(self.mismatch(path, &expected::field(name))),
            };
            out.push(Field {
                name: Rc::from(name),
                value,
                secret: f.is_secret,
            });
        }
        Ok(out)
    }

    /// The checked type of field `name` of record or union `def`.
    fn field_ty(&self, def: usize, name: &str) -> Option<Ty> {
        self.c.field_types.get(&(def, name.to_string())).cloned()
    }
}
