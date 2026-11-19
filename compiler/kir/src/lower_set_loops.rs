//! A set's `any`, `all`, `sum`, `count`, `find`, `map` and `filter`
//! (spec 2.2): the list loop over its items, in insertion order
//! (`lower_lambda.rs`); `map` and `filter` put the list made into a set.

use crate::ir::{self, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

/// The set methods that walk it with a function.
pub(crate) const SET_LOOPS: [&str; 8] = [
    "any", "all", "sum", "count", "find", "map", "filter", "for_each",
];

impl<'a> Lowerer<'a> {
    /// `s.method(f)` on set `set`, of `item`s, giving `ty`.
    pub(crate) fn set_loop(
        &mut self,
        method: &str,
        set: ir::Expr,
        item: Type,
        f: &'a ast::Expr,
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let items_ty = self.list_type(item);
        let items = self.at(K::Rt(RtOp::MapKeys, vec![set]), items_ty, line);
        if !matches!(method, "map" | "filter") {
            return self.list_loop(method, items, f, ty, span);
        }
        let Some(Shape::Set(out_item)) = self.shapes.shape(ty).cloned() else {
            return Err(self.unsupported(format!("`{method}` here"), span));
        };
        let made_ty = self.list_type(out_item);
        let made = self.list_loop(method, items, f, made_ty, span)?;
        // Each item of the list made into a new set, which keeps the first.
        let (held, out) = (self.new_local(made.ty), self.new_local(ty));
        let (var, index) = (self.new_local(out_item), self.new_local(Type::I64));
        let insert = self.at(
            K::Rt(
                RtOp::MapInsert,
                vec![
                    self.local_expr(out, line),
                    self.local_expr(var, line),
                    self.nothing(line),
                ],
            ),
            ty,
            line,
        );
        let body = Block {
            stmts: vec![Stmt::Expr(self.at(
                K::Assign(out, Box::new(insert)),
                Type::Unit,
                line,
            ))],
            value: None,
        };
        let block = Block {
            stmts: vec![
                Stmt::Let(held, made),
                Stmt::Let(out, self.at(K::Map(Vec::new()), ty, line)),
                Stmt::ForEach {
                    var,
                    index,
                    list: held,
                    body,
                },
            ],
            value: Some(Box::new(self.local_expr(out, line))),
        };
        Ok(self.at(K::Block(block), ty, line))
    }
}
