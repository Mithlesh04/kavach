//! Inlining small functions into their callers, so a call costs nothing.
//! A call becomes a block: a `let` for each parameter, then a copy of the
//! function's body, with the function's locals renumbered as new locals of
//! the caller. Everything the program can see stays the same: arguments
//! are computed once and in order, each check keeps its line, and panics
//! read the same.
//!
//! A recursive function is inlined into itself: `fib(n - 1)` becomes a
//! copy of `fib`'s body, which still calls `fib`. Each level halves the
//! real calls without changing what is computed. A small recursive
//! function gets `RECURSION_LEVELS` levels, as long as it stays smaller
//! than `RECURSION_SIZE`.
//!
//! First, `lift_returns` turns early returns into `if`/`else`, so that a
//! function body has no `return` left and its value is simply the body's
//! value. Functions that still return early, or can fail, are not inlined.
//!
//! Last, the calls that are left get the callee's quick exit in front of
//! them (`guard.rs`), so a recursion's leaves need no call.

use crate::ir::*;
use crate::walk::{any_in_block, exprs_mut, locals_mut};

/// Functions bigger than this many expressions are not inlined.
const MAX_SIZE: usize = 40;

/// The size for a function that takes or gives a record or tuple: a call
/// makes an object for each, which inlining saves, as an item of a flat
/// list is then read and written in place. Measured on a `map` of a
/// million particles through a `gpu fn` of 60 expressions: 12.4 → 6.0 s.
const MAX_FIXED_SIZE: usize = 120;

/// How many times a recursive function is inlined into itself, and the
/// size it may grow to. Each level copies the body as it is so far, so
/// `fib`'s 2 calls become 4, then 16. Measured on `fib`: the second
/// level makes it 15% faster; a third (256 calls) gains nothing.
const RECURSION_LEVELS: usize = 2;
const RECURSION_SIZE: usize = 400;

/// The size for a recursion `unrecurse.rs` turned into a loop, inlined
/// as its loop and a call for the code as written: in the caller, the
/// arguments it is called with bound the loop. Measured on the everyday
/// `stacks` benchmark: 190 → 113 ms (C++ 96).
const MAX_LOOP_SIZE: usize = 120;

/// What inlining a function needs: its parameters, locals and body.
type Inlinable = (Vec<Local>, Vec<Type>, Block);

pub fn inline_calls(functions: &mut [Function], shapes: &[Shape]) {
    for f in functions.iter_mut() {
        let ret = f.ret;
        lift_returns(&mut f.body, ret);
    }
    // The bodies as they are before any inlining: each call is inlined one
    // level, so recursion stops.
    let bodies: Vec<Option<Inlinable>> =
        (functions.iter().enumerate())
            .map(|(id, f)| match loop_half(f, id as FuncId) {
                Some(body) => (size(&body) <= MAX_LOOP_SIZE)
                    .then(|| (f.params.clone(), f.locals.clone(), body)),
                None => (inlinable(f, shapes))
                    .then(|| (f.params.clone(), f.locals.clone(), f.body.clone())),
            })
            .collect();
    for (id, f) in functions.iter_mut().enumerate() {
        let looped = loop_half(f, id as FuncId).is_some();
        let locals = &mut f.locals;
        exprs_mut(&mut f.body, &mut |e| {
            let ExprKind::Call(callee, _) = &e.kind else {
                return;
            };
            // Never a recursion turned loop into its own code as written:
            // each level would run the loop again.
            if looped && *callee as usize == id {
                return;
            }
            let Some((params, callee_locals, body)) = &bodies[*callee as usize] else {
                return;
            };
            let ExprKind::Call(_, args) = std::mem::replace(&mut e.kind, ExprKind::None) else {
                unreachable!("a call");
            };
            e.kind = ExprKind::Block(copy_in(locals, params, callee_locals, body, args));
        });
    }
    for (id, f) in functions.iter_mut().enumerate() {
        if bodies[id].is_some() {
            unroll(f, id as FuncId);
        }
    }
    guard::add_guards(functions);
}

