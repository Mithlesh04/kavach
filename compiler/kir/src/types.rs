//! KIR types for checker types and written types, and the table of heap
//! shapes (`Program::shapes`).

use crate::ir::{FieldShape, Shape, Type, TypeId, VariantShape};
use crate::lower::{Lowered, Lowerer};
use kavach_ast::{ItemKind, Span, TypeBody};
use kavach_checker::{Prim, Ty};
use kavach_resolver::{DefId, DefKind};
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct Shapes {
    pub(crate) list: Vec<Shape>,
    interned: HashMap<Shape, TypeId>,
    /// Records and unions, by declaration and type arguments.
    pub(crate) adts: HashMap<(DefId, Vec<Type>), TypeId>,
}

impl Shapes {
    pub(crate) fn intern(&mut self, shape: Shape) -> Type {
        if let Some(id) = self.interned.get(&shape) {
            return Type::Ref(*id);
        }
        let id = self.list.len() as TypeId;
        self.interned.insert(shape.clone(), id);
        self.list.push(shape);
        Type::Ref(id)
    }

    /// The record or union that `ty` was declared as, if any.
    pub(crate) fn def_of(&self, ty: Type) -> Option<DefId> {
        let Type::Ref(id) = ty else { return None };
        self.adts
            .iter()
            .find(|(_, t)| **t == id)
            .map(|((d, _), _)| *d)
    }

    /// The type arguments of a generic record or union type.
    pub(crate) fn args_of(&self, ty: Type) -> Option<&[Type]> {
        let Type::Ref(id) = ty else { return None };
        self.adts
            .iter()
            .find(|(_, t)| **t == id)
            .map(|((_, args), _)| args.as_slice())
    }

    /// The type of a non-generic record or union made so far.
    pub(crate) fn adt(&self, def: DefId) -> Option<Type> {
        self.adts.get(&(def, Vec::new())).map(|id| Type::Ref(*id))
    }

    /// Adds `shape` as the type of declaration `def` (an agent).
    pub(crate) fn add_adt(&mut self, def: DefId, shape: Shape) -> Type {
        let id = self.list.len() as TypeId;
        self.list.push(shape);
        self.adts.insert((def, Vec::new()), id);
        Type::Ref(id)
    }

    pub(crate) fn shape(&self, ty: Type) -> Option<&Shape> {
        match ty {
            Type::Ref(id) => self.list.get(id as usize),
            _ => None,
        }
    }
}

/// The KIR type of a built-in scalar, if supported.
pub(crate) fn prim_type(p: Prim) -> Option<Type> {
    Some(match p {
        Prim::Int => Type::I64,
        Prim::Int8 => Type::I8,
        Prim::Int16 => Type::I16,
        Prim::Int32 => Type::I32,
        Prim::UInt8 => Type::U8,
        Prim::UInt16 => Type::U16,
        Prim::UInt32 => Type::U32,
        Prim::UInt64 => Type::U64,
        Prim::Float => Type::F64,
        Prim::Float32 => Type::F32,
        Prim::Bool => Type::Bool,
        Prim::Char => Type::Char,
        _ => return None,
    })
}

