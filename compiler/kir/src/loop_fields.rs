//! A record's plain fields that a loop only reads and sets, kept in locals
//! while it runs: `r.line += 1` in a loop over a function's `inout r`
//! read and wrote the record on the heap each round, where a local stays
//! in a register (Sakshi's S11: 275 ms against 39 for the same loop on a
//! local). Each such field is read before the loop and set back after
//! it. Only when the loop does nothing else with the record (no call
//! given it, no read of all of it) and can't leave the function in the
//! middle (`return`, `fail`, a call that can fail), so it always ends
//! where the field is set back.

use crate::ir::*;
use crate::walk::{any_expr, any_in_block, exprs_mut};

pub fn keep_fields_in_locals(f: &mut Function, shapes: &[Shape]) {
    let mut locals = std::mem::take(&mut f.locals);
    walk(&mut f.body, &mut locals, shapes);
    f.locals = locals;
}

/// Each loop in `b` (not inside another loop), and the blocks of `b`'s
/// other statements.
fn walk(b: &mut Block, locals: &mut Vec<Type>, shapes: &[Shape]) {
    let mut i = 0;
    while i < b.stmts.len() {
        if is_loop(&b.stmts[i]) {
            let (before, after) = promote(&mut b.stmts[i], locals, shapes);
            let n = before.len();
            b.stmts.splice(i..i, before);
            i += n;
            b.stmts.splice(i + 1..i + 1, after.iter().cloned());
            i += 1 + after.len();
            continue;
        }
        match &mut b.stmts[i] {
            Stmt::Let(_, e) | Stmt::Expr(e) => inner_blocks(e, locals, shapes),
            _ => {}
        }
        i += 1;
    }
}

fn inner_blocks(e: &mut Expr, locals: &mut Vec<Type>, shapes: &[Shape]) {
    match &mut e.kind {
        ExprKind::Block(b) => walk(b, locals, shapes),
        ExprKind::If(_, a, b) => {
            walk(a, locals, shapes);
            walk(b, locals, shapes);
        }
        _ => {}
    }
}

fn is_loop(s: &Stmt) -> bool {
    matches!(
        s,
        Stmt::While { .. } | Stmt::ForRange { .. } | Stmt::ForEach { .. }
    )
}

/// The parts of loop `s` worked out each round: its condition and body.
fn parts(s: &mut Stmt) -> (Option<&mut Expr>, &mut Block) {
    match s {
        Stmt::While { cond, body } => (Some(cond), body),
        Stmt::ForRange { body, .. } | Stmt::ForEach { body, .. } => (None, body),
        _ => unreachable!("a loop"),
    }
}

/// A field kept in a local: its number, whether the loop sets it, and its
/// type.
type Kept = (u32, bool, Type);

/// The parts of loop `s`, to read.
fn parts_ref(s: &Stmt) -> (Option<&Expr>, &Block) {
    match s {
        Stmt::While { cond, body } => (Some(cond), body),
        Stmt::ForRange { body, .. } | Stmt::ForEach { body, .. } => (None, body),
        _ => unreachable!("a loop"),
    }
}

/// The statements before and after loop `s` that keep fields in locals.
fn promote(s: &mut Stmt, locals: &mut Vec<Type>, shapes: &[Shape]) -> (Vec<Stmt>, Vec<Stmt>) {
    let (mut before, mut after) = (Vec::new(), Vec::new());
    // First what to keep: each record, and its plain fields used.
    let mut plan: Vec<(Local, Type, Vec<Kept>)> = Vec::new();
    {
        let (cond, body) = parts_ref(s);
        if leaves_early(body) {
            return (before, after);
        }
        for r in records_used(body, cond, locals, shapes) {
            // The loop's own variable, or one it makes, has no value yet
            // before the loop.
            if made_in(s, r) {
                continue;
            }
            let Some(fields) = fields_only(body, cond, r) else {
                continue;
            };
            let Type::Ref(id) = locals[r as usize] else {
                continue;
            };
            let Some(Shape::Record { fields: decl, .. }) = shapes.get(id as usize) else {
                continue;
            };
            let kept: Vec<Kept> = (fields.into_iter())
                .filter_map(|(k, set)| decl.get(k as usize).map(|f| (k, set, f.ty)))
                .filter(|(_, _, t)| !t.is_ref() && *t != Type::Unit)
                .collect();
            // A field left on the heap would read a stale record.
            if !kept.is_empty() && kept.len() == fields_count(body, cond, r) {
                plan.push((r, locals[r as usize], kept));
            }
        }
    }
    for (r, rty, kept) in plan {
        for (k, set, fty) in kept {
            let held = locals.len() as Local;
            locals.push(fty);
            let line = 0;
            let rec = |kind| Expr {
                kind,
                ty: rty,
                line,
            };
            before.push(Stmt::Let(
                held,
                Expr {
                    kind: ExprKind::GetField(Box::new(rec(ExprKind::Local(r))), k),
                    ty: fty,
                    line,
                },
            ));
            let (cond, body) = parts(s);
            let mut swap = |e: &mut Expr| rewrite(e, r, k, held);
            exprs_mut(body, &mut swap);
            if let Some(c) = cond {
                swap_in(c, &mut swap);
            }
            if set {
                let value = Expr {
                    kind: ExprKind::Local(held),
                    ty: fty,
                    line,
                };
                let changed =
                    ExprKind::SetField(Box::new(rec(ExprKind::Move(r))), k, Box::new(value));
                let assign = ExprKind::Assign(r, Box::new(rec(changed)));
                after.push(Stmt::Expr(Expr {
                    kind: assign,
                    ty: Type::Unit,
                    line,
                }));
            }
        }
    }
    (before, after)
}

