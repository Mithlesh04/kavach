//! `json.schema<T>()` (spec 2.12) in native code. The schema is worked out
//! when the program is compiled, from the same rules the interpreter uses
//! when it runs (`kavach_driver::json_schema`), and built as a constant
//! `Json` value. Inside a generic function's instance, `T` is the
//! instance's type.

use crate::ir::{self, ExprKind as K, Shape, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast as ast;
use kavach_checker::{Prim, Ty};
use kavach_runtime_core::json::{self, Tree};

impl<'a> Lowerer<'a> {
    /// `json.schema<T>()` as the call `e` of `callee`, or `None` when it is
    /// another call.
    pub(crate) fn schema_call(
        &mut self,
        e: &'a ast::Expr,
        callee: &'a ast::Expr,
        json: Type,
    ) -> Option<Lowered<ir::Expr>> {
        let (ast::ExprKind::Field { base, name, .. }, ast::ExprKind::Call { generics, .. }) =
            (&callee.kind, &e.kind)
        else {
            return None;
        };
        let ast::ExprKind::Name(id) = &base.kind else {
            return None;
        };
        let def = self.c.resolution.uses.get(&id.span.start)?;
        if name.name != "schema" || self.std_modules.get(def).map(String::as_str) != Some("json") {
            return None;
        }
        let line = self.line(e.span);
        Some(self.schema_of(generics.first()?, json, line, e.span))
    }

    /// `json.describe(f)` (spec 2.12): the compiler's description of `f`,
    /// a constant `Json`, read into an `FnSchema` by `described` in
    /// `std/json.kav`. `None` when `e` is another call.
    pub(crate) fn describe_call(
        &mut self,
        e: &'a ast::Expr,
        callee: &'a ast::Expr,
        args: &'a [ast::Arg],
        ty: Type,
    ) -> Option<Lowered<ir::Expr>> {
        let ast::ExprKind::Field { base, name, .. } = &callee.kind else {
            return None;
        };
        let ast::ExprKind::Name(id) = &base.kind else {
            return None;
        };
        let def = self.c.resolution.uses.get(&id.span.start)?;
        if name.name != "describe" || self.std_modules.get(def).map(String::as_str) != Some("json")
        {
            return None;
        }
        let line = self.line(e.span);
        let named = args.first().is_some_and(|a| {
            matches!(&a.value.kind, ast::ExprKind::Name(id)
                if self.c.resolution.def_at(id.span.start)
                    .is_some_and(|d| d.kind == kavach_resolver::DefKind::Fn))
        });
        if let (false, Some(a)) = (named, args.first()) {
            return Some(self.describe_value(&a.value, ty, e.span));
        }
        let tree = args
            .first()
            .and_then(|a| kavach_driver::describe_fn(self.c, &a.value));
        let res = &self.c.resolution;
        let reader = self.c.file.items.iter().find_map(|item| match &item.kind {
            ast::ItemKind::Fn(f) if f.name.name == "described" => {
                let m = res.module_at(item.span.start);
                (res.modules[m].name == "std.json").then_some(f)
            }
            _ => None,
        });
        let fid = reader
            .and_then(|f| self.decl.get(&f.name.span.start))
            .and_then(|d| self.fn_ids.get(d))
            .copied();
        let json_def = res.std_type("Json");
        Some((|| {
            let (Some(tree), Some(fid), Some(json_def)) = (tree, fid, json_def) else {
                return Err(self.unsupported("describing this function", e.span));
            };
            let json = self.kir_type(&Ty::Adt(json_def, Vec::new()), e.span)?;
            let description = self.tree_value(&tree, json, line);
            Ok(ir::Expr {
                kind: K::Call(fid, vec![description]),
                ty,
                line,
            })
        })())
    }

    /// `json.describe(f)` on a function value (spec 2.12): its slot 0, the
    /// address of the function it calls, is compared with each named
    /// function of its type that the program uses as a value; a match
    /// gives that function's description, anything else `none`.
    fn describe_value(
        &mut self,
        arg: &'a ast::Expr,
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let value = self.expr(arg)?;
        let local = self.new_local(value.ty);
        let res = &self.c.resolution;
        let reader = self.c.file.items.iter().find_map(|item| match &item.kind {
            ast::ItemKind::Fn(f) if f.name.name == "described" => {
                let m = res.module_at(item.span.start);
                (res.modules[m].name == "std.json").then_some(f)
            }
            _ => None,
        });
        let reader = reader
            .and_then(|f| self.decl.get(&f.name.span.start))
            .and_then(|d| self.fn_ids.get(d))
            .copied();
        let (Some(reader), Some(json_def)) = (reader, res.std_type("Json")) else {
            return Err(self.unsupported("describing this function", span));
        };
        let json = self.kir_type(&Ty::Adt(json_def, Vec::new()), span)?;
        // The named functions used as values (their name is not followed
        // by `(`) whose type is the value's, as lowered: inside a generic
        // function's instance, with its type arguments.
        let mut used: Vec<(kavach_resolver::DefId, usize)> = Vec::new();
        for (&at, &def) in &res.uses {
            let d = &res.defs[def];
            let called = self.c.src[at as usize + d.name.len()..]
                .trim_start()
                .starts_with('(');
            if d.kind == kavach_resolver::DefKind::Fn && !called && !used.iter().any(|f| f.0 == def)
            {
                used.push((def, at as usize));
            }
        }
        // Each with its own type, when it differs from the value's: a
        // function that can't fail, adapted to one that can.
        let mut found = Vec::new();
        for (def, at) in used {
            let decl = self.c.resolution.defs[def].span;
            let t = decl.and_then(|s| self.c.expr_types.get(&(s.start, s.end)).cloned());
            let Some(own) = t.and_then(|t| self.kir_type(&t, span).ok()) else {
                continue;
            };
            let adapted = match (self.shapes.shape(own), self.shapes.shape(value.ty)) {
                (Some(Shape::Func(p, r, None)), Some(Shape::Func(q, s, Some(_)))) => {
                    p == q && r == s
                }
                _ => false,
            };
            if own == value.ty || adapted {
                found.push((def, at, own));
            }
        }
        found.sort_by_key(|f| f.1);
        let mut answer = ir::Expr {
            kind: K::None,
            ty,
            line,
        };
        for (def, _, own) in found.into_iter().rev() {
            let (Some(tree), Some(&fid)) = (
                kavach_driver::describe_def(self.c, def),
                self.fn_ids.get(&def),
            ) else {
                continue;
            };
            // The address of the small function a value of `def` calls.
            self.function_value(fid, own, span)?;
            let Some(&wrapper) = self.wrappers.get(&fid) else {
                continue;
            };
            let address = |kind| ir::Expr {
                kind,
                ty: Type::I64,
                line,
            };
            let is = |slot: ir::Expr, func| ir::Expr {
                kind: K::Binary(
                    ir::BinOp::Eq,
                    Box::new(slot),
                    Box::new(address(K::Closure {
                        func,
                        env: Type::Unit,
                        captures: Vec::new(),
                    })),
                ),
                ty: Type::Bool,
                line,
            };
            let held = self.local_expr(local, line);
            let cond = if own == value.ty {
                is(address(K::GetField(Box::new(held), 0)), wrapper)
            } else {
                // An adapter around a value of `def` (`never_failing`): the
                // adapter's function in slot 0, the value in slot 1.
                let Some(adapter) = self.adapter(own, value.ty, line) else {
                    continue;
                };
                let first = is(address(K::GetField(Box::new(held.clone()), 0)), adapter);
                let inner = ir::Expr {
                    kind: K::GetField(Box::new(held), 1),
                    ty: own,
                    line,
                };
                let second = is(address(K::GetField(Box::new(inner), 0)), wrapper);
                ir::Expr {
                    kind: K::And(Box::new(first), Box::new(second)),
                    ty: Type::Bool,
                    line,
                }
            };
            let description = self.tree_value(&tree, json, line);
            let schema = ir::Expr {
                kind: K::Call(reader, vec![description]),
                ty,
                line,
            };
            let yes = ir::Block {
                stmts: Vec::new(),
                value: Some(Box::new(ir::Expr {
                    kind: K::Some(Box::new(schema)),
                    ty,
                    line,
                })),
            };
            let no = ir::Block {
                stmts: Vec::new(),
                value: Some(Box::new(answer)),
            };
            answer = ir::Expr {
                kind: K::If(Box::new(cond), yes, no),
                ty,
                line,
            };
        }
        let block = ir::Block {
            stmts: vec![ir::Stmt::Let(local, value)],
            value: Some(Box::new(answer)),
        };
        Ok(ir::Expr {
            kind: K::Block(block),
            ty,
            line,
        })
    }

    fn schema_of(
        &mut self,
        written: &'a ast::TypeExpr,
        json: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let t = self.written_type(written)?;
        let tree = self
            .checker_ty(t)
            .and_then(|ty| kavach_driver::json_schema(self.c, &ty))
            .ok_or_else(|| self.unsupported("the JSON Schema of this type", span))?;
        Ok(self.tree_value(&tree, json, line))
    }

    /// The checker's type for type `t`, as far as a schema needs it.
    fn checker_ty(&self, t: Type) -> Option<Ty> {
        Some(Ty::Prim(match t {
            Type::I8 => Prim::Int8,
            Type::I16 => Prim::Int16,
            Type::I32 => Prim::Int32,
            Type::I64 => Prim::Int,
            Type::U8 => Prim::UInt8,
            Type::U16 => Prim::UInt16,
            Type::U32 => Prim::UInt32,
            Type::U64 => Prim::UInt64,
            Type::F64 => Prim::Float,
            Type::F32 => Prim::Float32,
            Type::Bool => Prim::Bool,
            Type::Char => Prim::Char,
            Type::Unit => return Some(Ty::Unit),
            Type::Ref(_) => {
                return match self.shapes.shape(t)? {
                    Shape::Str => Some(Ty::Prim(Prim::String)),
                    Shape::Bytes => Some(Ty::Prim(Prim::Bytes)),
                    Shape::Decimal => Some(Ty::Prim(Prim::Decimal)),
                    Shape::List(x) => Some(Ty::List(Box::new(self.checker_ty(*x)?))),
                    Shape::Optional(x) => Some(Ty::Optional(Box::new(self.checker_ty(*x)?))),
                    Shape::Record { .. } | Shape::Union { .. } => {
                        Some(Ty::Adt(self.shapes.def_of(t)?, Vec::new()))
                    }
                    _ => None,
                };
            }
        }))
    }

    /// `tree` as a `Json` value of type `json`.
    fn tree_value(&mut self, tree: &Tree, json: Type, line: u32) -> ir::Expr {
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let (name, field) = match tree {
            Tree::Null => return self.json_variant(json, "Null", None, line),
            Tree::Bool(b) => ("Boolean", at(K::Bool(*b), Type::Bool)),
            // Every number by its digits (decision 0071).
            Tree::Int(_) | Tree::Float(_) | Tree::Number(_) => match json::digits(tree) {
                Some(d) => ("Number", at(K::Str(d), self.json_field(json, "Number"))),
                None => return self.json_variant(json, "Null", None, line),
            },
            Tree::Text(s) => ("Text", at(K::Str(s.clone()), self.json_field(json, "Text"))),
            Tree::Array(items) => {
                let items = items
                    .iter()
                    .map(|i| self.tree_value(i, json, line))
                    .collect();
                ("Array", at(K::List(items), self.json_field(json, "Array")))
            }
            Tree::Object(fields) => {
                let text = self.str_type();
                let fields = fields
                    .iter()
                    .map(|(k, v)| (at(K::Str(k.clone()), text), self.tree_value(v, json, line)))
                    .collect();
                (
                    "Object",
                    at(K::Map(fields), self.json_field(json, "Object")),
                )
            }
        };
        self.json_variant(json, name, Some(field), line)
    }
}
