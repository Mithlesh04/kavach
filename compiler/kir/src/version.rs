//! Loop versioning. A counted loop that multiplies by a local it never
//! changes keeps its overflow checks when that local could be anything:
//! `total = (total + i * seed) % m` must check `i * seed` and the sum. So
//! the loop gets a copy that runs when a test before it shows the local is
//! in `0..=SMALL`; `ranges.rs` reads the test (`branch_facts`) and removes
//! the checks that can't fail there. Otherwise the loop as written runs,
//! so a check still fails where the program says (spec 5.6).
//!
//! A copy is kept only when the ranges remove checks from it, found by
//! trying: each candidate is versioned in a copy of the function, whose
//! ranges are then proven.

use crate::ir::*;
use crate::walk;

mod own;

/// The largest value of the local in the copy: `i * x` stays far from
/// overflow in any loop of fewer than 2³² rounds.
const SMALL: i128 = (1 << 31) - 1;

/// A loop with more expressions than this isn't copied.
const MOST: usize = 120;

/// How a loop is versioned.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A copy, run when the local is small.
    Plain,
    /// A copy with locals of its own (`own.rs`), run when the local is
    /// small and the loop's accumulators aren't negative, so the ranges
    /// in it don't mix with the loop as written; then, with `plain`, the
    /// `Plain` copy, and the loop as written last.
    Own { plain: bool },
}

/// Versions the loops of `f` where that removes checks or signs.
pub fn version_loops(f: &mut Function) {
    let mut chosen = Vec::new();
    for (n, x) in candidates(f) {
        // The cost left in the copy, and in the loop as written.
        let trial = |kind| {
            let mut trial = copy(f);
            wrap(&mut trial, &[(n, x, kind)]);
            crate::ranges::prove_ranges(&mut trial);
            branches(&trial.body, x).map(|(fast, written)| (cost(&fast), cost(&written)))
        };
        let plain = trial(Kind::Plain).filter(|(fast, written)| fast < written);
        let own = trial(Kind::Own { plain: false }).filter(|(fast, written)| fast < written);
        let kind = match (plain, own) {
            (Some(p), Some(o)) if o.0 < p.0 => Kind::Own { plain: true },
            (None, Some(_)) => Kind::Own { plain: false },
            (Some(_), _) => Kind::Plain,
            (None, None) => continue,
        };
        chosen.push((n, x, kind));
    }
    if !chosen.is_empty() {
        wrap(f, &chosen);
    }
}

/// Each loop worth trying, by its number in `blocks_mut`'s order, with the
/// local to test: the first unchanged integer local a checked `*` reads.
fn candidates(f: &mut Function) -> Vec<(usize, Local)> {
    let mut found = Vec::new();
    let mut n = 0;
    walk::blocks_mut(&mut f.body, &mut |b| {
        for s in &b.stmts {
            let Stmt::ForRange { var, body, .. } = s else {
                continue;
            };
            if let Some(x) = invariant_factor(*var, body) {
                found.push((n, x));
            }
            n += 1;
        }
    });
    found
}

/// The local to test for loop `body`: an integer local it multiplies by
/// with a check and never sets. `None` for a loop too big to copy.
fn invariant_factor(var: Local, body: &Block) -> Option<Local> {
    let mut size = 0;
    let mut factor = None;
    walk::any_in_block(body, &mut |e| {
        size += 1;
        if let ExprKind::Binary(BinOp::Mul, a, b) = &e.kind
            && e.ty.is_int()
            && factor.is_none()
        {
            factor = [a, b].into_iter().find_map(|side| match side.kind {
                ExprKind::Local(l) if l != var && side.ty == Type::I64 => Some(l),
                _ => None,
            });
        }
        false
    });
    let x = factor?;
    (size <= MOST && !walk::sets_local(body, x)).then_some(x)
}

/// Versions each loop numbered in `which` as its `Kind` says. Inner blocks
/// are done first, so the numbers stay those of `candidates`.
fn wrap(f: &mut Function, which: &[(usize, Local, Kind)]) {
    let Function { body, locals, .. } = f;
    let mut n = 0;
    walk::blocks_mut(body, &mut |b| {
        for s in &mut b.stmts {
            if !matches!(s, Stmt::ForRange { .. }) {
                continue;
            }
            if let Some(&(_, x, kind)) = which.iter().find(|(k, ..)| *k == n) {
                let written = std::mem::replace(s, Stmt::Break);
                *s = versioned(locals, written, x, kind);
            }
            n += 1;
        }
    });
}

