//! `BigInt`, whole numbers of any size (spec 2.1, decision 0041): the
//! constants the checker worked out, arithmetic, and conversions.

use crate::interp::Interp;
use crate::ops::fits;
use crate::value::{Eval, Value};
use kavach_ast::{BinaryOp, Expr, ExprKind, Span, UnaryOp};
use kavach_checker::Prim;
use kavach_runtime_core::BigInt;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// The value of `e` when the checker worked it out: a `BigInt`
    /// constant, or a float constant rounded once (decision 0043).
    pub(crate) fn big_constant(&self, e: &Expr) -> Option<Value<'a>> {
        if self.c.big_constants.is_empty() && self.c.float_constants.is_empty()
            || !matches!(
                e.kind,
                ExprKind::Int(_)
                    | ExprKind::BigInt(_)
                    | ExprKind::Binary(..)
                    | ExprKind::Unary(UnaryOp::Neg, _)
            )
        {
            return None;
        }
        let span = (e.span.start, e.span.end);
        if let Some(f) = self.c.float_constants.get(&span) {
            return Some(Value::Float(*f));
        }
        let digits = self.c.big_constants.get(&span)?;
        Some(Value::BigInt(Rc::new(digits.parse().ok()?)))
    }

    /// `x op y` on `BigInt`s: `+ - * / % **`; panics as `Int` does on a
    /// division by zero.
    pub(crate) fn big_arith(&self, op: BinaryOp, x: &BigInt, y: &BigInt, span: Span) -> Eval<'a> {
        let zero = BigInt::default();
        let n = match op {
            BinaryOp::Add => x + y,
            BinaryOp::Sub => x - y,
            BinaryOp::Mul => x * y,
            BinaryOp::Div | BinaryOp::Rem if *y == zero => {
                return Err(self.panic("division by zero", span));
            }
            BinaryOp::Div => x / y,
            BinaryOp::Rem => x % y,
            BinaryOp::Pow => kavach_runtime_core::big_pow(x, y).map_err(|m| self.panic(m, span))?,
            _ => return Err(self.panic("can't apply this operator to a `BigInt`", span)),
        };
        Ok(Value::BigInt(Rc::new(n)))
    }
}

/// `BigInt(v)`: from any number, cut to a whole one as `Int(v)` does.
pub(crate) fn to_big(v: &Value<'_>) -> Option<BigInt> {
    match v {
        Value::Int(n, _) => Some(BigInt::from(*n)),
        Value::BigInt(n) => Some((**n).clone()),
        Value::Decimal(_) | Value::BigDecimal(_) => Some(any_decimal(v)?.trunc()),
        Value::Float(f) if f.is_finite() => format!("{:.0}", f.trunc()).parse().ok(),
        _ => None,
    }
}

/// `Int(n)`, `Float(n)`, `Decimal(n)` and friends for a `BigInt` `n`:
/// `None` when it doesn't fit.
pub(crate) fn from_big<'a>(n: &BigInt, p: Prim) -> Option<Value<'a>> {
    match p {
        Prim::Float32 => n.to_string().parse().ok().map(Value::Float32),
        Prim::Float => n.to_string().parse().ok().map(Value::Float),
        Prim::Decimal => Some(decimal_value(kavach_runtime_core::AnyDecimal::parse(
            &n.to_string(),
        )?)),
        p => fits(i128::try_from(n).ok()?, p).map(|n| Value::Int(n, p)),
    }
}

/// A `Decimal` value in either of its forms (decision 0042).
pub(crate) fn any_decimal(v: &Value<'_>) -> Option<kavach_runtime_core::AnyDecimal> {
    use kavach_runtime_core::AnyDecimal;
    match v {
        Value::Decimal(d) => Some(AnyDecimal::Small(*d)),
        Value::BigDecimal(b) => Some(AnyDecimal::Big((**b).clone())),
        _ => None,
    }
}

/// The value of decimal `d`, in its own form.
pub(crate) fn decimal_value<'a>(d: kavach_runtime_core::AnyDecimal) -> Value<'a> {
    match d {
        kavach_runtime_core::AnyDecimal::Small(d) => Value::Decimal(d),
        kavach_runtime_core::AnyDecimal::Big(b) => Value::BigDecimal(Rc::new(b)),
    }
}

impl<'a> Interp<'a> {
    /// `x op y` on decimals, exact at any size (spec 2.1).
    pub(crate) fn decimal_arith(
        &self,
        op: BinaryOp,
        x: kavach_runtime_core::AnyDecimal,
        y: kavach_runtime_core::AnyDecimal,
        span: Span,
    ) -> Eval<'a> {
        let d = match op {
            BinaryOp::Add => x.add(&y),
            BinaryOp::Sub => x.sub(&y),
            BinaryOp::Mul => x.mul(&y),
            BinaryOp::Div => x
                .div(&y)
                .ok_or_else(|| self.panic("division by zero", span))?,
            _ => {
                return Err(self.panic(
                    format!(
                        "`{}` works on whole numbers and floats, not on `Decimal`",
                        op.as_str()
                    ),
                    span,
                ));
            }
        };
        Ok(decimal_value(d))
    }
}
