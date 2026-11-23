// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Conversions between built-in types, one form for all: `T(value)`
//! (spec 2.1). A number that doesn't fit its new type panics, or under
//! `try` and `catch` fails with a `SizeError`.

use crate::interp::Interp;
use crate::ops::fits;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use kavach_runtime_core::Decimal;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// `Float(n)`, `Int(x)`, `Decimal(n)`, `String(v)`: explicit conversions (spec 2.1).
    pub(crate) fn convert(&mut self, name: &str, v: Value<'a>, span: Span) -> Eval<'a> {
        let Some(target) = self.int_type_target(name, span).or(Prim::from_name(name)) else {
            return Ok(Value::Unit);
        };
        let out = match (target, &v) {
            (Prim::String, v) => {
                let text = self.shown(v, true, span)?;
                Some(self.str_value(&text))
            }
            (p, Value::Str(s)) => return self.read_text(p, s),
            (Prim::Char, Value::Int(n, _)) => match u32::try_from(*n).ok().and_then(char::from_u32)
            {
                Some(c) => Some(Value::Char(c)),
                None => {
                    return Err(self.panic(format!("{n} is not a character's code point"), span));
                }
            },
            (p, Value::Char(c)) => fits(*c as i128, p).map(|n| Value::Int(n, p)),
            (Prim::Float32, Value::Int(n, _)) => Some(Value::Float32(*n as f32)),
            (Prim::Float32, Value::Float(f)) => Some(Value::Float32(*f as f32)),
            (Prim::Float32, Value::Float32(f)) => Some(Value::Float32(*f)),
            (Prim::Float32, Value::Decimal(d)) => d.to_string().parse().ok().map(Value::Float32),
            (Prim::Float, Value::Float32(f)) => Some(Value::Float(f64::from(*f))),
            // From `Float32` to anything else: as from the same `Float`.
            (_, Value::Float32(f)) => return self.convert(name, Value::Float(f64::from(*f)), span),
            (Prim::Float, Value::Int(n, _)) => Some(Value::Float(*n as f64)),
            (Prim::Float, Value::Decimal(d)) => Some(Value::Float(d.to_f64())),
            (Prim::Float, Value::Float(f)) => Some(Value::Float(*f)),
            (Prim::Decimal, Value::Int(n, _)) => Some(Value::Decimal(Decimal::from_int(*n))),
            (Prim::Decimal, Value::Float(f)) => Decimal::parse(&f.to_string()).map(Value::Decimal),
            (Prim::Decimal, Value::Decimal(d)) => Some(Value::Decimal(*d)),
            (Prim::Decimal, Value::BigDecimal(_)) => Some(v.clone()),
            (Prim::Float | Prim::Float32, Value::BigDecimal(_)) => {
                crate::bigint::any_decimal(&v).map(|d| Value::Float(d.to_f64()))
            }
            (p, Value::BigDecimal(_)) if p != Prim::BigInt => {
                crate::bigint::any_decimal(&v).and_then(|d| crate::bigint::from_big(&d.trunc(), p))
            }
            (Prim::BigInt, v) => crate::bigint::to_big(v).map(|n| Value::BigInt(Rc::new(n))),
            (p, v) if p.is_wide() => crate::wide::convert(v, p),
            (p, Value::BigInt(n) | Value::Wide(n, _)) => crate::bigint::from_big(n, p),
            (p, Value::Int(n, _)) => fits(*n, p).map(|n| Value::Int(n, p)),
            (p, Value::Float(f)) if f.is_finite() => {
                fits(f.trunc() as i128, p).map(|n| Value::Int(n, p))
            }
            (p, Value::Decimal(d)) => fits(d.trunc(), p).map(|n| Value::Int(n, p)),
            _ => None,
        };
        let why = || format!("{} doesn't fit in {}", v.display(), target.name());
        match out {
            Some(out) => Ok(out),
            // `try Int8(n)`, `try UInt<bits: 12>(n)` (spec 2.1).
            None if self.checked_ctor == Some(span) && target.is_int() => {
                Err(Flow::Fail(Value::BuiltinError(why().into())))
            }
            None => Err(self.panic(why(), span)),
        }
    }
}
