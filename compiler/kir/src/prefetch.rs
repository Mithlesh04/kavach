//! Prefetching the list and map reads a loop will make (to-do item 1). In
//! a loop whose list index or integer map key comes from a pure
//! recurrence and not from what was read,
//!
//! ```text
//! for _ in 0..n { j = j * 48271 % 2147483647; total += xs[j % size] }
//! ```
//!
//! a copy of the recurrence runs `AHEAD` rounds ahead and asks the memory
//! for the item the loop will need then (`RtOp::ListPrefetch`), or for
//! the map bucket its key lands in (`RtOp::MapPrefetch`), so reads that
//! miss the caches overlap instead of waiting one after another.
//!
//! The copy never panics where the loop wouldn't: it adds, subtracts and
//! multiplies with wrapping, and divides only by positive constants. A
//! prefetch of a wrong address is harmless, and nothing it reads is used.
//! Only lists and maps of at least `BIG` items are prefetched: smaller
//! ones stay in the caches, where a prefetch is work for nothing.

use crate::ir::*;
use crate::rt_op::RtOp;
use crate::walk::{any_expr, any_in_block, sets_local};

/// How many rounds ahead the copy runs: measured on `benchmarks/lookups`
/// (16 rounds gave 1.5×, 32 gave 1.66×, 64 no more).
const AHEAD: i128 = 32;

/// Lists shorter than this (2 MB of 8-byte items) are not prefetched.
const BIG: i128 = 262_144;

pub fn prefetch_loops(f: &mut Function, shapes: &[Shape]) {
    let mut locals = std::mem::take(&mut f.locals);
    in_block(&mut f.body, shapes, &mut locals);
    f.locals = locals;
}

fn in_block(b: &mut Block, shapes: &[Shape], locals: &mut Vec<Type>) {
    for s in std::mem::take(&mut b.stmts) {
        let mut s = s;
        if let Stmt::While { body, .. } | Stmt::ForRange { body, .. } | Stmt::ForEach { body, .. } =
            &mut s
        {
            in_block(body, shapes, locals);
        }
        let plan = match &s {
            Stmt::While { body, .. } | Stmt::ForRange { body, .. } => plan(body, shapes, locals),
            _ => None,
        };
        let Some(p) = plan else {
            b.stmts.push(s);
            continue;
        };
        let (ahead, big, k) = (
            new_local(locals, Type::I64),
            new_local(locals, Type::Bool),
            new_local(locals, Type::I64),
        );
        let line = p.step.line;
        let step = || ahead_step(p.r, ahead, &p.step);
        let list = at(ExprKind::Local(p.list), locals[p.list as usize], line);
        // Before the loop: the copy starts `AHEAD` rounds ahead, and the
        // list's length decides whether prefetching pays.
        b.stmts.push(Stmt::Let(ahead, local(p.r, line)));
        b.stmts.push(Stmt::ForRange {
            var: k,
            start: int(0, line),
            end: int(AHEAD, line),
            inclusive: false,
            body: Block {
                stmts: vec![step()],
                value: None,
            },
        });
        let (len_op, fetch_op) = if p.map {
            (RtOp::MapLen, RtOp::MapPrefetch)
        } else {
            (RtOp::ListLen, RtOp::ListPrefetch)
        };
        let len = at(ExprKind::Rt(len_op, vec![list.clone()]), Type::I64, line);
        let enough = at(
            ExprKind::Binary(BinOp::Ge, Box::new(len), Box::new(int(BIG, line))),
            Type::Bool,
            line,
        );
        b.stmts.push(Stmt::Let(big, enough));
        // In the loop, after the real step: one step of the copy, and a
        // prefetch of the item it points at.
        let index = renamed(&p.index, p.r, ahead);
        let fetch = at(ExprKind::Rt(fetch_op, vec![list, index]), Type::Unit, line);
        let when_big = at(
            ExprKind::If(
                Box::new(at(ExprKind::Local(big), Type::Bool, line)),
                Block {
                    stmts: vec![step(), Stmt::Expr(fetch)],
                    value: None,
                },
                Block::default(),
            ),
            Type::Unit,
            line,
        );
        if let Stmt::While { body, .. } | Stmt::ForRange { body, .. } = &mut s {
            body.stmts.insert(p.at + 1, Stmt::Expr(when_big));
        }
        b.stmts.push(s);
    }
}