/// Inlines the function `f` (number `id`) into its own calls, level by
/// level, while it stays small (see `RECURSION_LEVELS`). The first level
/// was done with the other functions.
fn unroll(f: &mut Function, id: FuncId) {
    // A recursion `unrecurse.rs` already turned into a loop keeps its
    // shape: the self-call left in the fallback is the code as written.
    if matches!(
        f.body.value.as_deref(),
        Some(Expr {
            kind: ExprKind::Fallback(..),
            ..
        })
    ) {
        return;
    }
    for _ in 1..RECURSION_LEVELS {
        let calls_itself = any_in_block(
            &f.body,
            &mut |e| matches!(e.kind, ExprKind::Call(c, _) if c == id),
        );
        if !calls_itself || 2 * size(&f.body) > RECURSION_SIZE {
            return;
        }
        let (params, callee_locals, body) = (f.params.clone(), f.locals.clone(), f.body.clone());
        let locals = &mut f.locals;
        exprs_mut(&mut f.body, &mut |e| {
            let ExprKind::Call(callee, _) = &e.kind else {
                return;
            };
            if *callee != id {
                return;
            }
            let ExprKind::Call(_, args) = std::mem::replace(&mut e.kind, ExprKind::None) else {
                unreachable!("a call");
            };
            e.kind = ExprKind::Block(copy_in(locals, &params, &callee_locals, &body, args));
        });
    }
}

/// For a recursion `unrecurse.rs` turned into a loop (its body is the
/// loop, falling back to the code as written), the body to inline into
/// callers: the loop, falling back to a call. The loop only reads the
/// parameters and has no effects, so the call starts the same work again.
fn loop_half(f: &Function, id: FuncId) -> Option<Block> {
    let value = f.body.value.as_deref()?;
    let ExprKind::Fallback(fast, slow) = &value.kind else {
        return None;
    };
    let calls_itself = |b: &Block| {
        any_in_block(
            b,
            &mut |e| matches!(e.kind, ExprKind::Call(c, _) if c == id),
        )
    };
    if !f.body.stmts.is_empty()
        || !calls_itself(slow)
        || calls_itself(fast)
        || f.params.iter().any(|p| crate::walk::sets_local(fast, *p))
    {
        return None;
    }
    let args = (f.params.iter())
        .map(|p| Expr {
            kind: ExprKind::Local(*p),
            ty: f.locals[*p as usize],
            line: value.line,
        })
        .collect();
    let call = Expr {
        kind: ExprKind::Call(id, args),
        ty: value.ty,
        line: value.line,
    };
    let slow = Block {
        stmts: Vec::new(),
        value: Some(Box::new(call)),
    };
    Some(Block {
        stmts: Vec::new(),
        value: Some(Box::new(Expr {
            kind: ExprKind::Fallback(fast.clone(), slow),
            ty: value.ty,
            line: value.line,
        })),
    })
}

/// The number of expressions in `block`.
fn size(block: &Block) -> usize {
    let mut n = 0;
    any_in_block(block, &mut |_| {
        n += 1;
        false
    });
    n
}

/// The body of a call to a function with these `params`, `locals` and
/// `body`, with `args`, as a block in a caller whose locals are `into`.
fn copy_in(
    into: &mut Vec<Type>,
    params: &[Local],
    locals: &[Type],
    body: &Block,
    args: Vec<Expr>,
) -> Block {
    let base = into.len() as Local;
    into.extend_from_slice(locals);
    let mut copy = body.clone();
    locals_mut(&mut copy, &mut |l| *l += base);
    let mut stmts = Vec::new();
    for (p, a) in params.iter().zip(args) {
        let p = p + base;
        if a.ty == into[p as usize] && constant(&a, &mut copy, p) {
            exprs_mut(&mut copy, &mut |e| {
                if matches!(e.kind, ExprKind::Local(l) | ExprKind::Move(l) if l == p) {
                    e.kind = a.kind.clone();
                }
            });
        } else {
            stmts.push(Stmt::Let(p, a));
        }
    }
    stmts.append(&mut copy.stmts);
    Block {
        stmts,
        value: copy.value,
    }
}

/// Whether the argument `a` of parameter `p` can be written in place of
/// each read of `p` in `body`: a number, `Bool` or `Char`; a local of the
/// caller, which the inlined body can't change (so a flat list's item is
/// read in place, not boxed for the call); or text read once and not in a
/// loop (each read of a text constant makes a text
/// object, except as a map key, `kv_map_get_text`). Parameters are never
/// set (spec 3.1), but a loop over `p` mentions it outside expressions.
fn constant(a: &Expr, body: &mut Block, p: Local) -> bool {
    let text = match a.kind {
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Local(_) => false,
        ExprKind::Str(_) => true,
        _ => return false,
    };
    let mut mentions = 0;
    locals_mut(body, &mut |l| mentions += usize::from(*l == p));
    let mut reads = 0;
    any_in_block(body, &mut |e| {
        reads += usize::from(matches!(e.kind, ExprKind::Local(l) | ExprKind::Move(l) if l == p));
        false
    });
    mentions == reads && (!text || (reads <= 1 && !has_loop(body)))
}

