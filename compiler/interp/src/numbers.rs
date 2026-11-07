//! Methods of numbers (spec 14.2): `abs`, `near`, `bits`, `min`, `max`,
//! `clamp`, and `div_floor`, `mod`, `round`, `round_even` (decision 0051).

use crate::interp::Interp;
use crate::value::{Flow, Value};
use kavach_ast::Span;
use kavach_checker::Prim;

impl<'a> Interp<'a> {
    /// A method of a number, or `None` if `recv` and `name` aren't one.
    pub(crate) fn number_method(
        &mut self,
        recv: &Value<'a>,
        name: &str,
        args: &[Value<'a>],
        span: Span,
    ) -> Result<Option<Value<'a>>, Flow<'a>> {
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Unit);
        let order = |a: &Value<'a>, b: &Value<'a>| {
            crate::ops::compare(a, b).unwrap_or(std::cmp::Ordering::Equal)
        };
        Ok(Some(match (recv, name) {
            (Value::Int(n, p), "abs") => {
                match n.checked_abs().and_then(|a| crate::ops::fits(a, *p)) {
                    Some(a) => Value::Int(a, *p),
                    None => {
                        return Err(self.panic(
                            format!("integer overflow: -({n}) doesn't fit in {}", p.name()),
                            span,
                        ));
                    }
                }
            }
            (Value::Float(f), "abs") => Value::Float(f.abs()),
            (Value::Float32(f), "abs") => Value::Float32(f.abs()),
            (Value::Float32(f), "near") => match arg(0) {
                Value::Float32(g) => Value::Bool(kavach_runtime_core::float32_near(*f, g)),
                _ => Value::Bool(false),
            },
            (Value::Float(f), "near") => match arg(0) {
                Value::Float(g) => Value::Bool(kavach_runtime_core::float_near(*f, g)),
                _ => Value::Bool(false),
            },
            (Value::Float(f), "bits") => Value::Int(i128::from(f.to_bits()), Prim::UInt64),
            // Rounded down, through `/` and `%`, so a 0 or an overflow
            // panics as they do (decision 0051).
            (Value::Int(..), "div_floor" | "mod") => {
                use kavach_ast::BinaryOp::{Div, Rem};
                let q = self.arith(Div, recv.clone(), arg(0), span)?;
                let r = self.arith(Rem, recv.clone(), arg(0), span)?;
                let (Value::Int(q, p), Value::Int(r, _), Value::Int(n, _)) = (q, r, arg(0)) else {
                    return Ok(None);
                };
                let adjust = r != 0 && (r < 0) != (n < 0);
                match (name, adjust) {
                    ("div_floor", true) => Value::Int(q - 1, p),
                    ("div_floor", false) => Value::Int(q, p),
                    (_, true) => Value::Int(r + n, p),
                    (_, false) => Value::Int(r, p),
                }
            }
            (Value::Decimal(_) | Value::BigDecimal(_), "round" | "round_even") => {
                let places = match arg(0) {
                    Value::Int(n, _) => n,
                    _ => 0,
                };
                if places < 0 {
                    return Err(self.panic(
                        format!("can't round to {places} places: give 0 or more"),
                        span,
                    ));
                }
                let d = crate::bigint::any_decimal(recv).expect("a decimal");
                let places = u32::try_from(places).unwrap_or(u32::MAX);
                crate::bigint::decimal_value(d.round(places, name == "round_even"))
            }
            (Value::Decimal(d), "abs") => Value::Decimal(if d.mantissa < 0 { d.neg() } else { *d }),
            (
                Value::Int(..) | Value::Float(_) | Value::Float32(_) | Value::Decimal(_),
                "min" | "max" | "clamp",
            ) => {
                let lower = |a: &Value<'a>, b: &Value<'a>| {
                    if order(a, b).is_le() {
                        a.clone()
                    } else {
                        b.clone()
                    }
                };
                let upper = |a: &Value<'a>, b: &Value<'a>| {
                    if order(a, b).is_ge() {
                        a.clone()
                    } else {
                        b.clone()
                    }
                };
                match name {
                    "min" => lower(recv, &arg(0)),
                    "max" => upper(recv, &arg(0)),
                    _ => lower(&upper(recv, &arg(0)), &arg(1)),
                }
            }
            _ => return Ok(None),
        }))
    }
}
