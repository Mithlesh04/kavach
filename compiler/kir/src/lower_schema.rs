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
