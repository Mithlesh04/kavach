// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `xs.group_by(key).map_values(g => g.sum(f))` (spec 14.2, decision
//! 0135): one pass over the items, a running sum for each key, with no
//! groups made, as native code runs it (`compiler/kir/src/grouped_sums.rs`).
//! The checker's `grouped_sum` says which calls are one.

use crate::interp::Interp;
use crate::map::MapValue;
use crate::value::{Eval, Value};
use kavach_ast::{BinaryOp, Expr};
use kavach_checker::{Prim, Ty};
use kavach_runtime_core::math::exact::ExactSum;
use std::rc::Rc;

/// One key's sum so far.
enum Running<'a> {
    Exact(Box<ExactSum>),
    Total(Value<'a>),
}

impl<'a> Interp<'a> {
    /// `e` worked out as a sum per key, or `None` when it isn't one.
    pub(crate) fn grouped_sum(&mut self, e: &'a Expr) -> Option<Eval<'a>> {
        let parts = kavach_checker::grouped_sum(e, &self.c.expr_types, &self.c.resolution)?;
        let sum_span = parts.sum.span;
        let float = match self.c.expr_types.get(&(sum_span.start, sum_span.end)) {
            Some(Ty::Prim(p @ (Prim::Float | Prim::Float32))) => Some(*p),
            _ => None,
        };
        Some(self.sum_per_key(parts.items, parts.key, parts.value, float, sum_span))
    }

    fn sum_per_key(
        &mut self,
        items: &'a Expr,
        key: &'a Expr,
        value: &'a Expr,
        float: Option<Prim>,
        span: kavach_ast::Span,
    ) -> Eval<'a> {
        let items = self.eval(items)?;
        let (key, value) = (self.eval(key)?, self.eval(value)?);
        // A table's rows are its field `kept` (`stdlib/table/table.kav`).
        let items = match items {
            Value::Record { fields, .. } => (fields.into_iter())
                .find(|f| &*f.name == "kept")
                .map_or(Value::Unit, |f| f.value),
            v => v,
        };
        let Value::List(items) = items else {
            return Err(self.panic("`group_by` here needs a list", span));
        };
        // Each key's place in `sums`, in the order first seen.
        let mut places = MapValue::default();
        let mut sums: Vec<Running<'a>> = Vec::new();
        for item in items.iter() {
            let k = self.call_value(key.clone(), vec![item.clone()], span)?;
            let v = self.call_value(value.clone(), vec![item.clone()], span)?;
            let place = places.slot(k);
            let at = match place {
                Value::Int(i, _) => *i as usize,
                _ => {
                    *place = Value::Int(sums.len() as i128, Prim::Int);
                    sums.push(match float {
                        Some(_) => Running::Exact(Box::new(ExactSum::compact())),
                        None => Running::Total(Value::Unit),
                    });
                    sums.len() - 1
                }
            };
            match (&mut sums[at], v) {
                (Running::Exact(s), Value::Float(x)) => s.add(x),
                (Running::Exact(s), Value::Float32(x)) => s.add(f64::from(x)),
                (Running::Total(total), v) => {
                    *total = match std::mem::replace(total, Value::Unit) {
                        Value::Unit => v,
                        before => self.arith(BinaryOp::Add, before, v, span)?,
                    };
                }
                _ => {}
            }
        }
        let mut out = MapValue::default();
        for ((k, _), sum) in places.entries.into_iter().zip(sums) {
            let v = match (sum, float) {
                (Running::Exact(s), Some(Prim::Float32)) => Value::Float32(s.value32()),
                (Running::Exact(s), _) => Value::Float(s.value()),
                (Running::Total(v), _) => v,
            };
            out.insert(k, v);
        }
        Ok(Value::Map(Rc::new(out)))
    }
}
