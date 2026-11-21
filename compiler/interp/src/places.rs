// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The place an assignment stores into: a variable's field, item,
//! slice or tuple element, reached step by step.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{AssignOp, Expr, ExprKind, Span};
use kavach_resolver::DefId;
use std::rc::Rc;

/// One step from a variable to the part being assigned: `.field`, `[i]`, `.0`.
#[derive(Clone)]
pub(crate) enum Step<'a> {
    Field(&'a str),
    Index(Value<'a>),
    /// `[#k]` and `[a:b:step]` (spec 2.19).
    FromEnd(i64),
    Slice(kavach_runtime_core::slices::Part),
    Tuple(usize),
}

/// Follows `steps` inside `slot` and stores `value` there.
pub(crate) fn set<'a>(
    slot: &mut Value<'a>,
    steps: &[Step<'a>],
    value: Value<'a>,
) -> Result<(), String> {
    let Some((step, rest)) = steps.split_first() else {
        *slot = value;
        return Ok(());
    };
    let at;
    let step = match (step, &*slot) {
        (Step::Slice(part), _) if rest.is_empty() => {
            return crate::slicing::set_slice(slot, part, value);
        }
        (Step::FromEnd(k), v) => {
            let n = crate::slicing::positions(v).unwrap_or(0);
            at = Step::Index(Value::Int(
                kavach_runtime_core::slices::from_end(n, *k),
                kavach_checker::Prim::Int,
            ));
            &at
        }
        _ => step,
    };
    let next = match (step, slot) {
        (Step::Field(name), Value::Record { fields, life, .. }) => {
            let field = fields.iter_mut().find(|f| *f.name == **name);
            let Some(field) = field else {
                return Err("can't assign here".to_string());
            };
            set(&mut field.value, rest, value)?;
            // Changed in place: the same value, unless a copy shares it.
            crate::drops::renew(life, fields, false);
            return Ok(());
        }
        (Step::Index(Value::Int(i, _)), Value::List(items)) => {
            let items = Rc::make_mut(items);
            let len = items.len();
            let item = usize::try_from(*i).ok().and_then(|i| items.get_mut(i));
            if item.is_none() {
                return Err(kavach_runtime_core::list_index_panic(*i, len));
            }
            item
        }
        (Step::Index(key), Value::Map(m)) => {
            m.room_for(key, "map")?;
            Some(Rc::make_mut(m).slot(key.clone()))
        }
        (Step::Tuple(i), Value::Tuple(items)) => items.get_mut(*i),
        _ => None,
    };
    match next {
        Some(next) => set(next, rest, value),
        None => Err("can't assign here".to_string()),
    }
}

/// The value `steps` lead to inside `slot`, if there is one: a record's
/// field, a list's item (`#k` too), a dict's value or a tuple's part.
pub(crate) fn get<'v, 'a>(slot: &'v Value<'a>, steps: &[Step<'a>]) -> Option<&'v Value<'a>> {
    let Some((step, rest)) = steps.split_first() else {
        return Some(slot);
    };
    let next = match (step, slot) {
        (Step::Field(name), Value::Record { fields, .. }) => {
            fields.iter().find(|f| *f.name == **name).map(|f| &f.value)
        }
        (Step::Index(Value::Int(i, _)), Value::List(items)) => {
            usize::try_from(*i).ok().and_then(|i| items.get(i))
        }
        (Step::FromEnd(k), Value::List(items)) => {
            let at = kavach_runtime_core::slices::from_end(items.len(), *k);
            usize::try_from(at).ok().and_then(|i| items.get(i))
        }
        (Step::Index(key), Value::Map(m)) => m.get(key),
        (Step::Tuple(i), Value::Tuple(items)) => items.get(*i),
        _ => None,
    }?;
    get(next, rest)
}

/// A place worked out: its variable, and the steps into it, with every key
/// already worked out, so each is worked out once (spec 3.10).
pub(crate) struct Place<'a> {
    def: Option<DefId>,
    steps: Vec<Step<'a>>,
}

impl<'a> Place<'a> {
    /// The key of the place's last step, `k` in `d[k]`.
    fn last_key(&self) -> Option<&Value<'a>> {
        match self.steps.last() {
            Some(Step::Index(key)) => Some(key),
            _ => None,
        }
    }
}

impl<'a> Interp<'a> {
    /// Works out `target`'s keys, left to right as written: the base's
    /// before the outer ones, as `xs[i][j]` works out `i`, then `j`.
    pub(crate) fn place(&mut self, target: &'a Expr, span: Span) -> Result<Place<'a>, Flow<'a>> {
        let mut chain = Vec::new();
        let mut root = target;
        while let ExprKind::Field { base, .. }
        | ExprKind::Index { base, .. }
        | ExprKind::TupleField { base, .. } = &root.kind
        {
            chain.push(root);
            root = base;
        }
        let ExprKind::Name(id) = &root.kind else {
            return Err(self.panic("can't assign here", span));
        };
        let def = self.c.resolution.uses.get(&id.span.start).copied();
        let mut steps = Vec::new();
        for part in chain.into_iter().rev() {
            steps.push(match &part.kind {
                ExprKind::Field { name, .. } => Step::Field(name.name.as_str()),
                ExprKind::Index { index, .. } => match &index.kind {
                    ExprKind::FromEnd(k) => Step::FromEnd(self.int_of(k)?),
                    ExprKind::Part { .. } => Step::Slice(self.part(index)?),
                    _ => Step::Index(self.eval(index)?),
                },
                ExprKind::TupleField { index, .. } => Step::Tuple(*index as usize),
                _ => unreachable!("a step"),
            });
        }
        Ok(Place { def, steps })
    }

