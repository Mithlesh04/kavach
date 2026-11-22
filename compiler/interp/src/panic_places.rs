// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Where a panic is shown (spec 5.6): a panic in the standard library's
//! own code points at the program's call into it, the line a person can
//! change, as native code does (`kir/src/caller_lines.rs`).

use crate::interp::Interp;
use crate::value::{Eval, Flow};
use kavach_ast::{FnDecl, Span};

impl<'a> Interp<'a> {
    /// The result of a call of `f` at `call`, as its caller sees it. A
    /// panic leaving it adds `f` to the functions a crash report lists
    /// (14.30); one from the standard library's code, leaving it for the
    /// program's own code, now points at `call`.
    pub(crate) fn left_call(&mut self, f: &'a FnDecl, result: Eval<'a>, call: Span) -> Eval<'a> {
        let Err(Flow::Panic(message, at)) = result else {
            return result;
        };
        let (path, line) = self.c.shown_location(f.name.span.start);
        (self.panic_trail).push(format!("{} ({path}:{line})", f.name.name));
        let ours = self.c.in_std(at.start) && !self.c.in_std(call.start);
        Err(Flow::Panic(message, if ours { call } else { at }))
    }
}
