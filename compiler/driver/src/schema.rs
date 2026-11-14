//! The JSON Schema `ask<T>` sends the model (spec 8.4), built from `T`,
//! with the doc comments of types and fields as descriptions. Structured
//! outputs need an object at the top, with every object closed
//! (`additionalProperties: false`), so a `T` that is not a record is
//! wrapped as `{"value": …}`.
//! The interpreter and native code both send this schema.

use crate::Compiled;
use kavach_ast::{Field, ItemKind, TypeBody, TypeDecl};
use kavach_checker::{Prim, Ty};
use kavach_resolver::DefId;
use kavach_runtime_core::json::Tree;
use kavach_runtime_core::json::dates::Kind;

/// The schema for answers of type `t`, and whether the answer is wrapped
/// in `{"value": …}`; `None` for a type a schema can't express (a map, a
/// tuple, a function).
pub fn ask_schema(c: &Compiled, t: &Ty) -> Option<(Tree, bool)> {
    let s = Schemas { c };
    let inner = s.schema(t)?;
    // A model answers closed objects only, so maps stay out of `ask`,
    // wherever one hides.
    if open_objects(&inner) {
        return None;
    }
    if let Ty::Adt(def, _) = t
        && matches!(s.decl(*def), Some(d) if matches!(d.body, TypeBody::Record(_)))
    {
        return Some((inner, false));
    }
    Some((
        closed(vec![("value".into(), inner)], vec![text("value")]),
        true,
    ))
}

/// The JSON Schema of `t` (`json.schema<T>()`, spec 2.12), or `None`
/// for a type with none (maps, tuples, functions).
pub fn json_schema(c: &Compiled, t: &Ty) -> Option<Tree> {
    Schemas { c }.schema(t)
}

/// The description of the function `arg` names, for `json.describe`
/// (spec 2.12): its name, doc comment, parameters with their schemas, the
/// schemas of its result and error, its `uses` and its examples as text.
pub fn describe_fn(c: &Compiled, arg: &kavach_ast::Expr) -> Option<Tree> {
    let kavach_ast::ExprKind::Name(id) = &arg.kind else {
        return None;
    };
    let def = *c.resolution.uses.get(&id.span.start)?;
    let ty = c.expr_types.get(&(arg.span.start, arg.span.end))?;
    describe(c, def, ty, false)
}

/// The description of function `def` as a value, for `json.describe` on
/// a function value (spec 2.12): `None` for a generic function.
pub fn describe_def(c: &Compiled, def: kavach_resolver::DefId) -> Option<Tree> {
    let span = c.resolution.defs[def].span?;
    describe(c, def, c.expr_types.get(&(span.start, span.end))?, true)
}

/// `any`: a part with no schema (a function, a built-in `Error`) is `{}`,
/// any value, as a function value's description has it; otherwise the
/// description fails, which is a compile error for a named function.
fn describe(c: &Compiled, def: kavach_resolver::DefId, ty: &Ty, any: bool) -> Option<Tree> {
    let Ty::Fn(params, ret, _) = ty else {
        return None;
    };
    let schema = |t: &Ty| match (Schemas { c }).schema(t) {
        None if any => Some(Tree::Object(Vec::new())),
        found => found,
    };
    let span = c.resolution.defs[def].span?;
    let (docs, f) = c.file.items.iter().find_map(|item| match &item.kind {
        ItemKind::Fn(f) if f.name.span == span => Some((&item.docs, f)),
        _ => None,
    })?;
    let mut described = Vec::new();
    for (p, t) in f.params.iter().zip(params) {
        described.push(Tree::Object(vec![
            ("name".into(), text(&p.name.name)),
            ("schema".into(), schema(t)?),
        ]));
    }
    let (ok, err) = match &**ret {
        Ty::Result(ok, err) => (&**ok, Some(schema(err)?)),
        ok => (ok, None),
    };
    let result = match ok {
        Ty::Unit => kind("null"),
        t => schema(t)?,
    };
    let uses = c.uses.get(&def).cloned().unwrap_or_default();
    let examples = f.examples.iter().map(|x| {
        let end = x.fails.as_ref().map_or(x.expr.span.end, |e| e.span.end);
        text(&c.src[x.expr.span.start as usize..end as usize])
    });
    Some(Tree::Object(vec![
        ("name".into(), text(&f.name.name)),
        ("doc".into(), text(&docs.join(" "))),
        ("params".into(), Tree::Array(described)),
        ("result".into(), result),
        ("error".into(), err.unwrap_or(Tree::Null)),
        (
            "capabilities".into(),
            Tree::Array(uses.iter().map(|u| text(u)).collect()),
        ),
        ("examples".into(), Tree::Array(examples.collect())),
    ]))
}

/// The message for a `T` with no schema.
pub const NO_SCHEMA: &str = "`ask` can't answer with this type yet: maps, tuples and functions don't fit the model's closed objects (use a list of records)";

/// Whether the schema holds an object that isn't closed (a map's).
fn open_objects(t: &Tree) -> bool {
    match t {
        Tree::Object(fs) => fs.iter().any(|(k, v)| {
            (k == "additionalProperties" && !matches!(v, Tree::Bool(false))) || open_objects(v)
        }),
        Tree::Array(xs) => xs.iter().any(open_objects),
        _ => false,
    }
}

fn text(s: &str) -> Tree {
    Tree::Text(s.to_string())
}

fn kind(k: &str) -> Tree {
    Tree::Object(vec![("type".into(), text(k))])
}

struct Schemas<'c> {
    c: &'c Compiled,
}