impl Lowerer<'_> {
    pub(crate) fn str_type(&mut self) -> Type {
        self.shapes.intern(Shape::Str)
    }

    pub(crate) fn list_type(&mut self, item: Type) -> Type {
        self.shapes.intern(Shape::List(item))
    }

    pub(crate) fn optional_type(&mut self, inner: Type) -> Type {
        self.shapes.intern(Shape::Optional(inner))
    }

    pub(crate) fn shapes_intern(&mut self, shape: Shape) -> Type {
        self.shapes.intern(shape)
    }

    pub(crate) fn tuple_type(&mut self, items: Vec<Type>) -> Type {
        self.shapes.intern(Shape::Tuple(items))
    }

    pub(crate) fn map_type(&mut self, k: Type, v: Type) -> Type {
        self.shapes.intern(Shape::Map(k, v))
    }

    pub(crate) fn decimal_type(&mut self) -> Type {
        self.shapes.intern(Shape::Decimal)
    }

    pub(crate) fn error_type(&mut self) -> Type {
        self.shapes.intern(Shape::Error)
    }

    /// The KIR type of a checker type.
    pub(crate) fn kir_type(&mut self, ty: &Ty, span: Span) -> Lowered<Type> {
        Ok(match ty {
            Ty::Prim(Prim::String) => self.str_type(),
            Ty::Prim(Prim::Decimal) => self.decimal_type(),
            Ty::Prim(Prim::BigInt) => self.shapes.intern(Shape::BigInt),
            Ty::Prim(Prim::Bytes) => self.shapes.intern(Shape::Bytes),
            Ty::Builtin("Listener" | "Connection" | "UdpSocket", _) => {
                self.shapes.intern(Shape::Socket)
            }
            Ty::Builtin("Lines", _) => {
                let text = self.str_type();
                self.shapes.intern(Shape::Reader(text))
            }
            Ty::Builtin("Chunks", _) => {
                let bytes = self.shapes.intern(Shape::Bytes);
                self.shapes.intern(Shape::Reader(bytes))
            }
            Ty::Prim(p) => prim_type(*p)
                .ok_or_else(|| self.unsupported(format!("{} values", p.name()), span))?,
            Ty::Unit | Ty::Never => Type::Unit,
            // `json.of([])`: nothing tells the items' type, and there are
            // none, so any type will do.
            Ty::List(t) if matches!(**t, Ty::Var(_) | Ty::Error) => self.list_type(Type::I64),
            // `json.of(none)`: the same, for an optional that is always
            // `none`.
            Ty::Optional(t) if matches!(**t, Ty::Var(_) | Ty::Error) => {
                self.kir_type(&Ty::Optional(Box::new(Ty::Prim(Prim::Int))), span)?
            }
            Ty::List(t) => {
                let item = self.kir_type(t, span)?;
                self.list_type(item)
            }
            Ty::Optional(t) => {
                let inner = self.kir_type(t, span)?;
                self.optional_type(inner)
            }
            Ty::Map(k, v) => {
                let (k, v) = (self.kir_type(k, span)?, self.kir_type(v, span)?);
                self.map_type(k, v)
            }
            Ty::Set(t) => {
                let item = self.kir_type(t, span)?;
                self.shapes.intern(Shape::Set(item))
            }
            // A ring is a list with a limit that drops its oldest item
            // when full (spec 2.2): the runtime's ring flag says so.
            Ty::Builtin("Ring", args) if args.len() == 1 => {
                let item = self.kir_type(&args[0], span)?;
                self.list_type(item)
            }
            Ty::Builtin("Shared" | "Weak", args) if args.len() == 1 => {
                let inner = self.kir_type(&args[0], span)?;
                self.shapes.intern(Shape::Shared(inner))
            }
            Ty::Tuple(ts) => {
                let items = ts
                    .iter()
                    .map(|t| self.kir_type(t, span))
                    .collect::<Lowered<Vec<_>>>()?;
                self.shapes.intern(Shape::Tuple(items))
            }
            Ty::Adt(def, args) if self.c.resolution.defs[*def].kind == DefKind::Interface => {
                let args = args
                    .iter()
                    .map(|t| self.kir_type(t, span))
                    .collect::<Lowered<Vec<_>>>()?;
                self.iface_type(*def, args, span)?
            }
            Ty::Adt(def, args) => {
                let args = args
                    .iter()
                    .map(|t| self.kir_type(t, span))
                    .collect::<Lowered<Vec<_>>>()?;
                self.adt_instance(*def, args, span)?
            }
            // An anonymous record (spec 2.14): a record shape with no
            // name, interned by its fields, so equal shapes share one.
            Ty::Anon(fs) => {
                let fields = fs
                    .iter()
                    .map(|(n, t)| {
                        Ok(FieldShape {
                            name: n.clone(),
                            ty: self.kir_type(t, span)?,
                            secret: false,
                        })
                    })
                    .collect::<Lowered<Vec<_>>>()?;
                self.shapes.intern(Shape::Record {
                    name: String::new(),
                    fields,
                    drop: false,
                })
            }
            // A generic parameter of the function being lowered.
            Ty::Param(def) => match self.subst.get(def) {
                Some(t) => *t,
                None => return Err(self.unsupported("generic types here", span)),
            },
            Ty::Fn(params, ret, _) => {
                let params = params
                    .iter()
                    .map(|t| self.kir_type(t, span))
                    .collect::<Lowered<Vec<_>>>()?;
                let err = match &**ret {
                    Ty::Result(_, e) => Some(self.kir_type(e, span)?),
                    _ => None,
                };
                let ret = self.kir_type(ret, span)?;
                self.shapes.intern(Shape::Func(params, ret, err))
            }
            // A call that can fail: its success value (spec 5).
            Ty::Result(ok, _) => self.kir_type(ok, span)?,
            Ty::Builtin("Error" | "ParseError" | "CliError" | "RegexError" | "LibraryError", _) => {
                self.error_type()
            }
            Ty::Builtin("Regex", _) => self.shapes.intern(Shape::Regex),
            // Text in native code for now (`lower_views.rs`).
            Ty::Builtin("StrView" | "TextBuilder", _) => self.str_type(),
            Ty::Builtin("Sender" | "Receiver", args) if args.len() == 1 => {
                let inner = self.kir_type(&args[0], span)?;
                self.shapes.intern(Shape::Channel(inner))
            }
            Ty::Builtin("Task", args) if args.len() == 1 => {
                let inner = self.kir_type(&args[0], span)?;
                self.shapes.intern(Shape::Task(inner))
            }
            // `time.every_ms(ms)` is its period, in milliseconds (spec 14.4).
            Ty::Builtin("Ticker", _) => Type::I64,
            // An address (spec 6.4), never a reference Kavach counts; a
            // pipeline's number (15.3).
            Ty::Builtin("Ptr" | "Pipeline", _) => Type::I64,
            _ => {
                return Err(
                    self.unsupported("maps, generic types, functions and errors as values", span)
                );
            }
        })
    }

    /// A record or union declared in this file, with its type arguments
    /// (one shape per set of arguments). Its slot is reserved before its
    /// fields are lowered, so recursive types work.
    pub(crate) fn adt_instance(
        &mut self,
        def: DefId,
        args: Vec<Type>,
        span: Span,
    ) -> Lowered<Type> {
        if let Some(id) = self.shapes.adts.get(&(def, args.clone())) {
            return Ok(Type::Ref(*id));
        }
        if self.c.resolution.defs[def].kind == DefKind::Agent {
            return self.agent_type(def, span);
        }
        let decl_span = self.c.resolution.defs[def].span;
        let decl = self.c.file.items.iter().find_map(|item| match &item.kind {
            ItemKind::Type(t) if Some(t.name.span) == decl_span => Some(t),
            _ => None,
        });
        let Some(decl) = decl else {
            return Err(self.unsupported("this type", span));
        };
        // An alias names an anonymous shape (spec 2.14).
        if let TypeBody::Alias(t) = &decl.body {
            return self.written_type(t);
        }
        if decl.generics.len() != args.len() {
            return Err(self.unsupported("this generic type", span));
        }
        let id = self.shapes.list.len() as TypeId;
        self.shapes.list.push(Shape::Str);
        self.shapes.adts.insert((def, args.clone()), id);
        // Field types see the type's parameters as its arguments.
        let outer = self.subst.clone();
        for (g, t) in decl.generics.iter().zip(&args) {
            if let Some(&p) = self.decl.get(&g.name.span.start) {
                self.subst.insert(p, *t);
            }
        }
        let fields = |l: &mut Self, fields: &[kavach_ast::Field]| -> Lowered<Vec<FieldShape>> {
            fields
                .iter()
                .map(|f| {
                    Ok(FieldShape {
                        name: f.name.name.clone(),
                        ty: l.written_type(&f.ty)?,
                        secret: f.is_secret,
                    })
                })
                .collect()
        };
        let shape = match &decl.body {
            TypeBody::Record(r) => fields(self, &r.fields).map(|fields| Shape::Record {
                name: decl.name.name.clone(),
                fields,
                drop: self.c.drop_types.contains(&def),
            }),
            TypeBody::Union(vs) => vs
                .iter()
                .map(|v| {
                    Ok(VariantShape {
                        name: v.name.name.clone(),
                        fields: fields(self, &v.fields)?,
                    })
                })
                .collect::<Lowered<Vec<_>>>()
                .map(|variants| Shape::Union {
                    name: decl.name.name.clone(),
                    variants,
                }),
            // The checker replaces an alias by its anonymous shape.
            TypeBody::Alias(_) => Err(self.unsupported("an alias here", decl.name.span)),
        };
        self.subst = outer;
        let shape = shape?;
        self.shapes.list[id as usize] = shape;
        Ok(Type::Ref(id))
    }
}