fn has_loop(body: &Block) -> bool {
    let direct = |b: &Block| {
        b.stmts.iter().any(|s| {
            matches!(
                s,
                Stmt::While { .. } | Stmt::ForRange { .. } | Stmt::ForEach { .. }
            )
        })
    };
    direct(body)
        || any_in_block(body, &mut |e| match &e.kind {
            ExprKind::Block(b) => direct(b),
            ExprKind::If(_, a, b) | ExprKind::Fallback(a, b) => direct(a) || direct(b),
            _ => false,
        })
}

/// Whether `f` can be inlined: small (bigger when it takes or gives a
/// record or tuple), can't fail, and has no `return` or
/// `fail` left after `lift_returns`.
fn inlinable(f: &Function, shapes: &[Shape]) -> bool {
    // A C function's code is in its library, not here.
    if f.fails || f.external.is_some() || has_return(&f.body) {
        return false;
    }
    let fails = any_in_block(&f.body, &mut |e| matches!(e.kind, ExprKind::Fail(_)));
    let fixed = |t: Type| {
        matches!(t, Type::Ref(id) if matches!(
            shapes.get(id as usize),
            Some(Shape::Record { .. } | Shape::Tuple(_))
        ))
    };
    let takes_fixed = fixed(f.ret) || f.params.iter().any(|p| fixed(f.locals[*p as usize]));
    let max = if takes_fixed {
        MAX_FIXED_SIZE
    } else {
        MAX_SIZE
    };
    size(&f.body) <= max && !fails
}

fn has_return(block: &Block) -> bool {
    fn stmts(block: &Block) -> bool {
        block.stmts.iter().any(|s| match s {
            Stmt::Return(_) => true,
            Stmt::While { body, .. } | Stmt::ForRange { body, .. } | Stmt::ForEach { body, .. } => {
                stmts(body)
            }
            _ => false,
        })
    }
    stmts(block)
        || any_in_block(block, &mut |e| match &e.kind {
            ExprKind::If(_, then, otherwise) | ExprKind::Fallback(then, otherwise) => {
                stmts(then) || stmts(otherwise)
            }
            ExprKind::Block(b) => stmts(b),
            _ => false,
        })
}

/// Rewrites `if c { …; return v }; rest; value` into
/// `if c { …; v } else { rest; value }`, and a final `return v` into the
/// value `v`, in a function body of type `ret`. Returns inside loops stay.
pub fn lift_returns(block: &mut Block, ret: Type) {
    if block.value.is_none()
        && let Some(Stmt::Return(v)) = block.stmts.last()
        && (v.is_some() || ret == Type::Unit)
    {
        let Some(Stmt::Return(v)) = block.stmts.pop() else {
            unreachable!("a return");
        };
        block.value = v.map(Box::new);
    }
    let Some(i) = block.stmts.iter().position(early_return) else {
        return;
    };
    let rest: Vec<Stmt> = block.stmts.drain(i + 1..).collect();
    let Some(Stmt::Expr(e)) = block.stmts.pop() else {
        unreachable!("an early return");
    };
    let ExprKind::If(cond, mut then, _) = e.kind else {
        unreachable!("an if");
    };
    lift_returns(&mut then, ret);
    let mut otherwise = Block {
        stmts: rest,
        value: block.value.take(),
    };
    lift_returns(&mut otherwise, ret);
    block.value = Some(Box::new(Expr {
        kind: ExprKind::If(cond, then, otherwise),
        ty: ret,
        line: e.line,
    }));
}

/// `if c { …; return v }` with no `else`, whose branch ends in its only
/// `return`.
fn early_return(s: &Stmt) -> bool {
    let Stmt::Expr(e) = s else { return false };
    let ExprKind::If(_, then, otherwise) = &e.kind else {
        return false;
    };
    let empty = otherwise.stmts.is_empty() && otherwise.value.is_none();
    let ends = matches!(then.stmts.last(), Some(Stmt::Return(_))) && then.value.is_none();
    let returns = then
        .stmts
        .iter()
        .filter(|s| matches!(s, Stmt::Return(_)))
        .count();
    empty && ends && returns == 1
}

mod guard;
#[cfg(test)]
mod tests;
