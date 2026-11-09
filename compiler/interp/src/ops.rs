//! Operators. Integer overflow and division by zero are panics (spec 5.6).

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::{BinaryOp, Expr, Span, UnaryOp};
use kavach_checker::Prim;
use std::cmp::Ordering;

/// The values an integer type can hold.
pub(crate) fn int_range(p: Prim) -> (i128, i128) {
    match p {
        Prim::Int8 => (i8::MIN as i128, i8::MAX as i128),
        Prim::Int16 => (i16::MIN as i128, i16::MAX as i128),
        Prim::Int32 => (i32::MIN as i128, i32::MAX as i128),
        Prim::UInt8 => (0, u8::MAX as i128),
        Prim::UInt16 => (0, u16::MAX as i128),
        Prim::UInt32 => (0, u32::MAX as i128),
        Prim::UInt64 => (0, u64::MAX as i128),
        _ => (i64::MIN as i128, i64::MAX as i128),
    }
}

/// How many bits `p` has.
fn width(p: Prim) -> u32 {
    let (lo, hi) = int_range(p);
    (hi + 1).trailing_zeros() + u32::from(lo < 0)
}

/// `n` if it fits in `p`.
pub(crate) fn fits(n: i128, p: Prim) -> Option<i128> {
    let (lo, hi) = int_range(p);
    (lo..=hi).contains(&n).then_some(n)
}

impl<'a> Interp<'a> {
    pub(crate) fn unary(&mut self, op: UnaryOp, v: Value<'a>, span: Span) -> Eval<'a> {
        Ok(match (op, v) {
            (UnaryOp::Not, Value::Bool(b)) => Value::Bool(!b),
            (UnaryOp::Neg, Value::Int(n, p)) => match fits(-n, p) {
                Some(n) => Value::Int(n, p),
                None => {
                    return Err(self.panic(
                        format!("integer overflow: -({n}) doesn't fit in {}", p.name()),
                        span,
                    ));
                }
            },
            (UnaryOp::Neg, Value::Float(f)) => Value::Float(-f),
            (UnaryOp::Neg, Value::Float32(f)) => Value::Float32(-f),
            (UnaryOp::Neg, v @ (Value::Decimal(_) | Value::BigDecimal(_))) => {
                let d = crate::bigint::any_decimal(&v).expect("a decimal");
                crate::bigint::decimal_value(d.neg())
            }
            (UnaryOp::Neg, Value::BigInt(n)) => Value::BigInt(std::rc::Rc::new(-&*n)),
            (UnaryOp::Deref, _) => {
                return Err(self.panic(
                    "`*p` reads raw memory, which runs in native code only: run with `kavach run --native` (spec 6.4)",
                    span,
                ));
            }
            (UnaryOp::BitNot, Value::Int(n, p)) => {
                Value::Int(fits(!n, p).unwrap_or(!n & int_range(p).1), p)
            }
            (_, other) => {
                return Err(self.panic(
                    format!("can't apply this operator to {}", other.display()),
                    span,
                ));
            }
        })
    }

    pub(crate) fn binary(
        &mut self,
        op: BinaryOp,
        l: &'a Expr,
        r: &'a Expr,
        span: Span,
    ) -> Eval<'a> {
        use BinaryOp::*;
        // `and`, `or` and `??` only evaluate the right side when needed.
        match op {
            And => {
                return Ok(Value::Bool(
                    self.eval(l)? == Value::Bool(true) && self.eval(r)? == Value::Bool(true),
                ));
            }
            Or => {
                return Ok(Value::Bool(
                    self.eval(l)? == Value::Bool(true) || self.eval(r)? == Value::Bool(true),
                ));
            }
            Default => {
                return match self.eval(l)? {
                    Value::None => self.eval(r),
                    v => Ok(v),
                };
            }
            _ => {}
        }
        let (a, b) = (self.eval(l)?, self.eval(r)?);
        match op {
            Eq | NotEq | Lt | Le | Gt | Ge => self.compared(op, &a, &b, span).map(Value::Bool),
            Range | RangeInclusive => match (a, b) {
                (Value::Int(x, p), Value::Int(y, _)) => Ok(Value::Range(
                    x,
                    if op == RangeInclusive { y + 1 } else { y },
                    p,
                )),
                _ => Err(self.panic("ranges need integers", span)),
            },
            _ => self.arith(op, a, b, span),
        }
    }

