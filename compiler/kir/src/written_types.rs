// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The KIR type of a type written in the source (`Map<K, V>`, `[T]`,
//! `Pipeline<V, I, S, T>`, a record's name): what a declaration, a cast or
//! a type argument says, as `types.rs` does for the checker's types.

use crate::ir::{FieldShape, Shape, Type};
use crate::lower::{Lowered, Lowerer};
use crate::types::prim_type;
use kavach_ast::{TypeExpr, TypeKind};
use kavach_checker::Prim;
use kavach_resolver::DefKind;

impl Lowerer<'_> {
    /// The KIR type of a type written in the source.
    pub(crate) fn written_type(&mut self, t: &TypeExpr) -> Lowered<Type> {
        let span = t.span;
        Ok(match &t.kind {
            // A collection's numbers don't change its layout (spec 2.2).
            TypeKind::WithNums(base, _) => return self.written_type(base),
            TypeKind::Named { path, args }
                if args.iter().any(|a| matches!(a.kind, TypeKind::Num { .. })) =>
            {
                return self.numbered_type(t, path, args);
            }
            TypeKind::Named { path, args } => {
                // `regex.Regex`, `fs.FsError`: the type is the last name.
                let first = path.segments.last().unwrap_or(&path.segments[0]);
                let def = self.c.resolution.uses.get(&first.span.start).copied();
                match def.map(|d| self.c.resolution.defs[d].kind) {
                    // Text in native code for now (`lower_views.rs`).
                    Some(DefKind::Builtin)
                        if matches!(first.name.as_str(), "String" | "StrView" | "TextBuilder") =>
                    {
                        self.str_type()
                    }
                    Some(DefKind::Builtin) if first.name == "Decimal" => self.decimal_type(),
                    Some(DefKind::Builtin) if first.name == "BigInt" => {
                        self.shapes.intern(Shape::BigInt)
                    }
                    Some(DefKind::Builtin) if first.name == "Regex" => {
                        self.shapes.intern(Shape::Regex)
                    }
                    Some(DefKind::Builtin)
                        if matches!(
                            first.name.as_str(),
                            "Error"
                                | "ParseError"
                                | "InvariantError"
                                | "SizeError"
                                | "CliError"
                                | "RegexError"
                                | "LibraryError"
                        ) =>
                    {
                        self.error_type()
                    }
                    Some(DefKind::Builtin) if first.name == "List" && args.len() == 1 => {
                        let item = self.written_type(&args[0])?;
                        self.list_type(item)
                    }
                    Some(DefKind::Builtin) if first.name == "Dict" && args.len() == 2 => {
                        let (k, v) = (self.written_type(&args[0])?, self.written_type(&args[1])?);
                        self.map_type(k, v)
                    }
                    Some(DefKind::Builtin) if first.name == "Ring" && args.len() == 1 => {
                        let item = self.written_type(&args[0])?;
                        self.list_type(item)
                    }
                    Some(DefKind::Builtin) if first.name == "Set" && args.len() == 1 => {
                        let item = self.written_type(&args[0])?;
                        self.shapes.intern(Shape::Set(item))
                    }
                    Some(DefKind::Builtin) if ["Ptr", "Pipeline"].contains(&&*first.name) => {
                        Type::I64
                    }
                    Some(DefKind::Builtin)
                        if matches!(first.name.as_str(), "Shared" | "Weak") && args.len() == 1 =>
                    {
                        let inner = self.written_type(&args[0])?;
                        self.shapes.intern(Shape::Shared(inner))
                    }
                    Some(DefKind::Builtin) if first.name == "Task" && args.len() == 1 => {
                        let inner = self.written_type(&args[0])?;
                        self.shapes.intern(Shape::Task(inner))
                    }
                    Some(DefKind::Builtin)
                        if matches!(first.name.as_str(), "Sender" | "Receiver")
                            && args.len() == 1 =>
                    {
                        let inner = self.written_type(&args[0])?;
                        self.shapes.intern(Shape::Channel(inner))
                    }
                    Some(DefKind::Builtin)
                        if matches!(
                            first.name.as_str(),
                            "Listener" | "Connection" | "UdpSocket"
                        ) =>
                    {
                        self.shapes.intern(Shape::Socket)
                    }
                    // A frozen value is its value; a sealed one `(value,
                    // key)` (spec 2.20).
                    Some(DefKind::Builtin) if first.name == "Frozen" => {
                        let parts = args
                            .iter()
                            .map(|t| self.written_type(t))
                            .collect::<Lowered<Vec<_>>>()?;
                        match parts.as_slice() {
                            [value] => *value,
                            _ => self.shapes.intern(Shape::Tuple(parts)),
                        }
                    }
                    Some(DefKind::Builtin) if first.name == "Bytes" => {
                        self.shapes.intern(Shape::Bytes)
                    }
                    // `fs.lines` and `fs.chunks` readers (spec 14.3).
                    Some(DefKind::Builtin) if matches!(first.name.as_str(), "Lines" | "Chunks") => {
                        let item = match first.name.as_str() {
                            "Lines" => self.str_type(),
                            _ => self.shapes.intern(Shape::Bytes),
                        };
                        self.shapes.intern(Shape::Reader(item))
                    }
                    Some(DefKind::Builtin) => Prim::from_name(&first.name)
                        .and_then(prim_type)
                        .ok_or_else(|| {
                            self.unsupported(format!("the type `{}`", self.text(span)), span)
                        })?,
                    Some(DefKind::Type) => {
                        let args = args
                            .iter()
                            .map(|t| self.written_type(t))
                            .collect::<Lowered<Vec<_>>>()?;
                        self.adt_instance(def.unwrap(), args, span)?
                    }
                    Some(DefKind::Interface) => {
                        let args = args
                            .iter()
                            .map(|t| self.written_type(t))
                            .collect::<Lowered<Vec<_>>>()?;
                        self.iface_type(def.unwrap(), args, span)?
                    }
                    Some(DefKind::Agent) => self.agent_type(def.unwrap(), span)?,
                    Some(DefKind::GenericParam) => match self.subst.get(&def.unwrap()) {
                        Some(t) => *t,
                        None => return Err(self.unsupported("generic types here", span)),
                    },
                    _ => {
                        return Err(
                            self.unsupported(format!("the type `{}`", self.text(span)), span)
                        );
                    }
                }
            }
            TypeKind::List(item) => {
                let item = self.written_type(item)?;
                self.list_type(item)
            }
            TypeKind::Optional(inner) => {
                let inner = self.written_type(inner)?;
                self.optional_type(inner)
            }
            TypeKind::Map(k, v) => {
                let (k, v) = (self.written_type(k)?, self.written_type(v)?);
                self.map_type(k, v)
            }
            TypeKind::Fn {
                params,
                ret,
                errors,
                ..
            } => {
                let params = params
                    .iter()
                    .map(|t| self.written_type(t))
                    .collect::<Lowered<Vec<_>>>()?;
                let ret = match ret {
                    Some(r) => self.written_type(r)?,
                    None => Type::Unit,
                };
                let err = match errors {
                    Some(e) => Some(self.written_type(e)?),
                    None => None,
                };
                self.shapes.intern(Shape::Func(params, ret, err))
            }
            TypeKind::Tuple(items) => {
                let items = items
                    .iter()
                    .map(|t| self.written_type(t))
                    .collect::<Lowered<Vec<_>>>()?;
                self.shapes.intern(Shape::Tuple(items))
            }
            // An anonymous record type (spec 2.14): fields sorted by
            // name, as the checker sorts them.
            TypeKind::AnonRecord(fs) => {
                let mut fields = fs
                    .iter()
                    .map(|(n, t)| {
                        Ok(FieldShape {
                            name: n.name.clone(),
                            ty: self.written_type(t)?,
                            secret: false,
                        })
                    })
                    .collect::<Lowered<Vec<_>>>()?;
                fields.sort_by(|a, b| a.name.cmp(&b.name));
                self.shapes.intern(Shape::Record {
                    name: String::new(),
                    fields,
                    drop: false,
                })
            }
            _ => return Err(self.unsupported(format!("the type `{}`", self.text(span)), span)),
        })
    }
}
