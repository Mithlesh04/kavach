// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The values agent ops are given, read for an agent's checks (spec 8.3):
//! a capability call shown for the audit log, the one item of a tuple,
//! and the numbers a limit adds up.

use crate::display::{decimal_of, display};
use crate::heap::{Data, alloc, obj, release, slots};
use crate::shapes::{shape, with_str};
use kavach_kir::{Shape, Type};
use kavach_runtime_core::agents::Num;

/// `Payments.refund(order_id: 1042, amount: 1499)`, from the arguments in
/// tuple `args` (borrowed); secret ones are written `<secret>` (spec 8.7).
pub(crate) fn shown(call: &str, params: &[(String, bool)], args: i64) -> String {
    if call == "Ai.ask" {
        // The prompt's start, as the interpreter writes it.
        let prompt = with_str(slots(args)[0] as i64, |s| {
            s.chars().take(60).collect::<String>()
        });
        return format!("ask(\"{prompt}\")");
    }
    let types = match shape(obj(args).ty()) {
        Shape::Tuple(ts) => ts.clone(),
        _ => Vec::new(),
    };
    let values = slots(args);
    let shown: Vec<String> = params
        .iter()
        .zip(types.iter().zip(values))
        .map(|((name, secret), (t, v))| match secret {
            true => format!("{name}: <secret>"),
            false => format!("{name}: {}", display(*v as i64, *t, false)),
        })
        .collect();
    format!("{call}({})", shown.join(", "))
}

/// The value in one-item tuple `t` (borrowed) and its type; `None` for
/// nothing (`Unit`).
pub(crate) fn first(t: i64) -> Option<(i64, Type)> {
    if t == 0 {
        return None;
    }
    let ty = match shape(obj(t).ty()) {
        Shape::Tuple(ts) => ts.first().copied()?,
        _ => return None,
    };
    (ty != Type::Unit).then(|| (slots(t)[0] as i64, ty))
}

/// The number in slot `x` (owned) of type `ty`.
pub(crate) fn num_of(x: i64, ty: Type) -> Num {
    match ty {
        Type::F64 => Num::Float(f64::from_bits(x as u64)),
        Type::F32 => Num::Float(f64::from(crate::display::f32_of(x))),
        Type::Ref(_) => {
            let d = decimal_of(x);
            release(x);
            Num::Decimal(d)
        }
        _ => Num::Int(x),
    }
}

/// A slot of type `ty` holding `n`.
pub(crate) fn slot_of(n: &Num, ty: Type) -> i64 {
    match n {
        Num::Int(i) => *i,
        Num::Float(f) => f.to_bits() as i64,
        Num::Decimal(d) => alloc(ty, Data::Decimal(Box::new(*d))),
    }
}