/// A loop that can be prefetched: its recurrence `r = step` (statement
/// `at` of the body), and the list read at an index made from `r`, or the
/// map (`map`) read at such a key.
struct Plan {
    r: Local,
    step: Expr,
    at: usize,
    list: Local,
    index: Expr,
    map: bool,
}

fn plan(body: &Block, shapes: &[Shape], locals: &[Type]) -> Option<Plan> {
    for (at, s) in body.stmts.iter().enumerate() {
        let Stmt::Expr(Expr {
            kind: ExprKind::Assign(r, step),
            ..
        }) = s
        else {
            continue;
        };
        let r = *r;
        let set_once = body.stmts.iter().filter(|s| sets(s, r)).count() == 1;
        if locals[r as usize] != Type::I64
            || !set_once
            || !uses(step, r)
            || !pure(step, r, body)
            || nested_sets(body, r)
        {
            continue;
        }
        let later = Block {
            stmts: body.stmts[at + 1..].to_vec(),
            value: None,
        };
        let shape = |t: Type| match t {
            Type::Ref(id) => shapes.get(id as usize),
            _ => None,
        };
        let mut found = None;
        any_in_block(&later, &mut |e| {
            // `list[i]`, or a map read with an integer key.
            let (base, index, map) = match &e.kind {
                ExprKind::Index(base, index) if matches!(shape(base.ty), Some(Shape::List(_))) => {
                    (&**base, &**index, false)
                }
                ExprKind::Rt(RtOp::MapGet | RtOp::MapGetOr | RtOp::MapContains, args)
                    if matches!(shape(args[0].ty), Some(Shape::Map(Type::I64, _))) =>
                {
                    (&args[0], &args[1], true)
                }
                _ => return false,
            };
            if let ExprKind::Local(list) = base.kind
                && !sets_local(body, list)
                && uses(index, r)
                && pure(index, r, body)
            {
                found = Some((list, index.clone(), map));
                return true;
            }
            false
        });
        if let Some((list, index, map)) = found {
            return Some(Plan {
                r,
                step: (**step).clone(),
                at,
                list,
                index,
                map,
            });
        }
    }
    None
}

/// Whether statement `s` sets `r` at its top.
fn sets(s: &Stmt, r: Local) -> bool {
    match s {
        Stmt::Expr(Expr {
            kind: ExprKind::Assign(x, _),
            ..
        }) => *x == r,
        Stmt::Let(x, _) => *x == r,
        _ => false,
    }
}

/// Whether `r` is set anywhere but at the top of the body: in a nested
/// block or loop, where it couldn't be followed.
fn nested_sets(body: &Block, r: Local) -> bool {
    body.stmts.iter().any(|s| {
        let inner = match s {
            Stmt::While { body, .. } | Stmt::ForRange { body, .. } | Stmt::ForEach { body, .. } => {
                sets_local(body, r)
            }
            _ => false,
        };
        inner
            || any_in_block(
                &Block {
                    stmts: vec![s.clone()],
                    value: None,
                },
                &mut |e| match &e.kind {
                    ExprKind::If(_, a, b) => sets_local(a, r) || sets_local(b, r),
                    ExprKind::Block(b) => sets_local(b, r),
                    ExprKind::Move(x) => *x == r,
                    _ => false,
                },
            )
    })
}

fn uses(e: &Expr, r: Local) -> bool {
    any_expr(e, &mut |x| matches!(x.kind, ExprKind::Local(l) if l == r))
}

/// Whether `e` is made only of `r`, whole-number constants, locals the
/// loop never sets, `+ - *`, and `/ %` by positive constants: what the
/// copy can compute without ever panicking.
fn pure(e: &Expr, r: Local, body: &Block) -> bool {
    if e.ty != Type::I64 {
        return false;
    }
    match &e.kind {
        ExprKind::Int(_) => true,
        ExprKind::Local(l) => *l == r || !sets_local(body, *l),
        ExprKind::Binary(BinOp::Add | BinOp::Sub | BinOp::Mul, a, b) => {
            pure(a, r, body) && pure(b, r, body)
        }
        ExprKind::Binary(BinOp::Div | BinOp::Rem, a, b) => {
            matches!(b.kind, ExprKind::Int(n) if n > 0) && pure(a, r, body)
        }
        // What `ranges.rs` proved can't overflow for every value the
        // recurrence takes, so for the copy's values too.
        ExprKind::Fast(FastOp::Add | FastOp::Sub | FastOp::Mul, a, b) => {
            pure(a, r, body) && pure(b, r, body)
        }
        ExprKind::Fast(_, a, b) => matches!(b.kind, ExprKind::Int(n) if n > 0) && pure(a, r, body),
        _ => false,
    }
}

