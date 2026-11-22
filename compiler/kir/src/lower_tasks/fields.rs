// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A task given only the fields it reads (spec 9.1). A block given to
//! `spawn` that reads `r.flag` captured all of `r`, so a task spawned
//! beside a record holding millions of items copied them all (Sakshi's
//! S10). When the task's code reads a captured record only through its
//! fields, and never changes it, the task gets those fields instead: the
//! same values, as a captured variable is a copy made at the spawn.

use crate::ir::{Expr, ExprKind as K, Function, Local, Shape, Stmt, Type};
use crate::walk::{any_in_block, exprs_mut, locals_mut};
use std::collections::BTreeMap;

/// What one slot of a task's narrowed closure holds: slot `i` of the
/// closure as first made, or field `k` (of type `t`) of the record in it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Slot {
    Whole(usize),
    Field(usize, u32, Type),
}

/// Narrows the closure of task function `f`, whose first `captured`
/// slots hold the variables it captured (`unpack_env`), each read by a
/// `Let` at the top of its body. Gives the new slots in order, or `None`
/// when no captured record can be narrowed; then `f` is unchanged.
pub(crate) fn narrow(
    f: &mut Function,
    slots: usize,
    captured: usize,
    shapes: &[Shape],
) -> Option<Vec<Slot>> {
    let env = *f.params.first()?;
    // The local each captured slot is read into, from `unpack_env`.
    let mut unpacked: Vec<Local> = Vec::new();
    for s in f.body.stmts.iter().take(captured) {
        match s {
            Stmt::Let(l, e) if matches!(&e.kind, K::GetField(b, _) if is_local(b, env)) => {
                unpacked.push(*l);
            }
            _ => return None,
        }
    }
    if unpacked.len() != captured {
        return None;
    }
    // The closure is only read slot by slot, so its slots can move.
    let mut env_reads = 0;
    any_in_block(&f.body, &mut |e| {
        env_reads += usize::from(matches!(&e.kind, K::GetField(b, _) if is_local(b, env)));
        false
    });
    let mut env_mentions = 0;
    locals_mut(&mut f.body, &mut |x| env_mentions += usize::from(*x == env));
    if env_mentions != env_reads {
        return None;
    }
    // The fields each narrowed slot's record is read through.
    let mut fields: BTreeMap<usize, Vec<(u32, Type)>> = BTreeMap::new();
    for (i, &l) in unpacked.iter().enumerate() {
        if let Some(read) = only_fields(f, l, shapes) {
            fields.insert(i, read);
        }
    }
    if fields.is_empty() {
        return None;
    }
    // The new slots, and where each old one went.
    let mut new_slots = Vec::new();
    let mut moved: Vec<Option<u32>> = Vec::new();
    let mut field_locals: BTreeMap<(usize, u32), Local> = BTreeMap::new();
    for i in 0..slots {
        match fields.get(&i) {
            Some(read) => {
                moved.push(None);
                for &(k, t) in read {
                    new_slots.push(Slot::Field(i, k, t));
                    f.locals.push(t);
                    field_locals.insert((i, k), (f.locals.len() - 1) as Local);
                }
            }
            None => {
                moved.push(Some(new_slots.len() as u32 + 1));
                new_slots.push(Slot::Whole(i));
            }
        }
    }
    // Every read of the closure takes its new slot.
    let by_local: BTreeMap<Local, usize> = fields.keys().map(|&i| (unpacked[i], i)).collect();
    exprs_mut(&mut f.body, &mut |e| match &mut e.kind {
        K::GetField(b, slot) if is_local(b, env) => {
            if let Some(Some(to)) = moved.get(*slot as usize - 1) {
                *slot = *to;
            }
        }
        K::GetField(b, k) => {
            let read = match &b.kind {
                K::Local(l) | K::Move(l) => by_local.get(l).map(|&i| field_locals[&(i, *k)]),
                _ => None,
            };
            if let Some(l) = read {
                e.kind = K::Local(l);
            }
        }
        _ => {}
    });
    // The narrowed records are no longer unpacked; their fields are.
    let mut top = Vec::new();
    for (n, s) in std::mem::take(&mut f.body.stmts).into_iter().enumerate() {
        match (n < captured).then(|| fields.get(&n)).flatten() {
            Some(read) => top.extend(read.iter().map(|&(k, _)| {
                let (l, at) = (field_locals[&(n, k)], new_slot(&new_slots, n, k));
                let ty = f.locals[l as usize];
                let env_read = K::GetField(Box::new(local(env, f.locals[env as usize])), at);
                Stmt::Let(
                    l,
                    Expr {
                        kind: env_read,
                        ty,
                        line: f.line,
                    },
                )
            })),
            None => top.push(s),
        }
    }
    f.body.stmts = top;
    Some(new_slots)
}

