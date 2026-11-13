//! `std.json` (spec 2.12): JSON text to and from `Json` values, with the
//! parser and writer that native code uses too
//! (`kavach_runtime_core::json`), so both read and write exactly alike.

use crate::calls::Args;
use crate::interp::Interp;
use crate::map::MapValue;
use crate::value::{Eval, Field, Flow, Value};
use kavach_ast::{Expr, ExprKind, Span, StrPart};
use kavach_checker::{Prim, Ty};
use kavach_resolver::DefId;
use kavach_runtime_core::json::{self, Error, Limits, Tree};
use std::rc::Rc;

thread_local! {
    /// The `Json` union of the program being run, for printing.
    static JSON: std::cell::Cell<Option<DefId>> = const { std::cell::Cell::new(None) };
}

pub(crate) fn set_json_type(def: Option<DefId>) {
    JSON.with(|j| j.set(def));
}

/// Whether union `def` is `Json`, which prints as JSON text.
pub(crate) fn is_json(def: DefId) -> bool {
    JSON.with(|j| j.get() == Some(def))
}

/// The arguments of a call in parameter order: positional ones first,
/// then named ones; `None` where the default is used.
pub(crate) fn arrange<'a>(args: Args<'a>, names: &[&str]) -> Vec<Option<Value<'a>>> {
    let mut out = vec![None; names.len()];
    let mut next = 0;
    for (name, value) in args {
        let at = match name {
            Some(n) => names.iter().position(|p| *p == n),
            None => {
                next += 1;
                Some(next - 1)
            }
        };
        if let Some(slot) = at.and_then(|i| out.get_mut(i)) {
            *slot = Some(value);
        }
    }
    out
}

pub(crate) fn int_or(v: &Option<Value<'_>>, default: usize) -> usize {
    match v {
        Some(Value::Int(n, _)) => usize::try_from(*n).unwrap_or(0),
        _ => default,
    }
}

