//! Building the jammed loop (see `jam.rs`):
//!
//! ```text
//! j = start; end' = end
//! ok = k_start >= 0 and k_end <= len(l) for each list read by `k`
//! shortest = the shortest row l[k] of each list read as l[k][j]
//! while ok and j >= 0 and j < end' and j < end' - 7 and j < len(r) - 7 and j < shortest - 7 {
//!     eight copies of the body, their inner loops fused into one
//!     j = j + 8
//! }
//! for j in j..end' { the original body }
//! ```
//!
//! Each condition after `j < end'` is only reached when the ones before it
//! hold, so none of them can overflow.

use super::{FACTOR, Plan};
use crate::ir::*;
use crate::walk::{exprs_mut, locals_mut};

/// Makes KIR expressions at one line.
struct At(u32);

impl At {
    fn e(&self, kind: ExprKind, ty: Type) -> Expr {
        Expr {
            kind,
            ty,
            line: self.0,
        }
    }
    fn local(&self, l: Local, ty: Type) -> Expr {
        self.e(ExprKind::Local(l), ty)
    }
    fn int(&self, n: i64) -> Expr {
        self.e(ExprKind::Int(n as i128), Type::I64)
    }
    fn bin(&self, op: BinOp, a: Expr, b: Expr, ty: Type) -> Expr {
        self.e(ExprKind::Binary(op, Box::new(a), Box::new(b)), ty)
    }
    fn and(&self, a: Expr, b: Expr) -> Expr {
        self.e(ExprKind::And(Box::new(a), Box::new(b)), Type::Bool)
    }
    fn len(&self, list: Expr) -> Expr {
        self.e(ExprKind::Rt(RtOp::ListLen, vec![list]), Type::I64)
    }
    fn when(&self, cond: Expr, then: Vec<Stmt>) -> Stmt {
        Stmt::Expr(self.e(
            ExprKind::If(Box::new(cond), block(then), Block::default()),
            Type::Unit,
        ))
    }
}

fn fresh(locals: &mut Vec<Type>, t: Type) -> Local {
    locals.push(t);
    (locals.len() - 1) as Local
}

pub(super) fn jammed(
    j: Local,
    start: Expr,
    end: Expr,
    body: Block,
    p: Plan,
    locals: &mut Vec<Type>,
) -> Vec<Stmt> {
    let at = At(start.line);
    let Stmt::ForRange {
        var: k,
        start: ks,
        end: ke,
        body: kb,
        ..
    } = &body.stmts[p.inner]
    else {
        unreachable!("the inner loop")
    };
    let (k, ks, ke, kb) = (*k, ks.clone(), ke.clone(), kb.clone());
    let list = |l: Local, locals: &[Type]| at.local(l, locals[l as usize]);
    let (end_local, ok) = (fresh(locals, Type::I64), fresh(locals, Type::Bool));
    let mut stmts = vec![Stmt::Let(j, start), Stmt::Let(end_local, end)];

    // `ok`: every `l[k]` is in range for the whole inner loop.
    let mut ok_value = at.bin(BinOp::Ge, ks.clone(), at.int(0), Type::Bool);
    for &l in &p.by_k {
        let fits = at.bin(BinOp::Le, ke.clone(), at.len(list(l, locals)), Type::Bool);
        ok_value = at.and(ok_value, fits);
    }
    stmts.push(Stmt::Let(ok, ok_value));

    // The shortest row of each list read as `l[k][j]` (when `ok`).
    let mut shortest = Vec::new();
    for &(l, row_ty) in &p.rows {
        let (m, k2) = (fresh(locals, Type::I64), fresh(locals, Type::I64));
        stmts.push(Stmt::Let(m, at.int(i64::MAX)));
        let index = ExprKind::Index(Box::new(list(l, locals)), Box::new(at.local(k2, Type::I64)));
        let row_len = at.len(at.e(index, row_ty));
        let shorter = at.bin(
            BinOp::Lt,
            row_len.clone(),
            at.local(m, Type::I64),
            Type::Bool,
        );
        let set = Stmt::Expr(at.e(ExprKind::Assign(m, Box::new(row_len)), Type::Unit));
        let scan = Stmt::ForRange {
            var: k2,
            start: ks.clone(),
            end: ke.clone(),
            inclusive: false,
            body: block(vec![at.when(shorter, vec![set])]),
        };
        stmts.push(at.when(at.local(ok, Type::Bool), vec![scan]));
        shortest.push(m);
    }

    // The loop over groups of eight.
    let jl = || at.local(j, Type::I64);
    let below = |limit: Expr| {
        let less = at.bin(BinOp::Sub, limit, at.int(FACTOR - 1), Type::I64);
        at.bin(BinOp::Lt, jl(), less, Type::Bool)
    };
    let mut cond = at.bin(BinOp::Lt, jl(), at.local(end_local, Type::I64), Type::Bool);
    cond = at.and(cond, below(at.local(end_local, Type::I64)));
    for &r in &p.by_j {
        cond = at.and(cond, below(at.len(list(r, locals))));
    }
    for &m in &shortest {
        cond = at.and(cond, below(at.local(m, Type::I64)));
    }
    let not_negative = at.bin(BinOp::Ge, jl(), at.int(0), Type::Bool);
    let cond = at.and(at.local(ok, Type::Bool), at.and(not_negative, cond));

    let (mut group, mut inner, mut after) = (Vec::new(), Vec::new(), Vec::new());
    let before = block(body.stmts[..p.inner].to_vec());
    let rest = block(body.stmts[p.inner + 1..].to_vec());
    for t in 0..FACTOR {
        // Copy `t`: its own accumulators, and `j + t` for `j`.
        let renamed: Vec<(Local, Local)> = p
            .accs
            .iter()
            .map(|c| {
                let ty = locals[*c as usize];
                (*c, fresh(locals, ty))
            })
            .collect();
        // In the inner loop, the checks before cover every index.
        let copy = |b: &Block, in_range: bool| {
            let mut b = b.clone();
            locals_mut(&mut b, &mut |l| {
                if let Some((_, to)) = renamed.iter().find(|(from, _)| from == l) {
                    *l = *to;
                }
            });
            exprs_mut(&mut b, &mut |e| {
                e.kind = match std::mem::replace(&mut e.kind, ExprKind::None) {
                    // `j + t` can't overflow: `j < end' - 7`.
                    ExprKind::Local(l) if l == j && t > 0 => {
                        ExprKind::Fast(FastOp::Add, Box::new(jl()), Box::new(at.int(t)))
                    }
                    ExprKind::Index(list, i) if in_range => ExprKind::InRange(list, i),
                    kind => kind,
                };
            });
            b.stmts
        };
        group.extend(copy(&before, false));
        inner.extend(copy(&kb, true));
        after.extend(copy(&rest, false));
    }
    group.push(Stmt::ForRange {
        var: k,
        start: ks,
        end: ke,
        inclusive: false,
        body: block(inner),
    });
    group.extend(after);
    let next = at.bin(BinOp::Add, jl(), at.int(FACTOR), Type::I64);
    group.push(Stmt::Expr(
        at.e(ExprKind::Assign(j, Box::new(next)), Type::Unit),
    ));
    stmts.push(Stmt::While {
        cond,
        body: block(group),
    });

    // The rest, one `j` at a time.
    stmts.push(Stmt::ForRange {
        var: j,
        start: jl(),
        end: at.local(end_local, Type::I64),
        inclusive: false,
        body,
    });
    stmts
}

fn block(stmts: Vec<Stmt>) -> Block {
    Block { stmts, value: None }
}
