//! A versioned loop's copy with locals of its own. `ranges.rs` gives each
//! local one range for the whole function, so a copy that shares its
//! locals with the loop as written shares their ranges too: where the
//! copy's `total` stays in `0..m`, the written loop's may go negative, and
//! the copy's `% m` keeps its sign. With its own locals, the copy's ranges
//! are its own; its accumulators start from the loop's, through a `let`
//! before it, and go back after it.

use crate::ir::*;
use crate::walk;

/// A copy of loop `written` whose body sets locals of its own, made in
/// `locals`, and its accumulators: the locals the body sets and that live
/// across rounds, as (the loop's, the copy's). `None` when the body sets a
/// local that isn't an integer, or sets one other than by `let` or `=`.
pub(super) fn own_locals(
    locals: &mut Vec<Type>,
    written: &Stmt,
) -> Option<(Stmt, Vec<(Local, Local)>)> {
    let Stmt::ForRange { body, .. } = written else {
        return None;
    };
    let mut lets = Vec::new();
    let mut plain = true;
    let mut probe = body.clone();
    walk::blocks_mut(&mut probe, &mut |b| {
        for s in &b.stmts {
            match s {
                Stmt::Let(l, _) => lets.push(*l),
                Stmt::LetResult { .. }
                | Stmt::ForRange { .. }
                | Stmt::ForEach { .. }
                | Stmt::While { .. } => plain = false,
                _ => {}
            }
        }
    });
    let mut assigned = Vec::new();
    walk::any_in_block(body, &mut |e| {
        match e.kind {
            ExprKind::Assign(l, _) if !assigned.contains(&l) => assigned.push(l),
            ExprKind::Move(_) => plain = false,
            _ => {}
        }
        false
    });
    let mut set = lets.clone();
    set.extend(assigned.iter().filter(|l| !lets.contains(l)));
    if !plain || set.iter().any(|l| !locals[*l as usize].is_int()) {
        return None;
    }
    let renamed: Vec<(Local, Local)> = (set.iter())
        .map(|&old| {
            locals.push(locals[old as usize]);
            (old, (locals.len() - 1) as Local)
        })
        .collect();
    let mut copy = Block {
        stmts: vec![written.clone()],
        value: None,
    };
    walk::locals_mut(&mut copy, &mut |l| {
        if let Some((_, new)) = renamed.iter().find(|(old, _)| old == l) {
            *l = *new;
        }
    });
    let accumulators = (renamed.into_iter())
        .filter(|(old, _)| assigned.contains(old) && !lets.contains(old))
        .collect();
    Some((copy.stmts.remove(0), accumulators))
}
