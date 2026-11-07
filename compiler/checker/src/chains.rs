//! Chained comparisons (spec 1.8, decision 0040): `a < b <= c` compares
//! each pair of neighbours, so every operand has the type of the first.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;

impl Checker<'_> {
    pub(crate) fn chain(&mut self, first: &Expr, links: &[(BinaryOp, Expr)]) -> Ty {
        let t = self.value(first, None);
        let mut left = first;
        for (op, x) in links {
            self.check_expect(x, &t);
            self.check_compare(*op, left, x, &t);
            left = x;
        }
        Ty::Prim(Prim::Bool)
    }
}
