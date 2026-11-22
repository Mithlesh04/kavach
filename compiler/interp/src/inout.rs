// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `inout` arguments (spec 3.9): lent to a call that can't fail, so a
//! list or map in them changes in place there, and written back after.

use crate::interp::Interp;
use crate::places::Place;
use crate::receivers::Receiver;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{Arg, Expr, ExprKind, FnDecl, Span};

/// The places of `inout` arguments whose keys were worked out with the
/// call's arguments, by the argument's span.
pub(crate) type Held<'a> = Vec<((u32, u32), Place<'a>)>;

impl<'a> Interp<'a> {
    /// Which of `args`, as bits by position, are `inout` arguments with a
    /// key that calls something, `xs[pick()]`: none, at once, for a call
    /// whose arguments have only plain keys, as most do.
    pub(crate) fn keyed_inout(
        &self,
        callee: &'a Expr,
        first: &Receiver<'a>,
        args: &'a [Arg],
    ) -> u64 {
        if args.iter().all(|a| a.value.has_plain_keys()) {
            return 0;
        }
        let Some(f) = self.callee_decl(callee, first) else {
            return 0;
        };
        let mut bits = 0;
        let mut positional = 0;
        for (i, a) in args.iter().enumerate().take(64) {
            let mut params = f.params.iter().filter(|p| p.name.name != "self");
            let p = match &a.name {
                Some(n) => params.find(|p| p.name.name == n.name),
                None => {
                    positional += 1;
                    params.nth(positional - 1)
                }
            };
            if p.is_some_and(|p| p.is_inout) && !a.value.has_plain_keys() {
                bits |= 1 << i;
            }
        }
        bits
    }

    /// The function or method `callee(…)` calls, when it is one of the
    /// program's own, found as `call` and `method` find it.
    fn callee_decl(&self, callee: &'a Expr, first: &Receiver<'a>) -> Option<&'a FnDecl> {
        let uses = &self.c.resolution.uses;
        match &callee.kind {
            ExprKind::Name(id) => self.fns.get(uses.get(&id.span.start)?).copied(),
            ExprKind::Field { base, name, .. } => {
                let methods = match (first, &base.kind) {
                    (Receiver::Value(Value::Record { def, .. }), _) => self.methods.get(def),
                    (Receiver::Value(Value::Variant { union, .. }), _) => self.methods.get(union),
                    (Receiver::Value(v), _) => {
                        crate::calls::type_name(v).and_then(|t| self.ext_methods.get(t))
                    }
                    (Receiver::Later, ExprKind::Name(id)) => {
                        self.methods.get(uses.get(&id.span.start)?)
                    }
                    _ => None,
                };
                methods?.get(name.name.as_str()).copied()
            }
            _ => None,
        }
    }

    /// An `inout` argument with a key that calls something: its keys are
    /// worked out once, here, with the call's other arguments, and its
    /// place is kept in `held` for when its value goes back (spec 3.10).
    #[inline(never)]
    pub(crate) fn hold_inout(
        &mut self,
        arg: &'a Expr,
        held: &mut Held<'a>,
        span: Span,
    ) -> Eval<'a> {
        let place = self.place(arg, span)?;
        let value = self.read_place(&place, arg)?;
        held.push(((arg.span.start, arg.span.end), place));
        Ok(value)
    }

    /// Before a call: each `inout` argument's place lets go of its value,
    /// which the call now holds alone, so it changes in place there
    /// (Sakshi's S6); `write_back` puts it back after (spec 3.9).
    pub(crate) fn lend_inout(
        &mut self,
        f: &'a kavach_ast::FnDecl,
        args: &'a [Arg],
        held: &Held<'a>,
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
            let Some(a) = named.or_else(|| positional.get(k).copied()) else {
                continue;
            };
            let at = (a.value.span.start, a.value.span.end);
            if self
                .c
                .expr_types
                .get(&at)
                .is_none_or(|t| self.may_hold_drop(t, &mut Vec::new()))
            {
                continue;
            }
            // A place whose keys call something lets go through the
            // place they named, worked out with the arguments (3.10).
            match held.iter().find(|(s, _)| *s == at) {
                Some((_, place)) => self.store(place, Value::Unit, span)?,
                None if a.value.has_plain_keys() => self.assign(&a.value, Value::Unit, span)?,
                None => {}
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
    /// into the place its argument names (spec 3.9), the one `held` when
    /// its keys were worked out with the arguments (3.10).
    pub(crate) fn write_back(
        &mut self,
        args: &'a [Arg],
        held: Held<'a>,
        span: Span,
    ) -> Result<(), Flow<'a>> {
        for (name, at, value) in std::mem::take(&mut self.inout_out) {
            let named = args
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|n| n.name == name));
            let arg = named.or_else(|| args.iter().filter(|a| a.name.is_none()).nth(at));
            let Some(a) = arg else {
                continue;
            };
            let at = (a.value.span.start, a.value.span.end);
            match held.iter().find(|(s, _)| *s == at) {
                Some((_, place)) => self.store(place, value, span)?,
                None => self.assign(&a.value, value, span)?,
            }
        }
        Ok(())
    }
}
