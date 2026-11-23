// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Whole numbers of a `bits` type wider than 64 (spec 2.1):
//! `Int<bits: 128>`, `UInt<bits: 256>`. Their values are
//! `Value::Wide`, a `BigInt` with its type; each operation goes through
//! `kavach_runtime_core::wide`, as native code's do, so both say the same.

use crate::interp::Interp;
use crate::testing::values::Rng;
use crate::value::{Eval, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use kavach_runtime_core::BigInt;
use kavach_runtime_core::json::Tree;
use kavach_runtime_core::wide::Wide;
use std::rc::Rc;

/// The width and sign of wide type `p`.
pub(crate) fn wide_of(p: Prim) -> Wide {
    match p {
        Prim::Bits(bits, signed) => Wide { bits, signed },
        _ => Wide {
            bits: 64,
            signed: true,
        },
    }
}

/// `p(v)`: `v` as a value of wide type `p`, cut to a whole number as
/// `Int(v)` does; `None` when it doesn't fit.
pub(crate) fn convert<'a>(v: &Value<'a>, p: Prim) -> Option<Value<'a>> {
    let n = crate::bigint::to_big(v)?;
    wide_of(p).fits(&n).then(|| Value::Wide(Rc::new(n), p))
}

/// A whole number read from JSON as wide type `p`; `None` when it isn't
/// one or doesn't fit.
pub(crate) fn from_tree<'a>(t: &Tree, p: Prim) -> Option<Value<'a>> {
    let n: BigInt = match t {
        Tree::Int(n) => BigInt::from(*n),
        Tree::Number(text) => text.parse().ok()?,
        _ => return None,
    };
    wide_of(p).fits(&n).then(|| Value::Wide(Rc::new(n), p))
}

/// Text read as wide type `p` (`UInt<bits: 128>("12")`, spec 2.1).
pub(crate) fn from_text<'a>(text: &str, p: Prim) -> Option<Value<'a>> {
    let n: BigInt = text.trim().parse().ok()?;
    wide_of(p).fits(&n).then(|| Value::Wide(Rc::new(n), p))
}

/// A generated value of wide type `p` (spec 11.3): edge `edge` (0, 1,
/// -1, the smallest, the largest and those next to `near`, each when it
/// fits), or a random one.
pub(crate) fn generate<'a>(
    p: Prim,
    edge: Option<usize>,
    near: &[i128],
    rng: &mut Rng,
) -> Value<'a> {
    let w = wide_of(p);
    let (lo, hi) = w.limits();
    let mut edges: Vec<BigInt> = vec![0.into(), 1.into(), (-1).into(), lo.clone(), hi.clone()];
    edges.extend(
        near.iter()
            .flat_map(|n| [n - 1, *n, n + 1])
            .map(BigInt::from),
    );
    edges.retain(|n| w.fits(n));
    let n = match edge {
        Some(i) => edges[i % edges.len()].clone(),
        // Small numbers most often, then any of the type's bits.
        None if rng.below(2) == 0 => BigInt::from(rng.range(0, 1000)),
        None => {
            let mut n = BigInt::from(0);
            for _ in 0..w.bits.div_ceil(64) {
                n = (n << 64usize) + BigInt::from(rng.next());
            }
            let span = &hi - &lo + 1;
            lo + n % span
        }
    };
    Value::Wide(Rc::new(n), p)
}

impl<'a> Interp<'a> {
    /// `a op b` in wide type `p`, `op` as in `kavach_kir::WHOLE_OPS`:
    /// a result that doesn't fit panics, or is `none` for `checked_*`.
    pub(crate) fn wide_op(
        &self,
        op: &str,
        a: &BigInt,
        b: &BigInt,
        p: Prim,
        span: Span,
    ) -> Eval<'a> {
        match wide_of(p).op(op, a, b) {
            Ok(Some(n)) => Ok(Value::Wide(Rc::new(n), p)),
            Ok(None) => Ok(Value::None),
            Err(why) => Err(self.panic(why, span)),
        }
    }
}
