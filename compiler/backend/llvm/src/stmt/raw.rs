//! Which loop bodies allow raw stores and hoisted pushes
//! (`for_range` in `stmt.rs`): the only-use analyses and the
//! straight-line rule.

use super::*;
use kavach_kir::walk::{any_in_block, sets_local};

impl FnEmit<'_> {
    /// The lists the body writes as `list[var] = value` (plain number
    /// items) and uses in no other way: once the loop is proven in range
    /// and the list unshared, those stores can't panic, can't copy, and
    /// change neither the buffer nor the length, so the fast copy writes
    /// straight through the buffer.
    pub(super) fn raw_set_targets(&self, var: Local, body: &Block) -> Vec<Local> {
        if kavach_kir::walk::sets_local(body, var) {
            return Vec::new();
        }
        let mut ours: Vec<(Local, usize)> = Vec::new();
        for s in &body.stmts {
            let Stmt::Expr(e) = s else { continue };
            let ExprKind::Assign(l, value) = &e.kind else {
                continue;
            };
            let ExprKind::Rt(kavach_kir::RtOp::ListSet, args) = &value.kind else {
                continue;
            };
            let receiver =
                matches!(args[0].kind, ExprKind::Local(x) | ExprKind::Move(x) if x == *l);
            let by_var = matches!(args[1].kind, ExprKind::Local(x) if x == var);
            let own = |e: &Expr| {
                let mut found = false;
                kavach_kir::walk::any_in_block(
                    &Block {
                        stmts: Vec::new(),
                        value: Some(Box::new(e.clone())),
                    },
                    &mut |x| {
                        found |= matches!(
                            x.kind,
                            ExprKind::Local(y) | ExprKind::Move(y) if y == *l
                        );
                        false
                    },
                );
                found
            };
            if receiver && by_var && self.plain_items(value.ty) && !own(&args[2]) {
                match ours.iter_mut().find(|(x, _)| x == l) {
                    Some((_, n)) => *n += 1,
                    None => ours.push((*l, 1)),
                }
            }
        }
        // Any other mention of the list, anywhere in the body, rules it
        // out: another read, another assignment, a copy, a call taking it.
        ours.retain(|&(l, n)| {
            let (mut reads, mut assigns) = (0, 0);
            kavach_kir::walk::any_in_block(body, &mut |e| {
                match e.kind {
                    ExprKind::Local(x) | ExprKind::Move(x) if x == l => reads += 1,
                    ExprKind::Assign(x, _) if x == l => assigns += 1,
                    _ => {}
                }
                false
            });
            reads == n && assigns == n
        });
        ours.into_iter().map(|(l, _)| l).collect()
    }

    /// Lists the body grows by exactly one `push` per round, of plain
    /// items, touched nowhere else: the push can store straight into
    /// room reserved before the loop.
    pub(super) fn raw_push_targets(&self, var: Local, body: &Block) -> Vec<Local> {
        if sets_local(body, var) {
            return Vec::new();
        }
        // Every round must push exactly once, so the body must be
        // straight-line code: no branch may skip the push or leave the
        // round early.
        if !straight_line(body) {
            return Vec::new();
        }
        let mut ours: Vec<Local> = Vec::new();
        for s in &body.stmts {
            let Stmt::Expr(e) = s else { continue };
            let ExprKind::Assign(l, value) = &e.kind else {
                continue;
            };
            let ExprKind::Rt(kavach_kir::RtOp::ListPush, args) = &value.kind else {
                continue;
            };
            let receiver =
                matches!(args[0].kind, ExprKind::Local(x) | ExprKind::Move(x) if x == *l);
            let own = {
                let mut found = false;
                any_in_block(
                    &Block {
                        stmts: Vec::new(),
                        value: Some(Box::new(args[1].clone())),
                    },
                    &mut |x| {
                        found |= matches!(
                            x.kind,
                            ExprKind::Local(y) | ExprKind::Move(y) if y == *l
                        );
                        false
                    },
                );
                found
            };
            if receiver && self.plain_items(value.ty) && !own && !ours.contains(l) {
                ours.push(*l);
            }
        }
        // Any other mention of the list, anywhere in the body, rules it
        // out; so does a second push to it.
        ours.retain(|&l| {
            let (mut reads, mut assigns) = (0, 0);
            any_in_block(body, &mut |e| {
                match e.kind {
                    ExprKind::Local(x) | ExprKind::Move(x) if x == l => reads += 1,
                    ExprKind::Assign(x, _) if x == l => assigns += 1,
                    _ => {}
                }
                false
            });
            reads == 1 && assigns == 1
        });
        ours
    }
}

/// Whether `b` is straight-line code: `let`s, expressions and checks
/// only, with no branch anywhere in them (a panic still ends the whole
/// program, so it stays allowed).
fn straight_line(b: &Block) -> bool {
    let plain_stmts = b
        .stmts
        .iter()
        .all(|s| matches!(s, Stmt::Let(..) | Stmt::Expr(_) | Stmt::Check { .. }));
    plain_stmts
        && b.value.is_none()
        && !any_in_block(b, &mut |e| {
            matches!(
                e.kind,
                ExprKind::If(..)
                    | ExprKind::Block(_)
                    | ExprKind::Fallback(..)
                    | ExprKind::Fail(_)
                    | ExprKind::And(..)
                    | ExprKind::Or(..)
            )
        })
}
