// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.reflect` in native code (spec 14.32): the compiler worked each
//! answer out while compiling. `x.has_member(name)` is its `Bool`;
//! `T.members()` and `m.members()` pass its JSON to `std.reflect`'s
//! `members_from`.

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// The answer to reflection call `e` of type `ty`, if it is one.
    pub(crate) fn reflected(&mut self, e: &'a ast::Expr, ty: Type) -> Option<Lowered<ir::Expr>> {
        let at = (e.span.start, e.span.end);
        let line = self.line(e.span);
        if let Some(&has) = self.c.reflect_bools.get(&at) {
            return Some(Ok(ir::Expr {
                kind: K::Bool(has),
                ty: Type::Bool,
                line,
            }));
        }
        let text = self.c.reflect_texts.get(&at)?.clone();
        let reader = "members_from";
        let res = &self.c.resolution;
        let fid = self.fn_ids.iter().find_map(|(d, f)| {
            let def = &res.defs[*d];
            let module = def
                .span
                .map(|s| res.modules[res.module_at(s.start)].name.as_str());
            (def.name == reader && module == Some("std.reflect")).then_some(*f)
        })?;
        let text = ir::Expr {
            kind: K::Str(text),
            ty: self.str_type(),
            line,
        };
        Some(Ok(ir::Expr {
            kind: K::Call(fid, vec![text]),
            ty,
            line,
        }))
    }
}