/// Gives task function `f`'s closure parameter its narrowed type, where
/// it is declared and where it is read.
pub(crate) fn retype_env(f: &mut Function, env_ty: Type) {
    let env = f.params[0];
    f.locals[env as usize] = env_ty;
    exprs_mut(&mut f.body, &mut |e| {
        if is_local(e, env) {
            e.ty = env_ty;
        }
    });
}

fn new_slot(slots: &[Slot], i: usize, k: u32) -> u32 {
    let at = slots
        .iter()
        .position(|s| matches!(s, Slot::Field(j, f, _) if *j == i && *f == k));
    at.expect("the field's slot") as u32 + 1
}

fn local(l: Local, ty: Type) -> Expr {
    Expr {
        kind: K::Local(l),
        ty,
        line: 0,
    }
}

fn is_local(e: &Expr, l: Local) -> bool {
    matches!(e.kind, K::Local(x) if x == l)
}

/// The record fields `l` is read through, when it is a record without
/// `drop` read only as `l.field` and never set after its `Let`.
fn only_fields(f: &mut Function, l: Local, shapes: &[Shape]) -> Option<Vec<(u32, Type)>> {
    let Type::Ref(id) = f.locals[l as usize] else {
        return None;
    };
    if !matches!(
        shapes.get(id as usize),
        Some(Shape::Record { drop: false, .. })
    ) {
        return None;
    }
    let mut read = Vec::new();
    any_in_block(&f.body, &mut |e| {
        if let K::GetField(b, k) = &e.kind
            && matches!(b.kind, K::Local(x) | K::Move(x) if x == l)
        {
            read.push(*k);
        }
        false
    });
    let mut mentions = 0;
    locals_mut(&mut f.body, &mut |x| mentions += usize::from(*x == l));
    // Its own `Let`, and each field read: nothing else names it.
    if read.is_empty() || mentions != read.len() + 1 {
        return None;
    }
    read.sort_unstable();
    read.dedup();
    let rec = f.locals[l as usize];
    read.into_iter()
        .map(|k| Some((k, field_type(rec, k, shapes)?)))
        .collect()
}

fn field_type(rec: Type, k: u32, shapes: &[Shape]) -> Option<Type> {
    let Type::Ref(id) = rec else { return None };
    let t = match shapes.get(id as usize) {
        Some(Shape::Record { fields, .. }) => fields.get(k as usize)?.ty,
        _ => return None,
    };
    // A field waiting for its `drop` stays inside its record.
    match t {
        Type::Ref(f)
            if matches!(
                shapes.get(f as usize),
                Some(Shape::Record { drop: true, .. })
            ) =>
        {
            None
        }
        _ => Some(t),
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{ExprKind, Program, Type};

    /// The types each spawned task's closure holds.
    fn held(src: &str) -> Vec<Vec<Type>> {
        let c = kavach_driver::compile(src, "t.kav", Default::default());
        let program: Program = crate::lower(&c).expect("lowers");
        let mut held = Vec::new();
        for f in &program.functions {
            crate::walk::any_in_block(&f.body, &mut |e| {
                if let ExprKind::Rt(crate::rt_op::RtOp::TaskSpawn, args) = &e.kind
                    && let ExprKind::Closure { captures, .. } = &args[0].kind
                {
                    held.push(captures.iter().map(|c| c.ty).collect());
                }
                false
            });
        }
        held
    }

    const RECORD: &str = "type R {\n    flag: Bool\n    items: [Int]\n}\n\nfn work_on(f: Bool, n: Int) -> Int {\n    if f { n } else { 0 }\n}\n\n";

    /// Sakshi's S10: a block that reads `r.flag` holds a `Bool`, not `r`.
    #[test]
    fn a_spawned_block_takes_only_the_fields_it_reads() {
        let main = "fn main() {\n    const r = R(flag: true, items: [1, 2])\n    const t = spawn cores {\n        const n = 5\n        work_on(r.flag, n) + work_on(r.flag, 1)\n    }\n    print(t.wait() catch 0)\n}\n";
        assert_eq!(held(&format!("{RECORD}{main}")), vec![vec![Type::Bool]]);
    }

    /// A block that reads the whole record still holds it.
    #[test]
    fn a_spawned_block_reading_the_record_holds_it() {
        let main = "fn main() {\n    const r = R(flag: true, items: [1, 2])\n    const t = spawn cores {\n        const s = r\n        work_on(s.flag, r.items.len())\n    }\n    print(t.wait() catch 0)\n}\n";
        let held = held(&format!("{RECORD}{main}"));
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].len(), 1);
        assert!(matches!(held[0][0], Type::Ref(_)));
    }
}
