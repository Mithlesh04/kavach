//! Values nobody reads are not computed (spec 1.4): `const [head,
//! ...rest] = names` with `rest` unused makes no list. A `let` whose local
//! is never read is dropped when its value is quiet: it changes nothing,
//! prints nothing and can't panic, so the program can't tell. Dropping
//! one can leave another unread, so this runs until nothing changes.

use crate::ir::*;
use crate::rt_op::RtOp;
use crate::walk::{any_in_block, blocks_mut};

pub fn drop_unused(f: &mut Function) {
    loop {
        let read = reads(&f.body, f.locals.len());
        let mut dropped = false;
        blocks_mut(&mut f.body, &mut |b| {
            let before = b.stmts.len();
            b.stmts.retain(|s| match s {
                Stmt::Let(l, e) => read[*l as usize] || !quiet(e),
                _ => true,
            });
            dropped |= b.stmts.len() != before;
        });
        if !dropped {
            return;
        }
    }
}

/// Which locals something reads.
fn reads(body: &Block, n: usize) -> Vec<bool> {
    let mut read = vec![false; n];
    any_in_block(body, &mut |e| {
        if let ExprKind::Local(l) | ExprKind::Move(l) = e.kind {
            read[l as usize] = true;
        }
        false
    });
    // A loop over a list reads the local holding it.
    blocks_mut(&mut body.clone(), &mut |b| {
        for s in &b.stmts {
            if let Stmt::ForEach { list, .. } = s {
                read[*list as usize] = true;
            }
        }
    });
    read
}

/// Whether computing `e` has no effect a program could see, and can't
/// panic.
fn quiet(e: &Expr) -> bool {
    let all = |xs: &[Expr]| xs.iter().all(quiet);
    match &e.kind {
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Str(_)
        | ExprKind::Local(_)
        | ExprKind::None => true,
        ExprKind::Some(x)
        | ExprKind::IsSome(x)
        | ExprKind::GetField(x, _)
        | ExprKind::HasVariant(x, _) => quiet(x),
        ExprKind::Fast(_, a, b) | ExprKind::And(a, b) | ExprKind::Or(a, b) => quiet(a) && quiet(b),
        ExprKind::Binary(op, a, b) => {
            !matches!(
                op,
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::Shl
            ) && quiet(a)
                && quiet(b)
        }
        ExprKind::List(xs) | ExprKind::Tuple(xs) | ExprKind::Format(xs) => all(xs),
        ExprKind::Record { fields, .. } => all(fields),
        ExprKind::If(c, a, b) => quiet(c) && quiet_block(a) && quiet_block(b),
        ExprKind::Block(b) => quiet_block(b),
        ExprKind::Rt(op, args) => {
            matches!(
                op,
                RtOp::ListSkip
                    | RtOp::ListLen
                    | RtOp::ListIsEmpty
                    | RtOp::MapGet
                    | RtOp::MapGetOr
                    | RtOp::MapContains
                    | RtOp::MapLen
                    | RtOp::MapIsEmpty
            ) && all(args)
        }
        _ => false,
    }
}

fn quiet_block(b: &Block) -> bool {
    b.stmts.is_empty() && b.value.as_ref().is_none_or(|v| quiet(v))
}

#[cfg(test)]
mod tests {
    use crate::ir::*;
    use crate::rt_op::RtOp;
    use crate::walk::any_in_block;

    /// Whether `main` of `src`, lowered, has an expression `test` holds for.
    fn main_has(src: &str, test: fn(&Expr) -> bool) -> bool {
        let c = kavach_driver::compile(src, "main.kav", Default::default());
        let p = crate::lower(&c).expect("lowers");
        any_in_block(&p.functions[p.entry as usize].body, &mut |e| test(e))
    }

    fn skip(e: &Expr) -> bool {
        matches!(e.kind, ExprKind::Rt(RtOp::ListSkip, _))
    }

    #[test]
    fn an_unused_rest_makes_no_list() {
        let unused = "fn main() {
    const [head, ..._rest] = [1, 2, 3]
    print(head)
}
";
        assert!(!main_has(unused, skip));
        let used = "fn main() {
    const [head, ...rest] = [1, 2, 3]
    print(rest)
}
";
        assert!(main_has(used, skip));
    }

    #[test]
    fn a_value_that_can_panic_is_kept() {
        // `xs[9]` panics: dropping it would change what the program does.
        let src = "fn main() {
    const xs = [1]
    const _x = xs[9]
    print(1)
}
";
        assert!(main_has(src, |e| matches!(e.kind, ExprKind::Index(..))));
    }
}