/// Whether loop `s` gives `r` its value: its variable, or a `let` in any
/// block inside it, however deep (a `print`'s value can hold one).
fn made_in(s: &Stmt, r: Local) -> bool {
    let mut copy = s.clone();
    let (own, body) = match &mut copy {
        Stmt::ForRange { var, body, .. } => (*var == r, body),
        Stmt::ForEach {
            var, index, body, ..
        } => (*var == r || *index == r, body),
        Stmt::While { body, .. } => (false, body),
        _ => return true,
    };
    let mut found = own;
    crate::walk::blocks_mut(body, &mut |b| {
        found |= b.stmts.iter().any(|s| match s {
            Stmt::Let(l, _) => *l == r,
            Stmt::LetResult { value, error, .. } => *value == r || *error == r,
            Stmt::ForRange { var, .. } => *var == r,
            Stmt::ForEach { var, index, .. } => *var == r || *index == r,
            _ => false,
        });
    });
    found
}

/// How many different fields of `r` the loop uses.
fn fields_count(body: &Block, cond: Option<&Expr>, r: Local) -> usize {
    fields_only(body, cond, r).map_or(usize::MAX, |f| f.len())
}

fn swap_in(e: &mut Expr, f: &mut impl FnMut(&mut Expr)) {
    let mut b = Block {
        stmts: vec![Stmt::Expr(std::mem::replace(
            e,
            Expr {
                kind: ExprKind::Int(0),
                ty: Type::Unit,
                line: 0,
            },
        ))],
        value: None,
    };
    exprs_mut(&mut b, f);
    if let Some(Stmt::Expr(x)) = b.stmts.pop() {
        *e = x;
    }
}

/// `r.k` read becomes `held`; `r = r with k: v` becomes `held = v`.
fn rewrite(e: &mut Expr, r: Local, k: u32, held: Local) {
    match &mut e.kind {
        ExprKind::GetField(base, at) if *at == k && is_local(base, r) => {
            e.kind = ExprKind::Local(held);
        }
        ExprKind::Assign(l, value) if *l == r => {
            if let ExprKind::SetField(base, at, v) = &mut value.kind
                && *at == k
                && is_local(base, r)
            {
                let v = std::mem::replace(
                    v.as_mut(),
                    Expr {
                        kind: ExprKind::Int(0),
                        ty: Type::Unit,
                        line: 0,
                    },
                );
                e.kind = ExprKind::Assign(held, Box::new(v));
            }
        }
        _ => {}
    }
}

fn is_local(e: &Expr, r: Local) -> bool {
    matches!(e.kind, ExprKind::Local(l) | ExprKind::Move(l) if l == r)
}

/// Whether the loop can leave the function in the middle.
fn leaves_early(body: &Block) -> bool {
    fn stmts(b: &Block) -> bool {
        b.stmts.iter().any(|s| match s {
            Stmt::Return(_) | Stmt::LetResult { .. } => true,
            Stmt::While { body, .. } | Stmt::ForRange { body, .. } | Stmt::ForEach { body, .. } => {
                stmts(body)
            }
            _ => false,
        })
    }
    stmts(body)
        || any_in_block(body, &mut |e| match &e.kind {
            ExprKind::Fail(_) => true,
            ExprKind::Block(b) => stmts(b),
            ExprKind::If(_, a, b) => stmts(a) || stmts(b),
            _ => false,
        })
}

