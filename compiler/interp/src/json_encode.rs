//! Values as JSON (spec 2.12, decision 0132): every value has a form.
//! `json.of` and `json.stringify` hide `secret` fields and may write big
//! numbers as text; the agent journal and other processes keep values
//! whole, as state (8.5, 9.8).

use crate::value::{Field, Value};
use kavach_runtime_core::json::{self, Tree};

/// How a value is written.
#[derive(Clone, Copy, Default)]
pub(crate) struct Encode {
    /// A `secret` field as `"<secret>"`, as `print` shows it.
    pub(crate) hide_secrets: bool,
    /// `BigInt` and `Decimal` as text, for readers that round numbers
    /// past 2^53.
    pub(crate) big_as_text: bool,
}

/// `v` written whole: secrets kept, big numbers as numbers.
pub(crate) fn json_encode(v: &Value<'_>) -> Tree {
    encode(v, Encode::default())
}

/// `v` as `json.of` and `json.stringify` write it.
pub(crate) fn encode(v: &Value<'_>, o: Encode) -> Tree {
    let object = |fields: &[Field<'_>]| {
        Tree::Object(
            fields
                .iter()
                .map(|f| {
                    let value = if f.secret && o.hide_secrets {
                        Tree::Text("<secret>".into())
                    } else {
                        encode(&f.value, o)
                    };
                    (f.name.to_string(), value)
                })
                .collect(),
        )
    };
    let big = |text: String| {
        if o.big_as_text {
            Tree::Text(text)
        } else {
            Tree::Number(text)
        }
    };
    match v {
        Value::Variant { union, .. } if crate::json::is_json(*union) => json_tree(v),
        Value::Int(n, _) => i64::try_from(*n).map_or(Tree::Number(n.to_string()), Tree::Int),
        // Negative zero is `0.0`, as JavaScript writes it (2.12); JSON
        // text read in keeps its own.
        Value::Float(f) => Tree::Float(*f + 0.0),
        Value::Float32(f) => Tree::Float(f64::from(*f) + 0.0),
        // With all their digits (decision 0071).
        Value::Decimal(d) => big(d.to_string()),
        Value::BigDecimal(d) => big(d.to_string()),
        Value::BigInt(n) => big(n.to_string()),
        Value::Bool(b) => Tree::Bool(*b),
        Value::Char(c) => Tree::Text(c.to_string()),
        Value::Str(s) => Tree::Text(s.to_string()),
        // Base64 text (spec 2.12).
        Value::Bytes(b) => Tree::Text(kavach_runtime_core::base64::to_json(b)),
        Value::List(items) => Tree::Array(items.iter().map(|x| encode(x, o)).collect()),
        Value::Tuple(items) => Tree::Array(items.iter().map(|x| encode(x, o)).collect()),
        Value::Set(set) => Tree::Array(set.entries.iter().map(|(k, _)| encode(k, o)).collect()),
        Value::Map(map) => Tree::Object(
            map.entries
                .iter()
                .map(|(k, v)| (key_text(k, o), encode(v, o)))
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
        Value::None | Value::Unit => Tree::Null,
        Value::Shared(inner) => encode(&inner.borrow(), o),
        Value::Weak(w) => w
            .upgrade()
            .map_or(Tree::Null, |inner| encode(&inner.borrow(), o)),
        // No JSON meaning: the printed form, one way only (decision 0132).
        other => Tree::Text(other.display()),
    }
}

/// A dict key as JSON text (decision 0132): text as itself, numbers,
/// `Bool`, `Char` and a variant without fields as they print, any other
/// key as its own compact JSON.
pub(crate) fn key_text(k: &Value<'_>, o: Encode) -> String {
    match k {
        Value::Str(s) => s.to_string(),
        Value::Variant {
            union,
            name,
            fields,
            ..
        } if fields.is_empty() && !crate::json::is_json(*union) => name.to_string(),
        Value::Int(..)
        | Value::Float(_)
        | Value::Float32(_)
        | Value::Decimal(_)
        | Value::BigDecimal(_)
        | Value::BigInt(_)
        | Value::Bool(_)
        | Value::Char(_) => k.display(),
        _ => json::write(&encode(k, o), 0),
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