    /// Whether `a op b` holds, for a comparison `op`.
    fn compared(
        &self,
        op: BinaryOp,
        a: &Value<'a>,
        b: &Value<'a>,
        span: Span,
    ) -> Result<bool, crate::Flow<'a>> {
        use BinaryOp::*;
        Ok(match op {
            Eq => a == b,
            NotEq => a != b,
            _ => {
                let ord = compare(a, b)
                    .ok_or_else(|| self.panic("these values can't be compared", span))?;
                match op {
                    Lt => ord == Ordering::Less,
                    Le => ord != Ordering::Greater,
                    Gt => ord == Ordering::Greater,
                    _ => ord != Ordering::Less,
                }
            }
        })
    }

    /// `a < b <= c` (spec 1.8): each operand once, left to right, stopping
    /// at the first pair that doesn't hold.
    pub(crate) fn chain(
        &mut self,
        first: &'a Expr,
        links: &'a [(BinaryOp, Expr)],
        span: Span,
    ) -> Eval<'a> {
        let mut left = self.eval(first)?;
        for (op, x) in links {
            let right = self.eval(x)?;
            if !self.compared(*op, &left, &right, span)? {
                return Ok(Value::Bool(false));
            }
            left = right;
        }
        Ok(Value::Bool(true))
    }

    pub(crate) fn arith(
        &mut self,
        op: BinaryOp,
        a: Value<'a>,
        b: Value<'a>,
        span: Span,
    ) -> Eval<'a> {
        use BinaryOp::*;
        let zero = || "division by zero".to_string();
        match (a, b) {
            (Value::Int(x, p), Value::Int(y, _)) => {
                let result = match op {
                    Add => x.checked_add(y),
                    Sub => x.checked_sub(y),
                    Mul => x.checked_mul(y),
                    Div | Rem if y == 0 => return Err(self.panic(zero(), span)),
                    Div => x.checked_div(y),
                    Rem => x.checked_rem(y),
                    Pow if y < 0 => {
                        return Err(self.panic(
                            format!("{x} ** {y}: a whole number to a negative power isn't whole; use floats"),
                            span,
                        ));
                    }
                    Pow => kavach_runtime_core::int_pow(x, y),
                    BitAnd => Some(x & y),
                    BitOr => Some(x | y),
                    BitXor => Some(x ^ y),
                    // By 0 up to the width less one (spec 1); `x` has at
                    // most 64 bits, so `x << s` can't lose any in an i128.
                    Shl | Shr => match u32::try_from(y) {
                        Ok(s) if s < width(p) && op == Shl => Some(x << s),
                        Ok(s) if s < width(p) => Some(x >> s),
                        _ => None,
                    },
                    _ => None,
                };
                match result.and_then(|n| fits(n, p)) {
                    Some(n) => Ok(Value::Int(n, p)),
                    None => Err(self.panic(
                        format!(
                            "integer overflow: {x} {} {y} doesn't fit in {}",
                            op.as_str(),
                            p.name()
                        ),
                        span,
                    )),
                }
            }
            (Value::Float32(x), Value::Float32(y)) => Ok(Value::Float32(match op {
                Add => x + y,
                Sub => x - y,
                Mul => x * y,
                Div => x / y,
                // As a `Float`, then rounded once: native code does the same.
                Pow => f64::from(x).powf(f64::from(y)) as f32,
                _ => x % y,
            })),
            (Value::Float(x), Value::Float(y)) => Ok(Value::Float(match op {
                Add => x + y,
                Sub => x - y,
                Mul => x * y,
                Div => x / y,
                Pow => x.powf(y),
                _ => x % y,
            })),
            (Value::BigInt(x), Value::BigInt(y)) => self.big_arith(op, &x, &y, span),
            (a, b) if crate::bigint::any_decimal(&a).is_some() => {
                match (
                    crate::bigint::any_decimal(&a),
                    crate::bigint::any_decimal(&b),
                ) {
                    (Some(x), Some(y)) => self.decimal_arith(op, x, y, span),
                    _ => Err(self.panic("can't apply this operator to these values", span)),
                }
            }
            (Value::Str(x), Value::Str(y)) if op == Add => Ok(Value::Str(format!("{x}{y}").into())),
            (Value::Bytes(x), Value::Bytes(y)) if op == Add => {
                Ok(Value::Bytes([&x[..], &y[..]].concat().into()))
            }
            (x, y) => Err(self.panic(
                format!(
                    "can't use `{}` on {} and {}",
                    op.as_str(),
                    x.display(),
                    y.display()
                ),
                span,
            )),
        }
    }
}

/// The order items of a list sort in: as `compare`, with `NaN` after
/// every number, so a list holding one still sorts (spec 14.2).
pub(crate) fn list_order(a: &Value<'_>, b: &Value<'_>) -> Ordering {
    match (a, b) {
        (Value::Float32(x), Value::Float32(y)) if x.is_nan() || y.is_nan() => {
            x.is_nan().cmp(&y.is_nan())
        }
        (Value::Float(x), Value::Float(y)) if x.is_nan() || y.is_nan() => {
            x.is_nan().cmp(&y.is_nan())
        }
        _ => compare(a, b).unwrap_or(Ordering::Equal),
    }
}

pub(crate) fn compare(a: &Value<'_>, b: &Value<'_>) -> Option<Ordering> {
    match (a, b) {
        (Value::Int(x, _), Value::Int(y, _)) => Some(x.cmp(y)),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y),
        (Value::Float32(x), Value::Float32(y)) => x.partial_cmp(y),
        (Value::Decimal(x), Value::Decimal(y)) => x.partial_cmp(y),
        (Value::Decimal(_) | Value::BigDecimal(_), Value::Decimal(_) | Value::BigDecimal(_)) => {
            Some(crate::bigint::any_decimal(a)?.compare(&crate::bigint::any_decimal(b)?))
        }
        (Value::BigInt(x), Value::BigInt(y)) => Some(x.cmp(y)),
        (Value::Str(x), Value::Str(y)) => Some(x.cmp(y)),
        (Value::Char(x), Value::Char(y)) => Some(x.cmp(y)),
        // `false` before `true`; tuples item by item (spec 1.8).
        (Value::Bool(x), Value::Bool(y)) => Some(x.cmp(y)),
        (Value::Tuple(xs), Value::Tuple(ys)) => {
            for (x, y) in xs.iter().zip(ys) {
                match list_order(x, y) {
                    Ordering::Equal => {}
                    other => return Some(other),
                }
            }
            Some(xs.len().cmp(&ys.len()))
        }
        _ => None,
    }
}
