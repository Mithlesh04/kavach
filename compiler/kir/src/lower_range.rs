//! A range as the list of its numbers (spec 2.2): `(0..n).map(f)` reads
//! the list `[0, 1, …, n - 1]`, which a loop builds. The checker types a
//! range a list method is called on as that list.

use crate::ir::{self, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// The list `lo..hi` (`lo..=hi` when `inclusive`) of type `ty`.
    pub(crate) fn range_list(
        &mut self,
        lo: &'a ast::Expr,
        hi: &'a ast::Expr,
        inclusive: bool,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let Some(Shape::List(item)) = self.shapes.shape(ty).cloned() else {
            return Err(crate::Unsupported {
                feature: "this range".into(),
                line,
            });
        };
        let (start, end) = (self.expr(lo)?, self.expr(hi)?);
        let (start, end) = (self.coerce(start, item)?, self.coerce(end, item)?);
        let (acc, var) = (self.new_local(ty), self.new_local(item));
        let empty = self.at(K::List(Vec::new()), ty, line);
        let round = self.push_to(acc, self.local_expr(var, line));
        let build = Stmt::ForRange {
            var,
            start,
            end,
            inclusive,
            body: round,
        };
        Ok(self.at(
            K::Block(Block {
                stmts: vec![Stmt::Let(acc, empty), build],
                value: Some(Box::new(self.local_expr(acc, line))),
            }),
            ty,
            line,
        ))
    }
}
