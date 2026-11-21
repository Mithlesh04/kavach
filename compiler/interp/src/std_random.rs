// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Where random numbers come from (spec 14.15): `random_bits` for
//! `std.random`, and `random_ints` for `tensor.random_int` (14.33).

use crate::calls::Args;
use crate::interp::Interp;
use crate::value::Value;
use kavach_checker::Prim;
use kavach_runtime_net::random;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// 64 random bits: a `random.seeded` block's first, then the `Random`
    /// fake's under `kavach test` (11.5), else the system's.
    pub(crate) fn random_bits(&mut self) -> u64 {
        let seeded = kavach_runtime_core::fs_scope::seeded_bits(&self.fs_scopes);
        match (seeded, &mut self.random) {
            (Some(bits), _) => bits,
            (None, Some(fake)) => fake.next(),
            (None, None) => random::bits(),
        }
    }

    /// `random_ints(n, from, until)`: `n` whole numbers, each drawn as
    /// `random.int(from, until)` draws it, from the same bits.
    pub(crate) fn random_ints(&mut self, args: &Args<'a>) -> Value<'a> {
        let int = |i: usize| match args.get(i) {
            Some((_, Value::Int(v, _))) => *v as i64,
            _ => 0,
        };
        let (n, from, until) = (int(0).max(0) as usize, int(1), int(2));
        let seeded = self
            .fs_scopes
            .iter()
            .any(|s| matches!(s, kavach_runtime_core::fs_scope::Scope::Seeded(_)));
        let drawn = if seeded || self.random.is_some() {
            random::ints(n, from, until, || self.random_bits())
        } else {
            random::ints(n, from, until, random::many_bits())
        };
        let items = drawn
            .into_iter()
            .map(|v| Value::Int(i128::from(v), Prim::Int))
            .collect();
        Value::List(Rc::new(items))
    }
}
