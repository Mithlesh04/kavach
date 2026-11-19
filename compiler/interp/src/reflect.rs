// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.reflect` (spec 14.32): the compiler worked each answer out while
//! compiling. `x.has_member(name)` is its `Bool`; `T.members()` and
//! `m.members()` read its JSON with `std.reflect`'s `members_from`.

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::Expr;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// The answer to reflection call `e`, if it is one.
    pub(crate) fn reflected(&mut self, e: &'a Expr) -> Option<Eval<'a>> {
        let at = (e.span.start, e.span.end);
        if let Some(&has) = self.c.reflect_bools.get(&at) {
            return Some(Ok(Value::Bool(has)));
        }
        let text = self.c.reflect_texts.get(&at)?.clone();
        let f = self.fns.values().copied().find(|f| {
            f.name.name == "members_from"
                && self.c.resolution.modules[self.c.resolution.module_at(f.name.span.start)].name
                    == "std.reflect"
        })?;
        Some(self.call_fn(f, None, vec![(None, Value::Str(Rc::new(text)))], e.span))
    }
}
