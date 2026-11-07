//! `kavach proto` (spec 14.14): the `proto3` file for a program's record
//! and union types, with the wire conventions `std.grpc` uses, so
//! `protoc` in any language generates matching code.

use kavach_ast::{ItemKind, TypeBody, TypeDecl, Variant};
use kavach_driver::Compiled;
use kavach_driver::{Prim, Ty};
use std::fmt::Write;
use std::process::ExitCode;

pub(crate) fn proto(path: &str) -> ExitCode {
    let c = match crate::load(path) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let mut out = String::new();
    let _ = writeln!(out, "// Written by `kavach proto` from {path}.");
    let _ = writeln!(out, "syntax = \"proto3\";\n");
    for item in c.own_items() {
        let ItemKind::Type(t) = &item.kind else {
            continue;
        };
        if !t.generics.is_empty() {
            let _ = writeln!(out, "// `{}` is generic: no .proto for it.\n", t.name.name);
            continue;
        }
        match &t.body {
            TypeBody::Record(r) => {
                let def = c
                    .resolution
                    .defs
                    .iter()
                    .position(|d| d.span == Some(t.name.span));
                let Some(def) = def else { continue };
                let fields = message_fields(&c, def, &r.fields, "  ");
                let _ = writeln!(out, "message {} {{\n{fields}}}\n", t.name.name);
            }
            TypeBody::Union(vs) => {
                let _ = writeln!(out, "{}", union_message(&c, t, vs));
            }
            TypeBody::Alias(_) => {}
        }
    }
    print!("{out}");
    ExitCode::SUCCESS
}

/// The fields of a message, one `type name = n;` line each.
fn message_fields(c: &Compiled, def: usize, fields: &[kavach_ast::Field], pad: &str) -> String {
    let mut out = String::new();
    for (i, f) in fields.iter().enumerate() {
        let line = match c
            .field_types
            .get(&(def, f.name.name.clone()))
            .and_then(|t| proto_type(c, t))
        {
            Some(t) => format!("{pad}{t} {} = {};", f.name.name, i + 1),
            None => format!("{pad}// `{}` has a type .proto can't say yet.", f.name.name),
        };
        let _ = writeln!(out, "{line}");
    }
    out
}

/// A union as spec 14.14 sends it: a wrapper message holding a oneof,
/// fieldless variants first, each variant a nested message.
fn union_message(c: &Compiled, t: &TypeDecl, vs: &[Variant]) -> String {
    let plain: Vec<&Variant> = vs.iter().filter(|v| v.fields.is_empty()).collect();
    let fielded: Vec<&Variant> = vs.iter().filter(|v| !v.fields.is_empty()).collect();
    let mut out = String::new();
    let _ = writeln!(out, "message {} {{", t.name.name);
    let _ = writeln!(out, "  oneof value {{");
    for (k, v) in plain.iter().chain(&fielded).enumerate() {
        let _ = writeln!(
            out,
            "    {} {} = {};",
            v.name.name,
            v.name.name.to_lowercase(),
            k + 1
        );
    }
    let _ = writeln!(out, "  }}");
    for v in plain.iter().chain(&fielded) {
        if v.fields.is_empty() {
            let _ = writeln!(out, "  message {} {{}}", v.name.name);
            continue;
        }
        let def = c
            .resolution
            .defs
            .iter()
            .position(|d| d.span == Some(v.name.span));
        let Some(def) = def else { continue };
        let fields = message_fields(c, def, &v.fields, "    ");
        let _ = writeln!(out, "  message {} {{\n{fields}  }}", v.name.name);
    }
    let _ = writeln!(out, "}}");
    out
}

/// The `proto3` type of `t`, or `None` when .proto can't say it.
fn proto_type(c: &Compiled, t: &Ty) -> Option<String> {
    Some(match t {
        Ty::Prim(Prim::Bool) => "bool".into(),
        Ty::Prim(Prim::String | Prim::Char) => "string".into(),
        Ty::Prim(Prim::Float | Prim::Float32 | Prim::Decimal) => "double".into(),
        Ty::Prim(Prim::UInt64) => "uint64".into(),
        Ty::Prim(p) if p.is_int() => "int64".into(),
        Ty::Optional(inner) => format!("optional {}", proto_type(c, inner)?),
        Ty::Map(k, v) if **k == Ty::Prim(Prim::String) => {
            format!("map<string, {}>", proto_type(c, v)?)
        }
        Ty::List(item) => format!("repeated {}", proto_type(c, item)?),
        Ty::Adt(def, args) if args.is_empty() => c.resolution.defs[*def].name.clone(),
        _ => return None,
    })
}
