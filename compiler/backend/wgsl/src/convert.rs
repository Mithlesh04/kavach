//! Conversions in a kernel, checked as the CPU checks them, and the
//! `std.math` functions a `Float32` power or root arrives as.

use crate::expr::{Body, Writer};
use kavach_kir::{Expr, ExprKind as K, RtOp, Type};

impl<'p> Body<'p> {
    /// `x` converted to `to`, checked as the CPU checks it. A `Float32`
    /// power arrives as `Float32(Float(a) ** Float(b))` (`lower_power.rs`).
    pub(crate) fn convert(&mut self, w: &mut Writer<'p>, x: &Expr, to: Type) -> Option<String> {
        if let (Type::F32, K::Rt(op, args)) = (to, &x.kind) {
            let name = match op {
                RtOp::MathPow => "kpow",
                RtOp::MathSqrt => "sqrt",
                RtOp::MathFloor => "floor",
                RtOp::MathCeil => "ceil",
                RtOp::MathRound => "kround",
                RtOp::MathSin => "sin",
                RtOp::MathCos => "cos",
                RtOp::MathTan => "tan",
                RtOp::MathAsin => "asin",
                RtOp::MathAcos => "acos",
                RtOp::MathAtan => "atan",
                RtOp::MathAtan2 => "atan2",
                RtOp::MathExp => "exp",
                RtOp::MathLn => "log",
                _ => return None,
            };
            let mut vals = Vec::new();
            for a in args {
                let K::Convert(inner, _) = &a.kind else {
                    return None;
                };
                if inner.ty != Type::F32 {
                    return None;
                }
                vals.push(self.expr(w, inner)?);
            }
            return Some(format!("{name}({})", vals.join(", ")));
        }
        let v = self.expr(w, x)?;
        Some(match (x.ty, to) {
            (from, to) if from == to => v,
            (Type::I32 | Type::U32, Type::F32) => format!("f32({v})"),
            (Type::F32, Type::I32) => format!("f_to_i({v}, {}u)", x.line),
            (Type::F32, Type::U32) => format!("f_to_u({v}, {}u)", x.line),
            (Type::I32, Type::U32) => format!("i_to_u({v}, {}u)", x.line),
            (Type::U32, Type::I32) => format!("u_to_i({v}, {}u)", x.line),
            _ => return None,
        })
    }
}
