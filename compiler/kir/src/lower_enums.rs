//! Enums in native code (spec 2.6): `Type.all()` is a list of the
//! variants; `Type.parse(text)` and `Type.from(n)` compare with each
//! variant's name or number in turn; `v.name()` and `v.value()` test the
//! variant in turn. Everything is known when compiling, so this is plain
//! KIR: no runtime support.

use crate::ir::{self, Block, ExprKind as K, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast::{self as ast, ItemKind, TypeBody};
use kavach_resolver::DefId;

impl<'a> Lowerer<'a> {
    /// The variants of union `def`, as declared.
    fn declared_variants(&self, def: DefId) -> Option<&'a [ast::Variant]> {
        let span = self.c.resolution.defs[def].span;
        self.c.file.items.iter().find_map(|item| match &item.kind {
            ItemKind::Type(t) if Some(t.name.span) == span => match &t.body {
                TypeBody::Union(vs) => Some(&vs[..]),
                _ => None,
            },
            _ => None,
        })
    }

    /// `Type.all()`, `Type.parse(text)` or `Type.from(n)` on union `def`,
    /// giving `ty`; `None` when `name` is none of them.
    pub(crate) fn enum_call(
        &mut self,
        def: DefId,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Option<Lowered<ir::Expr>> {
        if !matches!(name, "all" | "parse" | "from") {
            return None;
        }
        let vs = self.declared_variants(def)?;
        // A variant of that name is a variant.
        if vs.iter().any(|v| v.name.name == name) {
            return None;
        }
        Some(self.enum_call_of(def, vs, name, args, ty, line, span))
    }

    #[allow(clippy::too_many_arguments)]
    fn enum_call_of(
        &mut self,
        def: DefId,
        vs: &'a [ast::Variant],
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let this = self.adt_instance(def, Vec::new(), span)?;
        let variant = |i: usize| ir::Expr {
            kind: K::Record {
                variant: i as u32,
                fields: Vec::new(),
            },
            ty: this,
            line,
        };
        if name == "all" {
            let items = (0..vs.len()).map(variant).collect();
            return Ok(ir::Expr {
                kind: K::List(items),
                ty,
                line,
            });
        }
        // `parse` and `from`: the argument once, then each variant in turn.
        let arg = args.first().map(|a| &a.value);
        let Some(arg) = arg else {
            return Err(self.unsupported("this call", span));
        };
        let value = self.expr(arg)?;
        let held = self.new_local(value.ty);
        let mut result = ir::Expr {
            kind: K::None,
            ty,
            line,
        };
        for (i, v) in vs.iter().enumerate().rev() {
            let key = if name == "parse" {
                K::Str(v.name.name.clone())
            } else {
                match v.value {
                    Some(n) => K::Int(n),
                    None => continue,
                }
            };
            let same = ir::Expr {
                kind: K::Binary(
                    ir::BinOp::Eq,
                    Box::new(self.local_expr(held, line)),
                    Box::new(ir::Expr {
                        kind: key,
                        ty: value.ty,
                        line,
                    }),
                ),
                ty: Type::Bool,
                line,
            };
            let found = ir::Expr {
                kind: K::Some(Box::new(variant(i))),
                ty,
                line,
            };
            result = choose(same, found, result, ty, line);
        }
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts: vec![Stmt::Let(held, value)],
                value: Some(Box::new(result)),
            }),
            ty,
            line,
        })
    }

    /// `v.name()` or `v.value()` on union value `receiver`; `None` when
    /// `name` is neither, or `receiver` isn't a declared union.
    pub(crate) fn enum_method(
        &mut self,
        receiver: &ir::Expr,
        def: DefId,
        name: &str,
        line: u32,
    ) -> Option<ir::Expr> {
        if !matches!(name, "name" | "value") {
            return None;
        }
        let vs = self.declared_variants(def)?;
        let ty = if name == "name" {
            self.str_type()
        } else {
            Type::I64
        };
        let held = self.new_local(receiver.ty);
        let constant = |v: &ast::Variant| ir::Expr {
            kind: if name == "name" {
                K::Str(v.name.name.clone())
            } else {
                K::Int(v.value.unwrap_or(0))
            },
            ty,
            line,
        };
        // The last variant needs no test: it is the one left.
        let mut result = constant(vs.last()?);
        for (i, v) in vs.iter().enumerate().rev().skip(1) {
            let is = ir::Expr {
                kind: K::HasVariant(Box::new(self.local_expr(held, line)), i as u32),
                ty: Type::Bool,
                line,
            };
            result = choose(is, constant(v), result, ty, line);
        }
        Some(ir::Expr {
            kind: K::Block(Block {
                stmts: vec![Stmt::Let(held, receiver.clone())],
                value: Some(Box::new(result)),
            }),
            ty,
            line,
        })
    }
}

/// `if cond { yes } else { no }`, giving `ty`.
fn choose(cond: ir::Expr, yes: ir::Expr, no: ir::Expr, ty: Type, line: u32) -> ir::Expr {
    let block = |e: ir::Expr| Block {
        stmts: Vec::new(),
        value: Some(Box::new(e)),
    };
    ir::Expr {
        kind: K::If(Box::new(cond), block(yes), block(no)),
        ty,
        line,
    }
}
