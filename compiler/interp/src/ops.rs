// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Operators. Integer overflow and division by zero are panics (spec 5.6).

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::{BinaryOp, Expr, ExprKind, Span, UnaryOp};
use kavach_checker::Prim;
use std::cmp::Ordering;

/// The values an integer type can hold.
pub(crate) fn int_range(p: Prim) -> (i128, i128) {
    p.int_range()
        .unwrap_or((i64::MIN as i128, i64::MAX as i128))
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
    /// The type `UInt<bits: 12>(x)` or `Int<range: 1..=9>(x)` at `span`
    /// converts to (spec 2.1), from the checker; `None` for other calls.
    pub(crate) fn int_type_target(&self, name: &str, span: Span) -> Option<Prim> {
        if !matches!(name, "Int" | "UInt") {
            return None;
        }
        match self.c.expr_types.get(&(span.start, span.end))? {
            kavach_checker::Ty::Prim(p) => Some(*p),
            kavach_checker::Ty::Result(ok, _) => match **ok {
                kavach_checker::Ty::Prim(p) => Some(p),
                _ => None,
            },
            _ => None,
        }
    }

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
    pub(crate) fn compared(
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
            // `NaN` is in no order: `<` and the others are false (1.8).
            _ if is_nan(a) || is_nan(b) => false,
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
            (Value::Float32(x), Value::Float32(y)) => {
                let got = match op {
                    Add => x + y,
                    Sub => x - y,
                    Mul => x * y,
                    Div => x / y,
                    // As a `Float`, then rounded once: native code does the same.
                    Pow => kavach_runtime_core::math::pow(f64::from(x), f64::from(y)) as f32,
                    _ => x % y,
                };
                if got.is_nan() {
                    // `**` is worked out as a `Float`, so shown as one.
                    let narrow = op != Pow;
                    let (x, y) = (f64::from(x), f64::from(y));
                    let message =
                        kavach_runtime_core::math::nan_watch_op(x, op.as_str(), y, narrow);
                    self.watch_nan(message, span)?;
                }
                Ok(Value::Float32(got))
            }
            (Value::Float(x), Value::Float(y)) => {
                let got = match op {
                    Add => x + y,
                    Sub => x - y,
                    Mul => x * y,
                    Div => x / y,
                    Pow => kavach_runtime_core::math::pow(x, y),
                    _ => x % y,
                };
                if got.is_nan() {
                    let message = kavach_runtime_core::math::nan_watch_op(x, op.as_str(), y, false);
                    self.watch_nan(message, span)?;
                }
                Ok(Value::Float(got))
            }
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
            // Text nothing else holds grows in place (`s += part`, spec 6.1).
            (Value::Str(mut x), Value::Str(y)) if op == Add => {
                match std::rc::Rc::get_mut(&mut x) {
                    Some(text) => text.push_str(&y),
                    None => x = std::rc::Rc::new(format!("{x}{y}")),
                }
                Ok(Value::Str(x))
            }
            // `xs + ys` (spec 2.2): a new list; `xs += ys` grows `xs` in
            // place when nothing else holds it.
            (Value::List(mut x), Value::List(y)) if op == Add => {
                if x.limit.is_some() {
                    // A new list has no limit (spec 2.2).
                    let all: Vec<Value<'a>> = x.iter().chain(y.iter()).cloned().collect();
                    return Ok(Value::List(std::rc::Rc::new(all.into())));
                }
                std::rc::Rc::make_mut(&mut x).extend(y.iter().cloned());
                Ok(Value::List(x))
            }
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

/// Whether the program is watched for NaN (spec 5.9): `[run] watch_nan`
/// or `--watch-nan`, set as `main` starts.
pub(crate) static WATCH_NAN: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

impl<'a> Interp<'a> {
    /// The NaN watch's panic, for an operation at `span` that gave `NaN`:
    /// when the watch is on and `span` is the program's own code.
    pub(crate) fn watch_nan(
        &self,
        message: String,
        span: Span,
    ) -> Result<(), crate::value::Flow<'a>> {
        let on = WATCH_NAN.load(std::sync::atomic::Ordering::Relaxed);
        if on && !self.c.in_std(span.start) {
            return Err(self.panic(message, span));
        }
        Ok(())
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
        // Lists and rings item by item, the shorter first when one runs
        // out; `Bytes` byte by byte (spec 1.8).
        (Value::List(xs), Value::List(ys)) => {
            for (x, y) in xs.iter().zip(ys.iter()) {
                match list_order(x, y) {
                    Ordering::Equal => {}
                    other => return Some(other),
                }
            }
            Some(xs.len().cmp(&ys.len()))
        }
        (Value::Bytes(x), Value::Bytes(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

/// Whether `v` is a float that is `NaN`.
pub(crate) fn is_nan(v: &Value<'_>) -> bool {
    match v {
        Value::Float(f) => f.is_nan(),
        Value::Float32(f) => f.is_nan(),
        _ => false,
    }
}

impl<'a> Interp<'a> {
    /// `assert` (spec 11.8): a failing comparison shows both sides, as
    /// `assertion failed: total == 10 (left: 9, right: 10)`, each worked
    /// out once; other conditions show only their text.
    pub(crate) fn assert_stmt(&mut self, e: &'a Expr) -> Eval<'a> {
        use BinaryOp::*;
        let text = self.text(e.span);
        if let ExprKind::Binary(op @ (Eq | NotEq | Lt | Le | Gt | Ge), l, r) = &e.kind {
            let (a, b) = (self.eval(l)?, self.eval(r)?);
            if self.compared(*op, &a, &b, e.span)? {
                return Ok(Value::Unit);
            }
            let (a, b) = (a.display(), b.display());
            return Err(self.panic(
                format!("assertion failed: {text} (left: {a}, right: {b})"),
                e.span,
            ));
        }
        match self.eval(e)? {
            Value::Bool(true) => Ok(Value::Unit),
            _ => Err(self.panic(format!("assertion failed: {text}"), e.span)),
        }
    }
}
