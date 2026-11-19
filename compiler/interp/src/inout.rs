// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `inout` arguments (spec 3.9): lent to a call that can't fail, so a
//! list or map in them changes in place there, and written back after.

use crate::interp::Interp;
use crate::value::{Flow, Value};
use kavach_ast::{Arg, Span};

impl<'a> Interp<'a> {
    /// Before a call: each `inout` argument's place lets go of its value,
    /// which the call now holds alone, so it changes in place there
    /// (Sakshi's S6); `write_back` puts it back after (spec 3.9).
    pub(crate) fn lend_inout(
        &mut self,
        f: &'a kavach_ast::FnDecl,
        args: &'a [Arg],
        span: Span,
    ) -> Result<(), Flow<'a>> {
        // A call that can fail leaves the old value where it was (3.9),
        // so its place keeps it: only a call that can't fail borrows.
        if f.errors.is_some() {
            return Ok(());
        }
        let positional: Vec<&Arg> = args.iter().filter(|a| a.name.is_none()).collect();
        for (k, p) in f
            .params
            .iter()
            .filter(|p| p.name.name != "self")
            .enumerate()
        {
            if !p.is_inout {
                continue;
            }
            let named = args
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|n| n.name == p.name.name));
            // A `Drop` value it replaces is freed after the call, in the
            // caller (6.1): such a value isn't lent.
            if let Some(a) = named.or_else(|| positional.get(k).copied())
                && let Some(t) = self
                    .c
                    .expr_types
                    .get(&(a.value.span.start, a.value.span.end))
                && !self.may_hold_drop(t, &mut Vec::new())
            {
                self.assign(&a.value, Value::Unit, span)?;
            }
        }
        Ok(())
    }

    /// Whether a value of type `t` may be or hold a `Drop` value: a record
    /// or union through its fields, an interface or function value always.
    fn may_hold_drop(
        &self,
        t: &kavach_checker::Ty,
        seen: &mut Vec<kavach_resolver::DefId>,
    ) -> bool {
        use kavach_checker::Ty;
        use kavach_resolver::DefKind;
        match t {
            Ty::Adt(d, args) => {
                if self.c.drop_types.contains(d) || args.iter().any(|a| self.may_hold_drop(a, seen))
                {
                    return true;
                }
                if !matches!(self.c.resolution.defs[*d].kind, DefKind::Type) {
                    return true;
                }
                if seen.contains(d) {
                    return false;
                }
                seen.push(*d);
                let defs = &self.c.resolution.defs;
                let part = |o: &kavach_resolver::DefId| {
                    *o == *d || matches!(defs[*o].kind, DefKind::Variant { of } if of == *d)
                };
                let fields: Vec<&Ty> = (self.c.field_types.iter())
                    .filter(|((o, _), _)| part(o))
                    .map(|(_, t)| t)
                    .collect();
                fields.into_iter().any(|f| self.may_hold_drop(f, seen))
            }
            Ty::List(x) | Ty::Optional(x) | Ty::Set(x) | Ty::Range(x) => {
                self.may_hold_drop(x, seen)
            }
            Ty::Map(k, v) => self.may_hold_drop(k, seen) || self.may_hold_drop(v, seen),
            Ty::Tuple(ts) => ts.iter().any(|x| self.may_hold_drop(x, seen)),
            Ty::Anon(fs) => fs.iter().any(|(_, x)| self.may_hold_drop(x, seen)),
            Ty::Builtin(_, args) => args.iter().any(|a| self.may_hold_drop(a, seen)),
            Ty::Prim(_) | Ty::Unit | Ty::Never | Ty::Error => false,
            _ => true,
        }
    }

    /// After a call that returned: each `inout` parameter's last value
    /// into the place its argument names (spec 3.9).
    pub(crate) fn write_back(&mut self, args: &'a [Arg], span: Span) -> Result<(), Flow<'a>> {
        for (name, at, value) in std::mem::take(&mut self.inout_out) {
            let named = args
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|n| n.name == name));
            let arg = named.or_else(|| args.iter().filter(|a| a.name.is_none()).nth(at));
            if let Some(a) = arg {
                self.assign(&a.value, value, span)?;
            }
        }
        Ok(())
    }
}