impl<'c> Schemas<'c> {
    /// The declaration of record or union `def`.
    fn decl(&self, def: DefId) -> Option<&'c TypeDecl> {
        self.item(def).map(|(_, t)| t)
    }

    /// The declaration of record or union `def`, with its doc comment.
    fn item(&self, def: DefId) -> Option<(&'c [String], &'c TypeDecl)> {
        let span = self.c.resolution.defs[def].span?;
        self.c.file.items.iter().find_map(|item| match &item.kind {
            ItemKind::Type(t) if t.name.span == span => Some((&item.docs[..], t)),
            _ => None,
        })
    }

    fn schema(&self, t: &Ty) -> Option<Tree> {
        Some(match t {
            Ty::Prim(Prim::Bool) => kind("boolean"),
            Ty::Prim(Prim::String | Prim::Char) => kind("string"),
            Ty::Prim(Prim::Float | Prim::Float32 | Prim::Decimal) => kind("number"),
            Ty::Prim(p) if p.is_int() => kind("integer"),
            Ty::Optional(inner) => Tree::Object(vec![(
                "anyOf".into(),
                Tree::Array(vec![self.schema(inner)?, kind("null")]),
            )]),
            Ty::List(item) => Tree::Object(vec![
                ("type".into(), text("array")),
                ("items".into(), self.schema(item)?),
            ]),
            // A map with text keys: an open object (spec 2.12).
            Ty::Map(k, v) if **k == Ty::Prim(Prim::String) => Tree::Object(vec![
                ("type".into(), text("object")),
                ("additionalProperties".into(), self.schema(v)?),
            ]),
            Ty::Adt(def, _) => self.type_schema(*def)?,
            _ => return None,
        })
    }

    /// A record or union's schema, with its doc comment as the
    /// description.
    fn type_schema(&self, def: DefId) -> Option<Tree> {
        let (docs, decl) = self.item(def)?;
        let mut s = self.shape_schema(def, decl)?;
        if !docs.is_empty()
            && let Tree::Object(parts) = &mut s
        {
            parts.push(("description".into(), text(&docs.join(" "))));
        }
        Some(s)
    }

    /// A record: an object with its fields; a union: one of its variants,
    /// as JSON writes them (spec 2.12): a name, or `{"Name": {fields}}`.
    fn shape_schema(&self, def: DefId, decl: &TypeDecl) -> Option<Tree> {
        let vs = match &decl.body {
            // `std.time`'s types are ISO 8601 text in JSON.
            TypeBody::Record(r) => {
                let names = r.fields.iter().map(|f| f.name.name.as_str());
                if let Some(kind) = Kind::of(&decl.name.name, names) {
                    let (key, value) = kind.schema();
                    return Some(Tree::Object(vec![
                        ("type".into(), text("string")),
                        (key.into(), text(value)),
                    ]));
                }
                return self.object(def, &r.fields);
            }
            TypeBody::Union(vs) => vs,
            // An alias names an anonymous shape; `ask<T>` and JSON take
            // declared types (spec 2.14).
            TypeBody::Alias(_) => return None,
        };
        let plain: Vec<Tree> = vs
            .iter()
            .filter(|v| v.fields.is_empty())
            .map(|v| text(&v.name.name))
            .collect();
        let mut options = Vec::new();
        if !plain.is_empty() {
            options.push(Tree::Object(vec![
                ("type".into(), text("string")),
                ("enum".into(), Tree::Array(plain)),
            ]));
        }
        for v in vs.iter().filter(|v| !v.fields.is_empty()) {
            let vdef = self
                .c
                .resolution
                .defs
                .iter()
                .position(|d| d.span == Some(v.name.span))?;
            let inner = self.object(vdef, &v.fields)?;
            options.push(closed(
                vec![(v.name.name.clone(), inner)],
                vec![text(&v.name.name)],
            ));
        }
        Some(Tree::Object(vec![("anyOf".into(), Tree::Array(options))]))
    }

    /// An object with `fields` (of record or variant `def`); doc comments
    /// become descriptions.
    fn object(&self, def: DefId, fields: &[Field]) -> Option<Tree> {
        let mut properties = Vec::new();
        let mut required = Vec::new();
        for f in fields {
            let name = f.name.name.clone();
            let ty = self.c.field_types.get(&(def, name.clone()))?;
            let mut s = self.schema(ty)?;
            if !f.docs.is_empty()
                && let Tree::Object(parts) = &mut s
            {
                add_description(parts, &f.docs.join(" "));
            }
            if f.default.is_none() && !matches!(ty, Ty::Optional(_)) {
                required.push(text(&name));
            }
            properties.push((name, s));
        }
        Some(closed(properties, required))
    }
}

/// An object schema with these properties and no others.
fn closed(properties: Vec<(String, Tree)>, required: Vec<Tree>) -> Tree {
    Tree::Object(vec![
        ("type".into(), text("object")),
        ("properties".into(), Tree::Object(properties)),
        ("required".into(), Tree::Array(required)),
        ("additionalProperties".into(), Tree::Bool(false)),
    ])
}

/// Gives `parts` the description `doc`, a field's: before the one its
/// type already has (a union's doc), in one `description`, since a JSON
/// object can't name a key twice (N6: native code refused such a schema).
fn add_description(parts: &mut Vec<(String, Tree)>, doc: &str) {
    match parts.iter_mut().find(|(k, _)| k == "description") {
        Some((_, Tree::Text(old))) if old != doc => *old = format!("{doc} {old}"),
        Some(_) => {}
        None => parts.push(("description".into(), text(doc))),
    }
}
