//! Lowering data: calls and methods, records and variants, fields, and
//! `match` with patterns.

use crate::ir::{self, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::lower_expr::wrap;
use kavach_ast::{self as ast, ExprKind, ItemKind, TypeBody};
use kavach_resolver::{DefId, DefKind};

impl<'a> Lowerer<'a> {
    /// A record (`variant` is `None`) or a union variant, from named arguments.
    pub(crate) fn construct(
        &mut self,
        def: DefId,
        variant: Option<&str>,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let decl_span = self.c.resolution.defs[def].span;
        let decl = self.c.file.items.iter().find_map(|item| match &item.kind {
            ItemKind::Type(t) if Some(t.name.span) == decl_span => Some(t),
            _ => None,
        });
        let Some(decl) = decl else {
            return Err(self.unsupported("this type", span));
        };
        let (index, fields): (u32, &'a [ast::Field]) = match (&decl.body, variant) {
            (TypeBody::Record(r), None) => (0, &r.fields),
            (TypeBody::Union(vs), Some(v)) => match vs.iter().position(|x| x.name.name == v) {
                Some(i) => (i as u32, &vs[i].fields),
                None => return Err(self.unsupported("this variant", span)),
            },
            _ => return Err(self.unsupported("this constructor", span)),
        };
        // The field types of this type (and its type arguments).
        let types: Vec<Type> = match self.shapes.shape(ty) {
            Some(Shape::Record { fields, .. }) => fields.iter().map(|f| f.ty).collect(),
            Some(Shape::Union { variants, .. }) => variants[index as usize]
                .fields
                .iter()
                .map(|f| f.ty)
                .collect(),
            _ => return Err(self.unsupported("this constructor", span)),
        };
        let mut values = Vec::new();
        for (i, f) in fields.iter().enumerate() {
            let given = args
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|n| n.name == f.name.name))
                .or_else(|| args.get(i).filter(|a| a.name.is_none()));
            let value = match (given, &f.default) {
                (Some(a), _) => self.expr(&a.value)?,
                (None, Some(d)) => self.field_default(def, d, ty)?,
                (None, None) => return Err(self.unsupported("this constructor", span)),
            };
            values.push(self.coerce(value, types[i])?);
        }
        let record = ir::Expr {
            kind: K::Record {
                variant: index,
                fields: values,
            },
            ty,
            line,
        };
        if variant.is_some() {
            return Ok(record);
        }
        self.with_invariants(record)
    }
    /// Default `d` of a field of type `def`, made as type `ty`: a default
    /// of generic type (`items: [Item<S>] = []`) takes `ty`'s arguments.
    fn field_default(&mut self, def: DefId, d: &'a ast::Expr, ty: Type) -> Lowered<ir::Expr> {
        let params = self.type_generics(def);
        let args = self.shapes.args_of(ty).unwrap_or_default().to_vec();
        let mut subst = self.subst.clone();
        subst.extend(params.into_iter().zip(args));
        let outer = std::mem::replace(&mut self.subst, subst);
        let value = self.expr(d);
        self.subst = outer;
        value
    }

    /// `value` (a new or changed record), after checking its type's
    /// invariants (spec 7.3): a false one panics with the contract.
    pub(crate) fn with_invariants(&mut self, value: ir::Expr) -> Lowered<ir::Expr> {
        let Some(def) = self.shapes.def_of(value.ty) else {
            return Ok(value);
        };
        let decl_span = self.c.resolution.defs[def].span;
        let invariants: &'a [ast::Expr] =
            match self.c.file.items.iter().find_map(|item| match &item.kind {
                ItemKind::Type(t) if Some(t.name.span) == decl_span => Some(&t.body),
                _ => None,
            }) {
                Some(TypeBody::Record(r)) => &r.invariants,
                _ => return Ok(value),
            };
        if invariants.is_empty() {
            return Ok(value);
        }
        let (ty, line) = (value.ty, value.line);
        let tmp = self.new_local(ty);
        let mut stmts = vec![Stmt::Let(tmp, value)];
        let outer = self.invariant_of.replace((tmp, ty));
        for inv in invariants {
            let cond = self.expr(inv);
            let cond = match cond {
                Ok(c) => c,
                Err(e) => {
                    self.invariant_of = outer;
                    return Err(e);
                }
            };
            stmts.push(Stmt::Check {
                cond,
                message: format!("contract failed: invariant {}", self.text(inv.span)),
                line,
            });
        }
        self.invariant_of = outer;
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts,
                value: Some(Box::new(self.local_expr(tmp, line))),
            }),
            ty,
            line,
        })
    }
    /// A variant of the union type `ty`, by name.
    pub(crate) fn variant_value(
        &mut self,
        ty: Type,
        name: &str,
        fields: Vec<ir::Expr>,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        match self.shapes.shape(ty) {
            Some(Shape::Union { variants, .. }) => {
                match variants.iter().position(|v| v.name == name) {
                    Some(i) => Ok(ir::Expr {
                        kind: K::Record {
                            variant: i as u32,
                            fields,
                        },
                        ty,
                        line,
                    }),
                    None => Err(self.unsupported("this variant", span)),
                }
            }
            _ => Err(self.unsupported("this variant", span)),
        }
    }
    pub(crate) fn field(
        &mut self,
        base: &'a ast::Expr,
        name: &'a ast::Ident,
        optional: bool,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        // `5.seconds` (spec 14.4): a `Duration` of `5 * 1000` ms.
        let unit = kavach_checker::DURATION_UNITS
            .iter()
            .find(|(u, _)| *u == name.name);
        let int_base = matches!(
            self.c.expr_types.get(&(base.span.start, base.span.end)),
            Some(kavach_checker::Ty::Prim(kavach_checker::Prim::Int))
        ) || matches!(base.kind, ExprKind::Int(_));
        if let (Some((_, factor)), true) = (unit, int_base) {
            let n = self.expr(base)?;
            let n = self.coerce(n, Type::I64)?;
            let factor = ir::Expr {
                kind: K::Int(*factor as i128),
                ty: Type::I64,
                line,
            };
            let ms = ir::Expr {
                kind: K::Binary(ir::BinOp::Mul, Box::new(n), Box::new(factor)),
                ty: Type::I64,
                line,
            };
            return Ok(ir::Expr {
                kind: K::Record {
                    variant: 0,
                    fields: vec![ms],
                },
                ty,
                line,
            });
        }
        if let ExprKind::Name(id) = &base.kind {
            let def = self.c.resolution.uses.get(&id.span.start).copied();
            if let Some(module) = def.and_then(|d| self.std_modules.get(&d)).cloned() {
                return self.std_constant(&module, &name.name, name.span);
            }
            if def.is_some_and(|d| self.c.resolution.defs[d].kind == DefKind::Type) {
                return self.variant_value(ty, &name.name, Vec::new(), line, name.span);
            }
        }
        let value = self.expr(base)?;
        if !optional {
            let (index, _) = self.field_index(value.ty, &name.name, name.span)?;
            return Ok(ir::Expr {
                kind: K::GetField(Box::new(value), index),
                ty,
                line,
            });
        }
        // `a?.b`: `none` stays `none`; otherwise the field, as an optional.
        let Some(Shape::Optional(inner)) = self.shapes.shape(value.ty).cloned() else {
            return Err(self.unsupported("`?.` here", name.span));
        };
        let (index, field_ty) = self.field_index(inner, &name.name, name.span)?;
        let tmp = self.new_local(value.ty);
        let present = ir::Expr {
            kind: K::IsSome(Box::new(self.local_expr(tmp, line))),
            ty: Type::Bool,
            line,
        };
        let inside = ir::Expr {
            kind: K::Unwrap(Box::new(self.local_expr(tmp, line))),
            ty: inner,
            line,
        };
        let got = ir::Expr {
            kind: K::GetField(Box::new(inside), index),
            ty: field_ty,
            line,
        };
        let got = self.coerce(got, ty)?;
        let none = ir::Expr {
            kind: K::None,
            ty,
            line,
        };
        let choice = ir::Expr {
            kind: K::If(Box::new(present), wrap(got, true), wrap(none, true)),
            ty,
            line,
        };
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts: vec![Stmt::Let(tmp, value)],
                value: Some(Box::new(choice)),
            }),
            ty,
            line,
        })
    }
    /// The index and type of a record field.
    pub(crate) fn field_index(
        &self,
        ty: Type,
        name: &str,
        span: ast::Span,
    ) -> Lowered<(u32, Type)> {
        match self.shapes.shape(ty) {
            Some(Shape::Record { fields, .. }) => fields
                .iter()
                .position(|f| f.name == name)
                .map(|i| (i as u32, fields[i].ty))
                .ok_or_else(|| self.unsupported(format!("the field `{name}`"), span)),
            _ => Err(self.unsupported(format!("the field `{name}` here"), span)),
        }
    }
}
