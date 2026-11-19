// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `f.unfreeze(key: k)` (spec 2.20). The driver has already turned the
//! rest of frozen values into plain code (`compiler/driver/src/frozen.rs`):
//! a sealed value is `(value, key)`. Here the key is checked.

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::*;

impl<'a> Interp<'a> {
    /// The value of `f.unfreeze(key: k)` when `e` is one: `f`'s value
    /// when `k` equals its key, else a panic.
    pub(crate) fn sealed_unfreeze(
        &mut self,
        e: &'a Expr,
        callee: &'a Expr,
        args: &'a [Arg],
    ) -> Option<Eval<'a>> {
        if self.c.sealed_unfreezes.is_empty()
            || !(self.c.sealed_unfreezes).contains(&(e.span.start, e.span.end))
        {
            return None;
        }
        let (ExprKind::Field { base, .. }, [k]) = (&callee.kind, args) else {
            return None;
        };
        Some((|| {
            let sealed = self.eval(base)?;
            let key = self.eval(&k.value)?;
            match sealed {
                Value::Tuple(mut parts) if parts.len() == 2 && parts[1] == key => {
                    Ok(parts.swap_remove(0))
                }
                _ => Err(self.panic(kavach_checker::WRONG_KEY, e.span)),
            }
        })())
    }
}