    /// `target = value` and `target op= value`: the place's keys first,
    /// each once, then the value, as written (spec 3.10). Kept out of
    /// `stmt`, so a call's stack stays small (spec 3.8).
    #[inline(never)]
    pub(crate) fn assign_stmt(
        &mut self,
        target: &'a Expr,
        op: AssignOp,
        value: &'a Expr,
        span: Span,
    ) -> Result<(), Flow<'a>> {
        // `(a, b) = (b, a)`: the whole right side first (spec 1.14).
        if matches!(target.kind, ExprKind::Tuple(_)) {
            let v = self.eval(value)?;
            return self.assign_parts(target, v, span);
        }
        let place = self.place(target, span)?;
        let Some(bin) = op.binary() else {
            self.hold_receiver(target, value, &place)?;
            let made = self.eval(value);
            self.held_receiver = None;
            return self.store(&place, made?, span);
        };
        let current = self.read_place(&place, target)?;
        let v = self.eval(value)?;
        // `xs += ys` and `s += part`: the place lets go of its list or
        // text first, so it grows in place, not in a copy (spec 2.2, 6.1).
        if matches!(current, Value::List(_) | Value::Str(_)) {
            // No invariant sees the empty place (spec 7.3).
            self.emptying = true;
            let emptied = self.store(&place, Value::Unit, span);
            self.emptying = false;
            emptied?;
        }
        let v = match (current, v) {
            // A list with a limit keeps it, and fills up to it (spec 2.2).
            (Value::List(mut xs), Value::List(ys)) if xs.limit.is_some() => {
                for y in ys.iter() {
                    crate::items::push(&mut xs, y.clone()).map_err(|m| self.panic(m, span))?;
                }
                Value::List(xs)
            }
            (current, v) => self.arith(bin, current, v, span)?,
        };
        self.store(&place, v, span)
    }

    /// Stores `value` into a variable, field, index or tuple element.
    pub(crate) fn assign(
        &mut self,
        target: &'a Expr,
        value: Value<'a>,
        span: Span,
    ) -> Result<(), Flow<'a>> {
        let place = self.place(target, span)?;
        self.store(&place, value, span)
    }

    /// Stores `value` into a place already worked out.
    pub(crate) fn store(
        &mut self,
        place: &Place<'a>,
        value: Value<'a>,
        span: Span,
    ) -> Result<(), Flow<'a>> {
        let Some(def) = place.def else {
            return Ok(());
        };
        let steps = &place.steps;
        let mut slot = self.frame().vars.remove(&def).unwrap_or(Value::Unit);
        let result = set(&mut slot, steps, value);
        // Each record on the path holds the changed value: its invariants
        // are checked, the innermost first (spec 7.3).
        let mut checked = Ok(());
        if result.is_ok() {
            for k in (0..steps.len().max(1)).rev() {
                if let Some(inner) = get(&slot, &steps[..k])
                    && let Err(e) = self.check_invariants(inner, span)
                {
                    checked = Err(e);
                    break;
                }
            }
        }
        self.frame().vars.insert(def, slot);
        result.map_err(|m| self.panic(m, span))?;
        checked
    }

    /// The value at a place already worked out, `target` the place as
    /// written: a dict's missing key is a panic that names it (2.2).
    pub(crate) fn read_place(&mut self, place: &Place<'a>, target: &'a Expr) -> Eval<'a> {
        let found = place.def.and_then(|d| {
            let slot = self.frame().vars.get(&d)?;
            get(slot, &place.steps).cloned()
        });
        let value = match found {
            Some(v) => v,
            None if place.last_key().is_some() && self.keyed_by_dict(target) => Value::None,
            // Not there: as the place is read when written alone, with
            // its panic (a list's item out of range).
            None => self.eval(target)?,
        };
        if let (Value::None, Some(key)) = (&value, place.last_key())
            && self.keyed_by_dict(target)
        {
            let key = key.display();
            return Err(self.panic(format!("the dict has no key `{key}`"), target.span));
        }
        Ok(value)
    }

    /// Takes the value out of a place, leaving it empty while the value
    /// changes, so a list changes in place, not in a copy (spec 2.2, 6.1).
    pub(crate) fn take_place(
        &mut self,
        place: &Place<'a>,
        target: &'a Expr,
        span: Span,
    ) -> Eval<'a> {
        if place.steps.is_empty()
            && let Some(v) = place.def.and_then(|d| self.frame().vars.remove(&d))
        {
            return Ok(v);
        }
        let value = self.read_place(place, target)?;
        // No invariant sees the empty place (spec 7.3).
        self.emptying = true;
        let emptied = self.store(place, Value::Unit, span);
        self.emptying = false;
        emptied?;
        Ok(value)
    }

    /// Whether `target` is an item of a dict: `d[k]`.
    fn keyed_by_dict(&self, target: &'a Expr) -> bool {
        let ExprKind::Index { base, .. } = &target.kind else {
            return false;
        };
        matches!(
            self.c.expr_types.get(&(base.span.start, base.span.end)),
            Some(kavach_checker::Ty::Map(..))
        )
    }
}