/// Loop `written`, versioned on local `x` as `kind` says.
fn versioned(locals: &mut Vec<Type>, written: Stmt, x: Local, kind: Kind) -> Stmt {
    let line = match &written {
        Stmt::ForRange { start, .. } => start.line,
        _ => 0,
    };
    let block = |stmts: Vec<Stmt>| Block { stmts, value: None };
    let choice = |test, fast, otherwise| {
        Stmt::Expr(Expr {
            kind: ExprKind::If(Box::new(test), fast, otherwise),
            ty: Type::Unit,
            line,
        })
    };
    let plain = |written: Stmt| {
        let fast = block(vec![written.clone()]);
        choice(in_range(x, line), fast, block(vec![written]))
    };
    let Kind::Own { plain: then_plain } = kind else {
        return plain(written);
    };
    let Some((copy, accumulators)) = own::own_locals(locals, &written) else {
        return plain(written);
    };
    let mut test = in_range(x, line);
    let mut fast = Vec::new();
    let mut back = Vec::new();
    for &(old, new) in &accumulators {
        let (ty, at) = (locals[old as usize], |kind, ty| Expr { kind, ty, line });
        let not_negative = ExprKind::Binary(
            BinOp::Ge,
            Box::new(at(ExprKind::Local(old), ty)),
            Box::new(at(ExprKind::Int(0), ty)),
        );
        test = at(
            ExprKind::And(Box::new(test), Box::new(at(not_negative, Type::Bool))),
            Type::Bool,
        );
        fast.push(Stmt::Let(new, at(ExprKind::Local(old), ty)));
        let assign = ExprKind::Assign(old, Box::new(at(ExprKind::Local(new), ty)));
        back.push(Stmt::Expr(at(assign, Type::Unit)));
    }
    fast.push(copy);
    fast.extend(back);
    let otherwise = if then_plain { plain(written) } else { written };
    choice(test, block(fast), block(vec![otherwise]))
}

/// `0 <= x && x <= SMALL`.
fn in_range(x: Local, line: u32) -> Expr {
    let at = |kind| Expr {
        kind,
        ty: Type::I64,
        line,
    };
    let compare = |op, c| Expr {
        kind: ExprKind::Binary(
            op,
            Box::new(at(ExprKind::Local(x))),
            Box::new(at(ExprKind::Int(c))),
        ),
        ty: Type::Bool,
        line,
    };
    Expr {
        kind: ExprKind::And(
            Box::new(compare(BinOp::Ge, 0)),
            Box::new(compare(BinOp::Le, SMALL)),
        ),
        ty: Type::Bool,
        line,
    }
}

/// The two branches of the outermost `if` that `wrap` made for local `x`.
fn branches(body: &Block, x: Local) -> Option<(Block, Block)> {
    let mut found = None;
    walk::any_in_block(body, &mut |e| {
        if let ExprKind::If(cond, fast, written) = &e.kind
            && is_guard(cond, x)
        {
            found = Some((fast.clone(), written.clone()));
            return true;
        }
        false
    });
    found
}

/// Whether `cond` is `wrap`'s test on `x`: `in_range`, then perhaps
/// `&& a >= 0` for accumulators.
fn is_guard(cond: &Expr, x: Local) -> bool {
    match &cond.kind {
        ExprKind::And(a, _) if is_guard(a, x) => true,
        _ => is_in_range(cond, x),
    }
}

fn is_in_range(cond: &Expr, x: Local) -> bool {
    let bound = |e: &Expr, op: BinOp, c: i128| {
        matches!(&e.kind, ExprKind::Binary(o, a, b)
            if *o == op
                && matches!(a.kind, ExprKind::Local(l) if l == x)
                && matches!(b.kind, ExprKind::Int(n) if n == c))
    };
    matches!(&cond.kind, ExprKind::And(a, b)
        if bound(a, BinOp::Ge, 0) && bound(b, BinOp::Le, SMALL))
}

/// What `b` costs that the ranges could take away: its checks, and its
/// signed divisions and remainders (a `%` proven to have no sign is
/// shorter).
fn cost(b: &Block) -> usize {
    let mut n = checks(b);
    walk::any_in_block(b, &mut |e| {
        if matches!(e.kind, ExprKind::Binary(BinOp::Div | BinOp::Rem, ..)) && e.ty.is_int() {
            n += 1;
        }
        false
    });
    n
}

/// How many checks `b` makes: arithmetic that may overflow, and
/// conversions that may not fit.
fn checks(b: &Block) -> usize {
    let mut n = 0;
    walk::any_in_block(b, &mut |e| {
        let arithmetic = matches!(
            e.kind,
            ExprKind::Binary(BinOp::Add | BinOp::Sub | BinOp::Mul, ..)
        ) && e.ty.is_int();
        if arithmetic || matches!(e.kind, ExprKind::Convert(_, true)) {
            n += 1;
        }
        false
    });
    n
}

/// A copy of `f` to try versioning on.
fn copy(f: &Function) -> Function {
    Function {
        name: f.name.clone(),
        params: f.params.clone(),
        ret: f.ret,
        locals: f.locals.clone(),
        body: f.body.clone(),
        line: f.line,
        fails: f.fails,
        external: f.external.clone(),
    }
}

#[cfg(test)]
mod tests;
