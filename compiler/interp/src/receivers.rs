// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A method call's receiver, worked out before its arguments (spec 3.10):
//! a value, or the place a method that changes it in place changes.

use crate::calls::Args;
use crate::interp::Interp;
use crate::places::Place;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{Expr, ExprKind, Ident, Span};
use kavach_resolver::DefKind;
use std::rc::Rc;

/// What a call worked out before its arguments.
pub(crate) enum Receiver<'a> {
    /// Nothing: a type's or module's function, or a special form, whose
    /// receiver is worked out (if at all) with the call.
    Later,
    /// The receiver's value, or the function a callee expression gave.
    Value(Value<'a>),
    /// The place a method that changes its receiver in place changes.
    Place(Place<'a>),
}

impl<'a> Interp<'a> {
    /// What call `callee(…)` works out before its arguments. Kept out of
    /// `call`, so a call's stack stays small (spec 3.8).
    #[inline(never)]
    pub(crate) fn first_of(
        &mut self,
        callee: &'a Expr,
        span: Span,
    ) -> Result<Receiver<'a>, Flow<'a>> {
        match &callee.kind {
            ExprKind::Field { base, name, .. } => self.receiver_first(base, name, span),
            ExprKind::Name(_) => Ok(Receiver::Later),
            _ => Ok(Receiver::Value(self.eval(callee)?)),
        }
    }

    /// Works out the receiver of `base.name(…)`, before the arguments.
    pub(crate) fn receiver_first(
        &mut self,
        base: &'a Expr,
        name: &'a Ident,
        span: Span,
    ) -> Result<Receiver<'a>, Flow<'a>> {
        if let Some((at, _)) = &self.held_receiver
            && *at == (base.span.start, base.span.end)
            && let Some((_, v)) = self.held_receiver.take()
        {
            return Ok(Receiver::Value(v));
        }
        // `ch.take().timeout(d)`: the call it times runs with it.
        if name.name == "timeout" && matches!(base.kind, ExprKind::Call { .. }) {
            return Ok(Receiver::Later);
        }
        // `Point.origin()`, `fs.read_text(p)`: no value to work out.
        if let ExprKind::Name(id) = &base.kind
            && let Some(&def) = self.c.resolution.uses.get(&id.span.start)
            && !matches!(
                self.c.resolution.defs[def].kind,
                DefKind::Local { .. }
                    | DefKind::Param
                    | DefKind::Result
                    | DefKind::Variant { .. }
                    | DefKind::Field
            )
        {
            return Ok(Receiver::Later);
        }
        if self.in_place(base, name) {
            return Ok(Receiver::Place(self.place(base, span)?));
        }
        Ok(Receiver::Value(self.eval(base)?))
    }

    /// Whether `base.name(…)` changes `base` in place: `xs.push(x)` and
    /// `xs.pop()` on a list, and the other methods the checker's one list
    /// names (`kavach_checker::changes_in_place`). A user type's own
    /// `add` or `push`, or a method added to lists (spec 2.5), is a
    /// method like any other, and `7.add(2)` is `7 + 2`.
    pub(crate) fn in_place(&self, base: &'a Expr, name: &'a Ident) -> bool {
        (self.c.expr_types.get(&(base.span.start, base.span.end)))
            .is_some_and(|ty| kavach_checker::changes_in_place(ty, &name.name))
    }

    /// `xs.push(x)`, `rows[i].push(x)`, `xs.pop()`: the place lets go of
    /// its value while it changes, so it changes in place, not in a copy
    /// (Sakshi's S6), and gets the changed value back.
    pub(crate) fn change_in_place(
        &mut self,
        base: &'a Expr,
        name: &'a Ident,
        args: Args<'a>,
        place: Place<'a>,
        span: Span,
    ) -> Eval<'a> {
        let receiver = self.take_place(&place, base, span)?;
        // `xs.pop()` (spec 14.2): the last item, and the list without it.
        if name.name == "pop" {
            let Value::List(mut items) = receiver else {
                unreachable!("a list");
            };
            let last = Rc::make_mut(&mut items).pop();
            self.store(&place, Value::List(items), span)?;
            return Ok(last.unwrap_or(Value::None));
        }
        let changed = self.mutate(receiver, &name.name, args, span)?;
        self.store(&place, changed, span)?;
        Ok(Value::Unit)
    }

    /// `c[k] = c[k].add(a)`, made from `c[k] += a` on a program's own type
    /// (spec 2.16), with `c[k]` written once: the call's receiver is the
    /// value at the place already worked out (spec 3.10).
    pub(crate) fn hold_receiver(
        &mut self,
        target: &'a Expr,
        value: &'a Expr,
        place: &Place<'a>,
    ) -> Result<(), Flow<'a>> {
        if let ExprKind::Call { callee, .. } = &value.kind
            && let ExprKind::Field { base, .. } = &callee.kind
            && base.span == target.span
            && !matches!(target.kind, ExprKind::Name(_))
        {
            let current = self.read_place(place, target)?;
            self.held_receiver = Some(((base.span.start, base.span.end), current));
        }
        Ok(())
    }
}
