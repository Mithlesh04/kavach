// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Functions of built-in types, called on the type's name: `Bytes.of` and
//! `Bytes.of_list` (spec 2.13), `Float.from_bits`, `TextBuilder.capacity`
//! (2.18), `Set.of`, and `capacity` of lists, maps, sets and rings (2.2).

use crate::calls::Args;
use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::{Span, TypeExpr};

impl<'a> Interp<'a> {
    /// `owner.name(args)` for built-in type `owner`, or the arguments back
    /// when it has no such function.
    pub(crate) fn builtin_static(
        &mut self,
        owner: &str,
        name: &str,
        generics: &'a [TypeExpr],
        args: Args<'a>,
        span: Span,
    ) -> Result<Eval<'a>, Args<'a>> {
        let int = |args: &Args<'a>| match args.first() {
            Some((_, Value::Int(n, _))) => *n,
            _ => 0,
        };
        Ok(match (owner, name, generics) {
            ("Bytes", "of", [item]) => self.bytes_of_number(item, &args, span),
            ("Bytes", "of_list", [item]) => self.bytes_of_list(item, &args, span),
            ("Float", "from_bits", _) => Ok(Value::Float(f64::from_bits(int(&args) as u64))),
            ("TextBuilder", "capacity", _) => Ok(self.builder_with_room(int(&args))),
            ("Set", "of", _) => Ok(self.set_of(args.into_iter().map(|(_, v)| v).collect())),
            ("List" | "Dict" | "Set" | "Ring", "capacity", _) => self.capacity(owner, args, span),
            _ => return Err(args),
        })
    }
}
