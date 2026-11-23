// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `xs.group_by(key).map_values(g => g.sum(f))` on a list or a table (spec 14.2,
//! decision 0135): a sum per key, which the interpreter and native code
//! run as one pass over the items, with no groups made. Here: whether a
//! call is one, so both decide alike.

use crate::Ty;
use kavach_ast::{Expr, ExprKind};
use kavach_resolver::Resolution;
use std::collections::HashMap;

/// The parts of a sum per key.
pub struct GroupedSum<'e> {
    /// The list or table grouped.
    pub items: &'e Expr,
    /// `key`, of an item.
    pub key: &'e Expr,
    /// `f`, of an item: what is added.
    pub value: &'e Expr,
    /// The `g.sum(f)` call: its type is each sum's.
    pub sum: &'e Expr,
}

/// `e` as a sum per key, or `None`: `items` is a list, the lambda is
/// `g => g.sum(f)`, and `f` doesn't mention `g`.
pub fn grouped_sum<'e>(
    e: &'e Expr,
    types: &HashMap<(u32, u32), Ty>,
    res: &Resolution,
) -> Option<GroupedSum<'e>> {
    let (outer, "map_values", [lambda]) = method_call(e)? else {
        return None;
    };
    let (items, "group_by", [key]) = method_call(outer)? else {
        return None;
    };
    // A list, or a `Table` (14.34), whose rows are kept as a list.
    let table = res.builtin_type("std.table", "Table");
    match types.get(&(items.span.start, items.span.end)) {
        Some(Ty::List(_)) => {}
        Some(Ty::Adt(d, _)) if Some(*d) == table => {}
        _ => return None,
    }
    let ExprKind::Lambda { params, body } = &lambda.kind else {
        return None;
    };
    let [g] = params.as_slice() else {
        return None;
    };
    let (group, "sum", [value]) = method_call(body)? else {
        return None;
    };
    let ExprKind::Name(id) = &group.kind else {
        return None;
    };
    let def = |start: &u32| res.uses.get(start).and_then(|&d| res.defs[d].span);
    if def(&id.span.start) != Some(g.name.span) {
        return None;
    }
    let inside = value.span.start..value.span.end;
    let mentioned = (res.uses.keys()).any(|s| inside.contains(s) && def(s) == Some(g.name.span));
    (!mentioned).then_some(GroupedSum {
        items,
        key,
        value,
        sum: body,
    })
}

/// `base.name(args)`: the base, the method's name and the arguments'
/// values, for a call with no names on its arguments.
fn method_call(e: &Expr) -> Option<(&Expr, &str, [&Expr; 1])> {
    let ExprKind::Call { callee, args, .. } = &e.kind else {
        return None;
    };
    let ExprKind::Field {
        base,
        name,
        optional: false,
    } = &callee.kind
    else {
        return None;
    };
    let [arg] = args.as_slice() else {
        return None;
    };
    arg.name
        .is_none()
        .then_some((base, name.name.as_str(), [&arg.value]))
}
