//! The hoisting half of fusing: a chain written as one expression
//! is pulled apart into the statement sequence the pairwise fusing
//! in `fuse.rs` matches, one stage per `let`.

use super::{Cx, defined, first_evaluated};
use crate::ir::*;
use crate::walk::any_in_block;

impl Cx<'_> {
    /// When `s` is `let l = { let src = <block>; … }`, takes the inner
    /// block out as `let new = <block>` and reads `new` in its place:
    /// one chain stage per statement.
    pub(super) fn hoist(&mut self, s: &mut Stmt) -> Option<Vec<Stmt>> {
        let e = first_evaluated(s)?;
        if let ExprKind::Block(inner) = &mut e.kind {
            // A block whose value is a block runs its statements, then
            // the value block: the statements move out in front, and
            // the value block stays whole, the shape `fuse` matches.
            while matches!(
                inner.value.as_deref().map(|v| &v.kind),
                Some(ExprKind::Block(_))
            ) {
                let ExprKind::Block(vb) = inner.value.take().unwrap().kind else {
                    unreachable!()
                };
                let pre = std::mem::replace(&mut inner.stmts, vb.stmts);
                inner.value = vb.value;
                if !pre.is_empty() {
                    return Some(pre);
                }
            }
            // A stage nested in the next stage's first `let`.
            let Some(Stmt::Let(_, from)) = inner.stmts.first_mut() else {
                return None;
            };
            if matches!(from.kind, ExprKind::Block(_)) && from.ty.is_ref() {
                return Some(vec![self.lift(from)]);
            }
            return None;
        }
        // A chain used as a value inside the statement, as in
        // `total = total + row.map(f).sum(g)`.
        let b = spine_block(e)?;
        let lifted = self.lift(b);
        Some(vec![lifted])
    }

    /// `from` taken out as its own `let`, a read of the new local in
    /// its place.
    fn lift(&mut self, from: &mut Expr) -> Stmt {
        let new = self.locals.len() as Local;
        self.locals.push(from.ty);
        self.reads.push(1);
        let read = Expr {
            kind: ExprKind::Local(new),
            ty: from.ty,
            line: from.line,
        };
        Stmt::Let(new, std::mem::replace(from, read))
    }
}

/// The first loop-bearing block `e` evaluates, before reading anything
/// a block could change: only blocks that assign nothing but their own
/// locals are returned, so running one early changes nothing.
fn spine_block(e: &mut Expr) -> Option<&mut Expr> {
    let atom = |x: &Expr| {
        !x.ty.is_ref()
            && matches!(
                x.kind,
                ExprKind::Local(_)
                    | ExprKind::Int(_)
                    | ExprKind::Float(_)
                    | ExprKind::Bool(_)
                    | ExprKind::Char(_)
            )
    };
    let lifts = matches!(&e.kind, ExprKind::Block(b)
        if b.value.is_some()
            && (b.stmts.iter().any(|s| matches!(s, Stmt::ForEach { .. }))
                || matches!(b.value.as_deref().map(|v| &v.kind), Some(ExprKind::Block(_))))
            && self_contained(b));
    if lifts {
        return Some(e);
    }
    match &mut e.kind {
        ExprKind::Assign(_, v)
        | ExprKind::Unary(_, v)
        | ExprKind::Convert(v, _)
        | ExprKind::Some(v) => spine_block(v),
        ExprKind::Binary(_, a, b) => {
            if atom(a) {
                spine_block(b)
            } else {
                spine_block(a)
            }
        }
        _ => None,
    }
}

/// Whether `b` assigns only locals it sets up itself.
fn self_contained(b: &Block) -> bool {
    let mut own = Vec::new();
    defined(b, &mut own);
    !any_in_block(
        b,
        &mut |e| matches!(e.kind, ExprKind::Assign(l, _) if !own.contains(&l)),
    )
}

/// A rough picture of `b` for `KAVACH_FUSE_SKELETON`, one line a
/// statement.
pub(super) fn skeleton(b: &Block, depth: usize) {
    let pad = "  ".repeat(depth);
    for s in &b.stmts {
        match s {
            Stmt::Let(l, e) => {
                eprintln!("{pad}Let({l}, {})", name(e));
                if let ExprKind::Block(x) = &e.kind {
                    skeleton(x, depth + 1);
                }
            }
            Stmt::Expr(e) => {
                eprintln!("{pad}Expr({})", name(e));
                if let ExprKind::Block(x) = &e.kind {
                    skeleton(x, depth + 1);
                }
            }
            Stmt::ForEach {
                var, list, body, ..
            } => {
                eprintln!("{pad}ForEach({var} in {list})");
                skeleton(body, depth + 1);
            }
            Stmt::ForRange { var, body, .. } => {
                eprintln!("{pad}ForRange({var})");
                skeleton(body, depth + 1);
            }
            Stmt::While { body, .. } => {
                eprintln!("{pad}While");
                skeleton(body, depth + 1);
            }
            _ => eprintln!("{pad}(other)"),
        }
    }
}

fn name(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Block(x) => format!(
            "Block[{} stmts, value {}]",
            x.stmts.len(),
            x.value.as_ref().map_or("none".into(), |v| name(v)),
        ),
        ExprKind::Assign(l, v) => format!("Assign({l}, {})", name(v)),
        ExprKind::Binary(op, a, c) => format!("Binary({op:?}, {}, {})", name(a), name(c)),
        ExprKind::Fallback(..) => "Fallback".into(),
        ExprKind::Local(l) => format!("Local({l})"),
        ExprKind::Rt(op, _) => format!("Rt({op:?})"),
        ExprKind::If(..) => "If".into(),
        _ => "·".into(),
    }
}
