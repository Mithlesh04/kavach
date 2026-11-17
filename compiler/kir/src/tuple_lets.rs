//! A tuple made only to be taken apart: a function with `inout`
//! parameters gives back a tuple of its value and the parameters' last
//! values (`lower_inout.rs`), which the caller reads item by item. Once
//! the call is inlined, `let t = { …; (a, b) }` and `t[0]`, `t[1]` need no
//! tuple: the block's statements go first, each item into a local of its
//! own, and the reads read those (Sakshi's S11: an `inout` call cost about
//! 130 ns, nearly all of it the tuple's object).

use crate::ir::*;
use crate::walk::{any_in_block, blocks_mut, exprs_mut};

pub fn split_tuple_lets(f: &mut Function, shapes: &[Shape]) {
    let is_tuple = |t: Type| matches!(t, Type::Ref(id) if matches!(shapes.get(id as usize), Some(Shape::Tuple(_))));
    // Tuple locals read only as `t[k]`, and the reads of each.
    let candidates: Vec<Local> = (0..f.locals.len() as Local)
        .filter(|l| is_tuple(f.locals[*l as usize]) && only_indexed(&f.body, *l))
        .collect();
    if candidates.is_empty() {
        return;
    }
    let mut locals = std::mem::take(&mut f.locals);
    let mut split: Vec<(Local, Vec<Local>)> = Vec::new();
    blocks_mut(&mut f.body, &mut |b| {
        let mut i = 0;
        while i < b.stmts.len() {
            let Stmt::Let(t, value) = &mut b.stmts[i] else {
                i += 1;
                continue;
            };
            let t = *t;
            if !candidates.contains(&t) || split.iter().any(|(s, _)| *s == t) {
                i += 1;
                continue;
            }
            let (mut first, items) = match std::mem::replace(&mut value.kind, ExprKind::Int(0)) {
                ExprKind::Tuple(items) => (Vec::new(), items),
                ExprKind::Block(Block {
                    stmts,
                    value: Some(v),
                }) if matches!(v.kind, ExprKind::Tuple(_)) => {
                    let ExprKind::Tuple(items) = v.kind else {
                        unreachable!("checked")
                    };
                    (stmts, items)
                }
                other => {
                    value.kind = other;
                    i += 1;
                    continue;
                }
            };
            let mut parts = Vec::new();
            for item in items {
                let l = locals.len() as Local;
                locals.push(item.ty);
                first.push(Stmt::Let(l, item));
                parts.push(l);
            }
            split.push((t, parts));
            let n = first.len();
            b.stmts.splice(i..=i, first);
            i += n;
        }
    });
    f.locals = locals;
    if split.is_empty() {
        return;
    }
    exprs_mut(&mut f.body, &mut |e| {
        if let ExprKind::Index(base, at) = &e.kind
            && let (ExprKind::Local(t), ExprKind::Int(k)) = (&base.kind, &at.kind)
            && let Some((_, parts)) = split.iter().find(|(s, _)| s == t)
            && let Some(&part) = parts.get(*k as usize)
        {
            e.kind = ExprKind::Local(part);
        }
    });
}

/// Whether every use of local `t` in `body` is `t[k]` with a constant `k`
/// (and its one `let`).
fn only_indexed(body: &Block, t: Local) -> bool {
    let mut uses = 0usize;
    let mut indexed = 0usize;
    any_in_block(body, &mut |e| {
        match &e.kind {
            ExprKind::Local(l) | ExprKind::Move(l) if *l == t => uses += 1,
            ExprKind::Index(base, at)
                if matches!(base.kind, ExprKind::Local(l) if l == t)
                    && matches!(at.kind, ExprKind::Int(_)) =>
            {
                indexed += 1
            }
            ExprKind::Assign(l, _) if *l == t => uses += 1_000_000,
            _ => {}
        }
        false
    });
    uses > 0 && uses == indexed
}

#[cfg(test)]
mod tests {
    use crate::ir::ExprKind;

    /// After `bump(n)` with `inout n: Int` is inlined, no tuple is built.
    #[test]
    fn an_inout_call_builds_no_tuple() {
        let src = "fn bump(inout n: Int) {\n    n += 1\n}\n\nfn main() {\n    let n = 0\n    for _ in 0..10 {\n        bump(n)\n    }\n    print(n)\n}\n";
        let c = kavach_driver::compile(src, "t.kav", Default::default());
        let program = crate::lower(&c).expect("lowers");
        let main = &program.functions[program.entry as usize];
        let tuples =
            crate::walk::any_in_block(&main.body, &mut |e| matches!(e.kind, ExprKind::Tuple(_)));
        assert!(!tuples, "{:#?}", main.body);
    }
}