impl<'a> Interp<'a> {
    /// `json.parse(text)` and `json.stringify(value)`.
    pub(crate) fn json_call(&mut self, name: &str, args: Args<'a>, e: &Expr) -> Eval<'a> {
        let span = e.span;
        match name {
            "decode" | "decode_value" => {
                // Inside a generic function, `T` is the running call's.
                let ty = self.c.expr_types.get(&(e.span.start, e.span.end));
                let ty = ty.map(|t| self.concrete(t));
                self.json_decode(name, args, ty)
            }
            "parse" => {
                let a = arrange(
                    args,
                    &["text", "max_depth", "max_bytes", "max_fields", "comments"],
                );
                let text = match &a[0] {
                    Some(Value::Str(s)) => s.clone(),
                    _ => Rc::from(""),
                };
                let d = Limits::default();
                let limits = Limits {
                    max_depth: int_or(&a[1], d.max_depth),
                    max_bytes: int_or(&a[2], d.max_bytes),
                    max_fields: int_or(&a[3], d.max_fields),
                    comments: matches!(a[4], Some(Value::Bool(true))),
                };
                match json::parse(&text, &limits) {
                    Ok(tree) => Ok(self.json_value(tree)),
                    Err(e) => Err(Flow::Fail(self.json_error(e))),
                }
            }
            "of" => {
                let a = arrange(args, &["value"]);
                if let Some(v) = &a[0] {
                    self.refuse_secret(v, span)?;
                }
                Ok(self.json_value(a[0].as_ref().map_or(Tree::Null, json_encode)))
            }
            // The type's shape (spec 2.12); inside a generic function,
            // `T` is the running call's.
            "schema" => {
                let written = match &e.kind {
                    kavach_ast::ExprKind::Call { generics, .. } => generics.first(),
                    _ => None,
                };
                let ty = written
                    .and_then(|t| self.ty_of(t))
                    .map(|t| self.concrete(&t));
                let tree = ty.and_then(|t| kavach_driver::json_schema(self.c, &t));
                let Some(tree) = tree else {
                    return Err(self.panic("this type has no JSON Schema (spec 2.12)", span));
                };
                Ok(self.json_value(tree))
            }
            // A function described (spec 2.12): `described` in
            // `std/json.kav` reads the compiler's description.
            "describe" => {
                let arg = match &e.kind {
                    ExprKind::Call { args, .. } => args.first(),
                    _ => None,
                };
                let tree = arg.and_then(|a| kavach_driver::describe_fn(self.c, &a.value));
                let reader = self.c.file.items.iter().find_map(|item| match &item.kind {
                    kavach_ast::ItemKind::Fn(f) if f.name.name == "described" => {
                        let m = self.c.resolution.module_at(item.span.start);
                        (self.c.resolution.modules[m].name == "std.json").then_some(f)
                    }
                    _ => None,
                });
                let (Some(tree), Some(reader)) = (tree, reader) else {
                    return Err(self.panic("this function can't be described (spec 2.12)", span));
                };
                let description = self.json_value(tree);
                self.call_fn(reader, None, vec![(None, description)], span)
            }
            "stringify" => {
                let a = arrange(args, &["value", "indent"]);
                if let Some(v) = &a[0] {
                    self.refuse_secret(v, span)?;
                }
                let tree = a[0].as_ref().map_or(Tree::Null, json_encode);
                Ok(self.str_value(&json::write(&tree, int_or(&a[1], 0))))
            }
            _ => Err(self.panic(format!("std.json has no function `{name}`"), span)),
        }
    }

    /// Refuses to encode a value holding a `secret` field (spec 2.11):
    /// the static check covers written types, this covers generic ones.
    fn refuse_secret(&self, v: &Value<'a>, span: Span) -> Result<(), Flow<'a>> {
        match find_secret(v) {
            Some((holder, field)) => Err(self.panic(
                format!(
                    "`{holder}.{field}` is `secret`, so it can't be encoded as JSON; use `reveal(…)` on a copy if you really mean to (spec 2.11)"
                ),
                span,
            )),
            None => Ok(()),
        }
    }

    /// Whether `e` was checked as a `Json`: a literal becomes one there.
    pub(crate) fn is_json_typed(&self, e: &Expr) -> bool {
        match self.c.expr_types.get(&(e.span.start, e.span.end)) {
            Some(Ty::Adt(def, _)) => is_json(*def),
            _ => false,
        }
    }

    /// A literal where a `Json` is expected, as a `Json` (spec 2.12).
    pub(crate) fn json_literal(&mut self, e: &'a Expr) -> Eval<'a> {
        // A number literal keeps the digits it is written with (decision
        // 0071): `0.1` is `0.1`, not the nearest `Float`.
        let number = |text: &str, negative: bool| {
            let digits: String = text.chars().filter(|c| *c != '_').collect();
            let digits = if negative {
                format!("-{digits}")
            } else {
                digits
            };
            Value::Str(Rc::from(digits.as_str()))
        };
        let (name, field) = match &e.kind {
            ExprKind::Int(n) => ("Number", number(&n.to_string(), false)),
            ExprKind::Float(t) => ("Number", number(t, false)),
            ExprKind::Unary(_, x) => match &x.kind {
                ExprKind::Int(n) => ("Number", number(&n.to_string(), true)),
                ExprKind::Float(t) => ("Number", number(t, true)),
                _ => return Ok(self.builtin_variant("Json", "Null", vec![])),
            },
            ExprKind::Bool(b) => ("Boolean", Value::Bool(*b)),
            ExprKind::Str(parts) => {
                let mut s = String::new();
                for part in parts {
                    match part {
                        StrPart::Text(t) => s.push_str(t),
                        StrPart::Expr(x) => s.push_str(&self.eval(x)?.display()),
                    }
                }
                ("Text", self.str_value(&s))
            }
            ExprKind::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.json_item(item)?);
                }
                ("Array", Value::List(Rc::new(out)))
            }
            ExprKind::Map(pairs) => {
                let mut map = MapValue::default();
                for (k, v) in pairs {
                    let key = self.eval(k)?;
                    map.insert(key, self.json_item(v)?);
                }
                ("Object", Value::Map(Rc::new(map)))
            }
            _ => return Ok(self.builtin_variant("Json", "Null", vec![])),
        };
        let field_name = match name {
            "Number" => "digits",
            "Array" => "items",
            "Object" => "fields",
            _ => "value",
        };
        Ok(self.builtin_variant("Json", name, vec![(field_name, field)]))
    }

