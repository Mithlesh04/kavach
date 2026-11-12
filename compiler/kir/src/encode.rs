//! The shape table as text, so an executable can carry it to the runtime
//! (`kavach build`). One shape per line:
//!
//! ```text
//! str | dec | err | regex | list T | map K V | set T | shared T | opt T | tup T T…
//! fn R P P… (a function value) | clo T T… (a closure's captures)
//! rec Name field:T field:T:s…          (s: secret)
//! union Name Variant=field:T,field:T…
//! ```
//!
//! Types are `i8`…`u64`, `f64`, `f32`, `bool`, `char`, `unit`, or `r<id>` for a
//! heap type. Names are identifiers, so they never hold the separators.

use crate::ir::{FieldShape, Shape, Type, VariantShape};

const SCALARS: [(Type, &str); 13] = [
    (Type::I8, "i8"),
    (Type::I16, "i16"),
    (Type::I32, "i32"),
    (Type::I64, "i64"),
    (Type::U8, "u8"),
    (Type::U16, "u16"),
    (Type::U32, "u32"),
    (Type::U64, "u64"),
    (Type::F64, "f64"),
    (Type::F32, "f32"),
    (Type::Bool, "bool"),
    (Type::Char, "char"),
    (Type::Unit, "unit"),
];

fn ty(t: Type) -> String {
    match t {
        Type::Ref(id) => format!("r{id}"),
        t => SCALARS
            .iter()
            .find(|(s, _)| *s == t)
            .map_or("unit", |(_, n)| n)
            .to_string(),
    }
}

fn parse_ty(text: &str) -> Option<Type> {
    if let Some(id) = text.strip_prefix('r') {
        return id.parse().ok().map(Type::Ref);
    }
    SCALARS.iter().find(|(_, n)| *n == text).map(|(t, _)| *t)
}

fn field(f: &FieldShape) -> String {
    let secret = if f.secret { ":s" } else { "" };
    format!("{}:{}{secret}", f.name, ty(f.ty))
}

fn parse_field(text: &str) -> Option<FieldShape> {
    let mut parts = text.split(':');
    let name = parts.next()?.to_string();
    let ty = parse_ty(parts.next()?)?;
    Some(FieldShape {
        name,
        ty,
        secret: parts.next() == Some("s"),
    })
}

pub fn encode_shapes(shapes: &[Shape]) -> String {
    let line = |s: &Shape| match s {
        Shape::Str => "str".to_string(),
        Shape::Bytes => "bytes".to_string(),
        Shape::Socket => "socket".to_string(),
        Shape::Reader(t) => format!("reader {}", ty(*t)),
        Shape::Decimal => "dec".to_string(),
        Shape::BigInt => "bigint".to_string(),
        Shape::Error => "err".to_string(),
        Shape::Regex => "regex".to_string(),
        Shape::List(t) => format!("list {}", ty(*t)),
        Shape::Map(k, v) => format!("map {} {}", ty(*k), ty(*v)),
        Shape::Set(t) => format!("set {}", ty(*t)),
        Shape::Shared(t) => format!("shared {}", ty(*t)),
        Shape::Task(t) => format!("task {}", ty(*t)),
        Shape::Channel(t) => format!("chan {}", ty(*t)),
        Shape::Func(params, ret, err) => {
            let params: Vec<String> = params.iter().map(|t| ty(*t)).collect();
            let word = match err {
                Some(e) => format!("fnf {}", ty(*e)),
                None => "fn".to_string(),
            };
            format!("{word} {} {}", ty(*ret), params.join(" "))
                .trim_end()
                .to_string()
        }
        Shape::Closure(ts) => {
            let items: Vec<String> = ts.iter().map(|t| ty(*t)).collect();
            format!("clo {}", items.join(" ")).trim_end().to_string()
        }
        Shape::Optional(t) => format!("opt {}", ty(*t)),
        Shape::Tuple(ts) => {
            let items: Vec<String> = ts.iter().map(|t| ty(*t)).collect();
            format!("tup {}", items.join(" ")).trim_end().to_string()
        }
        Shape::Record { name, fields, drop } => {
            let fields: Vec<String> = fields.iter().map(field).collect();
            let tag = if *drop { "drec" } else { "rec" };
            format!("{tag} {name} {}", fields.join(" "))
                .trim_end()
                .to_string()
        }
        Shape::Iface { name, methods } => {
            let ms: Vec<String> = methods
                .iter()
                .map(|(n, t)| format!("{n}:{}", ty(*t)))
                .collect();
            format!("iface {name} {}", ms.join(" "))
                .trim_end()
                .to_string()
        }
        Shape::Union { name, variants } => {
            let vs: Vec<String> = variants
                .iter()
                .map(|v| {
                    let fields: Vec<String> = v.fields.iter().map(field).collect();
                    format!("{}={}", v.name, fields.join(","))
                })
                .collect();
            format!("union {name} {}", vs.join(" "))
        }
    };
    shapes.iter().map(line).collect::<Vec<_>>().join("\n")
}

