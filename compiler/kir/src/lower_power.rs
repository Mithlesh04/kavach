//! `a ** b` in native code (spec 1.8): floats through the op `math.pow`
//! uses, whole numbers through `IntPow`, which panics when the result
//! doesn't fit the type, as `*` does.

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::Lowerer;
use crate::rt_op::RtOp;

impl Lowerer<'_> {
    pub(crate) fn power(&self, l: ir::Expr, r: ir::Expr, line: u32) -> ir::Expr {
        let ty = l.ty;
        let at = |kind, ty| ir::Expr { kind, ty, line };
        // A `Float32` power is worked out as a `Float`, then rounded once:
        // the interpreter does the same.
        if ty == Type::F32 {
            let wide = |x: ir::Expr| at(K::Convert(Box::new(x), false), Type::F64);
            let pow = at(K::Rt(RtOp::MathPow, vec![wide(l), wide(r)]), Type::F64);
            return at(K::Convert(Box::new(pow), false), Type::F32);
        }
        let op = if ty == Type::F64 {
            RtOp::MathPow
        } else {
            RtOp::IntPow
        };
        at(K::Rt(op, vec![l, r]), ty)
    }
}