    /// An item or field inside a `Json` literal: another literal, or any
    /// value that can be encoded.
    fn json_item(&mut self, e: &'a Expr) -> Eval<'a> {
        if kavach_checker::json_literal_kind(e) {
            return self.json_literal(e);
        }
        let v = self.eval(e)?;
        Ok(self.json_value(json_encode(&v)))
    }

    /// `j["key"]` and `j[i]`: the `get` and `at` methods of `Json`
    /// (`stdlib/json/json.kav`).
    pub(crate) fn json_index(&mut self, j: Value<'a>, i: Value<'a>, span: Span) -> Eval<'a> {
        let (Value::Variant { union, .. }, name) = (
            &j,
            if matches!(i, Value::Str(_)) {
                "get"
            } else {
                "at"
            },
        ) else {
            return Ok(j);
        };
        match self.methods.get(union).and_then(|m| m.get(name)).copied() {
            Some(f) => self.call_fn(f, Some(j), vec![(None, i)], span),
            None => Err(self.panic(format!("`Json` has no method `{name}`"), span)),
        }
    }

    /// Variant `name` of the built-in union `union` (`Json`, `JsonError`).
    pub(crate) fn builtin_variant(
        &self,
        union: &str,
        name: &str,
        fields: Vec<(&str, Value<'a>)>,
    ) -> Value<'a> {
        Value::Variant {
            union: self.c.resolution.std_type(union).unwrap_or(0),
            name: Rc::from(name),
            fields: fields
                .into_iter()
                .map(|(n, value)| Field {
                    name: Rc::from(n),
                    value,
                    secret: false,
                })
                .collect(),
        }
    }

    pub(crate) fn json_value(&self, tree: Tree) -> Value<'a> {
        let (name, fields) = match tree {
            Tree::Null => ("Null", vec![]),
            Tree::Bool(b) => ("Boolean", vec![("value", Value::Bool(b))]),
            // Every number, by its digits (decision 0071).
            Tree::Int(_) | Tree::Float(_) | Tree::Number(_) => match json::digits(&tree) {
                Some(d) => ("Number", vec![("digits", self.str_value(&d))]),
                None => ("Null", vec![]),
            },
            Tree::Text(s) => ("Text", vec![("value", self.str_value(&s))]),
            Tree::Array(items) => {
                let items = items.into_iter().map(|t| self.json_value(t)).collect();
                ("Array", vec![("items", Value::List(Rc::new(items)))])
            }
            Tree::Object(fields) => {
                let mut map = MapValue::default();
                for (k, v) in fields {
                    map.insert(self.str_value(&k), self.json_value(v));
                }
                ("Object", vec![("fields", Value::Map(Rc::new(map)))])
            }
        };
        self.builtin_variant("Json", name, fields)
    }

    pub(crate) fn json_error(&self, e: Error) -> Value<'a> {
        let int = |n: usize| Value::Int(n as i128, Prim::Int);
        let (name, fields) = match e {
            Error::Syntax {
                line,
                column,
                message,
            } => (
                "Syntax",
                vec![
                    ("line", int(line)),
                    ("column", int(column)),
                    ("message", self.str_value(&message)),
                ],
            ),
            Error::TooDeep(limit) => ("TooDeep", vec![("limit", int(limit))]),
            Error::TooLarge(limit) => ("TooLarge", vec![("limit", int(limit))]),
            Error::DuplicateKey { key, line, column } => (
                "DuplicateKey",
                vec![
                    ("key", self.str_value(&key)),
                    ("line", int(line)),
                    ("column", int(column)),
                ],
            ),
        };
        self.builtin_variant("JsonError", name, fields)
    }
}