pub fn decode_shapes(text: &str) -> Option<Vec<Shape>> {
    text.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let mut words = l.split(' ');
            let kind = words.next()?;
            let rest: Vec<&str> = words.collect();
            Some(match kind {
                "str" => Shape::Str,
                "bytes" => Shape::Bytes,
                "socket" => Shape::Socket,
                "reader" => Shape::Reader(parse_ty(rest.first()?)?),
                "dec" => Shape::Decimal,
                "bigint" => Shape::BigInt,
                "err" => Shape::Error,
                "regex" => Shape::Regex,
                "list" => Shape::List(parse_ty(rest.first()?)?),
                "map" => Shape::Map(parse_ty(rest.first()?)?, parse_ty(rest.get(1)?)?),
                "set" => Shape::Set(parse_ty(rest.first()?)?),
                "shared" => Shape::Shared(parse_ty(rest.first()?)?),
                "task" => Shape::Task(parse_ty(rest.first()?)?),
                "chan" => Shape::Channel(parse_ty(rest.first()?)?),
                "fn" => Shape::Func(
                    rest[1..]
                        .iter()
                        .map(|t| parse_ty(t))
                        .collect::<Option<_>>()?,
                    parse_ty(rest.first()?)?,
                    None,
                ),
                "fnf" => Shape::Func(
                    rest[2..]
                        .iter()
                        .map(|t| parse_ty(t))
                        .collect::<Option<_>>()?,
                    parse_ty(rest.get(1)?)?,
                    Some(parse_ty(rest.first()?)?),
                ),
                "clo" => Shape::Closure(rest.iter().map(|t| parse_ty(t)).collect::<Option<_>>()?),
                "opt" => Shape::Optional(parse_ty(rest.first()?)?),
                "tup" => Shape::Tuple(rest.iter().map(|t| parse_ty(t)).collect::<Option<_>>()?),
                "rec" | "drec" => Shape::Record {
                    name: rest.first()?.to_string(),
                    fields: rest[1..]
                        .iter()
                        .map(|f| parse_field(f))
                        .collect::<Option<_>>()?,
                    drop: kind == "drec",
                },
                "iface" => Shape::Iface {
                    name: rest.first()?.to_string(),
                    methods: rest[1..]
                        .iter()
                        .map(|m| {
                            let (n, t) = m.split_once(':')?;
                            Some((n.to_string(), parse_ty(t)?))
                        })
                        .collect::<Option<_>>()?,
                },
                "union" => Shape::Union {
                    name: rest.first()?.to_string(),
                    variants: rest[1..]
                        .iter()
                        .map(|v| {
                            let (name, fields) = v.split_once('=')?;
                            Some(VariantShape {
                                name: name.to_string(),
                                fields: fields
                                    .split(',')
                                    .filter(|f| !f.is_empty())
                                    .map(parse_field)
                                    .collect::<Option<_>>()?,
                            })
                        })
                        .collect::<Option<_>>()?,
                },
                _ => return None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_survive_encoding() {
        let f = |name: &str, ty, secret| FieldShape {
            name: name.into(),
            ty,
            secret,
        };
        let shapes = vec![
            Shape::Str,
            Shape::Decimal,
            Shape::BigInt,
            Shape::Error,
            Shape::Regex,
            Shape::List(Type::Ref(0)),
            Shape::Map(Type::Ref(0), Type::I64),
            Shape::Set(Type::Char),
            Shape::Shared(Type::Bool),
            Shape::Task(Type::F64),
            Shape::Channel(Type::I64),
            Shape::Func(vec![Type::I64, Type::Ref(0)], Type::Unit, None),
            Shape::Func(vec![Type::Bool], Type::F64, Some(Type::Ref(0))),
            Shape::Closure(vec![Type::Ref(1)]),
            Shape::Closure(Vec::new()),
            Shape::Optional(Type::Char),
            Shape::Tuple(vec![Type::I8, Type::U64, Type::F64]),
            Shape::Tuple(Vec::new()),
            Shape::Record {
                name: "Gebruiker".into(),
                fields: vec![f("naam", Type::Ref(0), false), f("pin", Type::I32, true)],
                drop: false,
            },
            Shape::Record {
                name: "Empty".into(),
                fields: Vec::new(),
                drop: true,
            },
            Shape::Union {
                name: "Status".into(),
                variants: vec![
                    VariantShape {
                        name: "Paid".into(),
                        fields: Vec::new(),
                    },
                    VariantShape {
                        name: "Late".into(),
                        fields: vec![f("days", Type::I64, false), f("note", Type::Ref(0), false)],
                    },
                ],
            },
        ];
        assert_eq!(decode_shapes(&encode_shapes(&shapes)), Some(shapes));
    }
}