/// The record locals the loop names.
fn records_used(
    body: &Block,
    cond: Option<&Expr>,
    locals: &[Type],
    shapes: &[Shape],
) -> Vec<Local> {
    let mut out: Vec<Local> = Vec::new();
    let mut note = |e: &Expr| {
        if let ExprKind::Local(l) | ExprKind::Move(l) = e.kind
            && let Type::Ref(id) = locals[l as usize]
            && matches!(shapes.get(id as usize), Some(Shape::Record { .. }))
            && !out.contains(&l)
        {
            out.push(l);
        }
        false
    };
    any_in_block(body, &mut note);
    if let Some(c) = cond {
        any_expr(c, &mut note);
    }
    out
}

/// The fields of `r` the loop uses, and whether it sets each, when it
/// uses `r` only as `r.k` and `r = r with k: v`; `None` otherwise.
fn fields_only(body: &Block, cond: Option<&Expr>, r: Local) -> Option<Vec<(u32, bool)>> {
    let mut fields: Vec<(u32, bool)> = Vec::new();
    let (mut uses, mut allowed) = (0usize, 0usize);
    let mut look = |e: &Expr| {
        match &e.kind {
            ExprKind::Local(l) | ExprKind::Move(l) if *l == r => uses += 1,
            ExprKind::GetField(base, k) if is_local(base, r) => {
                allowed += 1;
                if !fields.iter().any(|(f, _)| f == k) {
                    fields.push((*k, false));
                }
            }
            ExprKind::Assign(l, value) if *l == r => match &value.kind {
                ExprKind::SetField(base, k, _) if is_local(base, r) => {
                    allowed += 1;
                    match fields.iter_mut().find(|(f, _)| f == k) {
                        Some(f) => f.1 = true,
                        None => fields.push((*k, true)),
                    }
                }
                _ => uses += 1_000_000,
            },
            _ => {}
        }
        false
    };
    any_in_block(body, &mut look);
    if let Some(c) = cond {
        any_expr(c, &mut look);
    }
    (uses > 0 && uses == allowed).then_some(fields)
}

#[cfg(test)]
mod tests {
    use crate::ir::{ExprKind, Stmt};

    /// `r.line += 1` in a loop over an `inout` record keeps `line` in a
    /// local: the loop sets no field.
    #[test]
    fn a_loop_keeps_a_records_field_in_a_local() {
        let src = "type Rec {\n    line: Int\n    names: [String]\n}\n\nfn feed(inout r: Rec, data: Bytes) {\n    for i in 0..data.len() {\n        if data[i] == 10 { r.line += 1 }\n    }\n}\n\nfn main() {\n    let r = Rec(line: 0, names: [])\n    feed(r, Bytes([10, 1, 10]))\n    print(r.line)\n}\n";
        let c = kavach_driver::compile(src, "t.kav", Default::default());
        let program = crate::lower(&c).expect("lowers");
        // `feed` may be inlined into `main`: every function's loops.
        let mut loops = 0;
        let mut sets_in_loop = false;
        for f in &program.functions {
            let mut look = |b: &crate::ir::Block| {
                for s in &b.stmts {
                    if let Stmt::ForRange { body, .. } = s {
                        loops += 1;
                        sets_in_loop |= crate::walk::any_in_block(body, &mut |e| {
                            matches!(e.kind, ExprKind::SetField(..))
                        });
                    }
                }
            };
            crate::walk::blocks_mut(&mut f.body.clone(), &mut |b| look(b));
        }
        assert!(loops > 0 && !sets_in_loop);
    }

    /// A loop over records reads each one's fields where it is: the loop
    /// gives the variable its value, so nothing is read before the loop
    /// (a crash, 2026-11-17, in `lists-of-records-are-values.kav`).
    #[test]
    fn a_loops_own_record_is_left_alone() {
        let src = "type P {
    x: Int
    y: Int
}

fn main() {
    const ps = [P(x: 1, y: 2), P(x: 3, y: 4)]
    let total = 0
    for p in ps {
        total += p.x
    }
    print(total)
}
";
        let c = kavach_driver::compile(src, "t.kav", Default::default());
        let program = crate::lower(&c).expect("lowers");
        let main = &program.functions[program.entry as usize];
        let mut reads_in_loop = false;
        for s in &main.body.stmts {
            if let Stmt::ForEach { body, .. } = s {
                reads_in_loop |= crate::walk::any_in_block(body, &mut |e| {
                    matches!(e.kind, ExprKind::GetField(..))
                });
            }
        }
        assert!(reads_in_loop, "{:#?}", main.body);
    }
}