/// `ahead = step`, with `r` read as `ahead` and `+ - *` wrapping.
fn ahead_step(r: Local, ahead: Local, step: &Expr) -> Stmt {
    let line = step.line;
    Stmt::Expr(at(
        ExprKind::Assign(ahead, Box::new(renamed(step, r, ahead))),
        Type::Unit,
        line,
    ))
}

/// `e` with `r` read as `ahead`, and `+ - *` wrapping instead of checked.
fn renamed(e: &Expr, r: Local, ahead: Local) -> Expr {
    let kind = match &e.kind {
        ExprKind::Local(l) if *l == r => ExprKind::Local(ahead),
        ExprKind::Binary(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul), a, b) => {
            let wrapping = match op {
                BinOp::Add => RtOp::WrappingAdd,
                BinOp::Sub => RtOp::WrappingSub,
                _ => RtOp::WrappingMul,
            };
            ExprKind::Rt(wrapping, vec![renamed(a, r, ahead), renamed(b, r, ahead)])
        }
        ExprKind::Binary(op, a, b) => ExprKind::Binary(
            *op,
            Box::new(renamed(a, r, ahead)),
            Box::new(renamed(b, r, ahead)),
        ),
        ExprKind::Fast(op, a, b) => ExprKind::Fast(
            *op,
            Box::new(renamed(a, r, ahead)),
            Box::new(renamed(b, r, ahead)),
        ),
        other => other.clone(),
    };
    at(kind, e.ty, e.line)
}

fn new_local(locals: &mut Vec<Type>, ty: Type) -> Local {
    locals.push(ty);
    (locals.len() - 1) as Local
}

fn at(kind: ExprKind, ty: Type, line: u32) -> Expr {
    Expr { kind, ty, line }
}

fn local(l: Local, line: u32) -> Expr {
    at(ExprKind::Local(l), Type::I64, line)
}

fn int(n: i128, line: u32) -> Expr {
    at(ExprKind::Int(n), Type::I64, line)
}

#[cfg(test)]
mod tests {
    use crate::ir::*;
    use crate::rt_op::RtOp;
    use crate::walk::any_in_block;

    fn prefetches(src: &str) -> (bool, String) {
        let c = kavach_driver::compile(src, "main.kav", Default::default());
        let p = crate::lower(&c).expect("lowers");
        let main = &p.functions[p.entry as usize];
        let found = any_in_block(&main.body, &mut |e| {
            matches!(
                e.kind,
                ExprKind::Rt(RtOp::ListPrefetch | RtOp::MapPrefetch, _)
            )
        });
        (found, format!("{:#?}", main.body.stmts))
    }

    #[test]
    fn a_recurrence_index_is_prefetched() {
        let (found, body) = prefetches(
            "fn main() {
    let xs: [Int] = []
    for i in 0..1000 {
        xs.push(i)
    }
    let j = 7
    let total = 0
    for _ in 0..100 {
        j = j * 48271 % 2147483647
        total += xs[j % 1000]
    }
    print(total)
}
",
        );
        assert!(found, "{body}");
    }

    #[test]
    fn a_recurrence_map_key_is_prefetched() {
        let (found, body) = prefetches(
            "fn main() {
    let m: [Int: Int] = [:]
    for i in 0..1000 {
        m[i] = i
    }
    let j = 7
    let total = 0
    for _ in 0..100 {
        j = j * 48271 % 2147483647
        total += m[j % 1000] ?? 0
    }
    print(total)
}
",
        );
        assert!(found, "{body}");
    }

    #[test]
    fn an_index_read_from_the_list_is_not_prefetched() {
        // The next index is what was read: nothing to run ahead of.
        let (found, _) = prefetches(
            "fn main() {
    let xs: [Int] = []
    for i in 0..1000 {
        xs.push((i * 7) % 1000)
    }
    let j = 0
    for _ in 0..100 {
        j = xs[j]
    }
    print(j)
}
",
        );
        assert!(!found);
    }
}
