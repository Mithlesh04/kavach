// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The place an assignment stores into: a variable's field, item,
//! slice or tuple element, reached step by step.

use crate::value::Value;
use std::rc::Rc;

/// One step from a variable to the part being assigned: `.field`, `[i]`, `.0`.
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