/// Any value as JSON (spec 2.12): what `json.of` gives. A `Json` value
/// stays as it is.
/// A `secret` field anywhere inside `v`: who holds it and its name.
pub(crate) fn find_secret(v: &Value<'_>) -> Option<(String, String)> {
    match v {
        Value::Record { name, fields, .. } | Value::Variant { name, fields, .. } => {
            fields.iter().find_map(|f| {
                if f.secret {
                    Some((name.to_string(), f.name.to_string()))
                } else {
                    find_secret(&f.value)
                }
            })
        }
        Value::List(items) => items.iter().find_map(find_secret),
        Value::Tuple(items) => items.iter().find_map(find_secret),
        Value::Set(set) => set.entries.iter().find_map(|(k, _)| find_secret(k)),
        Value::Map(map) => map.entries.iter().find_map(|(_, v)| find_secret(v)),
        Value::Shared(inner) => find_secret(&inner.borrow()),
        _ => None,
    }
}

pub(crate) fn json_encode(v: &Value<'_>) -> Tree {
    let object = |fields: &[Field<'_>]| {
        Tree::Object(
            fields
                .iter()
                .map(|f| (f.name.to_string(), json_encode(&f.value)))
                .collect(),
        )
    };
    match v {
        Value::Variant { union, .. } if is_json(*union) => json_tree(v),
        Value::Int(n, _) => i64::try_from(*n).map_or(Tree::Number(n.to_string()), Tree::Int),
        Value::Float(f) => Tree::Float(*f),
        Value::Float32(f) => Tree::Float(f64::from(*f)),
        // With all their digits (decision 0071).
        Value::Decimal(d) => Tree::Number(d.to_string()),
        Value::BigDecimal(d) => Tree::Number(d.to_string()),
        Value::BigInt(n) => Tree::Number(n.to_string()),
        Value::Bool(b) => Tree::Bool(*b),
        Value::Char(c) => Tree::Text(c.to_string()),
        Value::Str(s) => Tree::Text(s.to_string()),
        Value::List(items) => Tree::Array(items.iter().map(json_encode).collect()),
        Value::Tuple(items) => Tree::Array(items.iter().map(json_encode).collect()),
        Value::Set(set) => Tree::Array(set.entries.iter().map(|(k, _)| json_encode(k)).collect()),
        Value::Map(map) => Tree::Object(
            map.entries
                .iter()
                .map(|(k, v)| (k.display(), json_encode(v)))
                .collect(),
        ),
        Value::Record { name, fields, .. } => match date_numbers(name, fields) {
            Some((kind, n)) => Tree::Text(json::dates::write(kind, &n)),
            None => object(fields),
        },
        Value::Variant { name, fields, .. } if fields.is_empty() => Tree::Text(name.to_string()),
        Value::Variant { name, fields, .. } => {
            Tree::Object(vec![(name.to_string(), object(fields))])
        }
        _ => Tree::Null,
    }
}

/// A `std.time` `Date`, `Time` or `DateTime` (spec 2.12): which, and its
/// numbers in order, for its ISO 8601 text.
fn date_numbers(name: &str, fields: &[Field<'_>]) -> Option<(json::dates::Kind, Vec<i64>)> {
    let kind = json::dates::Kind::of(name, fields.iter().map(|f| &*f.name))?;
    let mut n = Vec::new();
    for f in fields {
        match &f.value {
            Value::Int(v, _) => n.push(i64::try_from(*v).ok()?),
            Value::Record { name, fields, .. } => n.extend(date_numbers(name, fields)?.1),
            _ => return None,
        }
    }
    Some((kind, n))
}

/// A `Json` value as a tree for the writer.
pub(crate) fn json_tree(v: &Value<'_>) -> Tree {
    let Value::Variant { name, fields, .. } = v else {
        return Tree::Null;
    };
    let field = fields.first().map(|f| &f.value);
    match (&**name, field) {
        ("Boolean", Some(Value::Bool(b))) => Tree::Bool(*b),
        // Digits that aren't a JSON number are written `null`, as `NaN` is.
        ("Number", Some(Value::Str(s))) if json::is_number(s) => Tree::Number(s.to_string()),
        ("Text", Some(Value::Str(s))) => Tree::Text(s.to_string()),
        ("Array", Some(Value::List(items))) => Tree::Array(items.iter().map(json_tree).collect()),
        ("Object", Some(Value::Map(map))) => Tree::Object(
            map.entries
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        Value::Str(s) => s.to_string(),
                        _ => String::new(),
                    };
                    (key, json_tree(v))
                })
                .collect(),
        ),
        _ => Tree::Null,
    }
}
