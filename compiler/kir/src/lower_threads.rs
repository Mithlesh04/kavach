//! `on threads(n) for …` (spec 4.9) in native code: each round a task of
//! its own, as `spawn thread` makes it, at most `n` at once; before round
//! `k` starts, round `k - n` is waited for, as `parallel_map` does (9.3).
//! A round starts with its sums at 0 and gives back its slot values and
//! sums, which are applied when it is waited for, in round order. The
//! interpreter does the same (`compiler/interp/src/on_threads.rs`).

use crate::ir::{self, Block, ExprKind as K, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::rounds::{RoundWrites, round_writes};
use kavach_ast::{self as ast, ExprKind, PatternKind};

fn at(kind: K, ty: Type, line: u32) -> ir::Expr {
    ir::Expr { kind, ty, line }
}

/// What the loop walks: a range, or a list with or without its index.
enum Walk<'a> {
    Range(&'a ast::Expr, &'a ast::Expr, bool),
    List(&'a ast::Expr),
}

impl<'a> Lowerer<'a> {
    /// The loop as tasks, into `out`; `false` when its rounds run one
    /// after another instead: the body can leave early, or the loop's
    /// pattern or what it walks is of another kind. `count` holds `n`.
    pub(crate) fn threads_loop(
        &mut self,
        count: Option<Local>,
        pattern: &'a ast::Pattern,
        iter: &'a ast::Expr,
        body: &'a ast::Block,
        out: &mut Vec<Stmt>,
    ) -> Lowered<bool> {
        let name = |p: &'a ast::Pattern| match &p.kind {
            PatternKind::Binding(id) => Some(Some(id)),
            PatternKind::Wildcard => Some(None),
            _ => None,
        };
        // The walk, the index's name and the item's name.
        let (walk, index, item) = match (&pattern.kind, &iter.kind) {
            (
                _,
                ExprKind::Binary(
                    op @ (ast::BinaryOp::Range | ast::BinaryOp::RangeInclusive),
                    lo,
                    hi,
                ),
            ) => {
                let Some(i) = name(pattern) else {
                    return Ok(false);
                };
                (
                    Walk::Range(lo, hi, *op == ast::BinaryOp::RangeInclusive),
                    i,
                    None,
                )
            }
            (PatternKind::Tuple(ps), ExprKind::Call { callee, args, .. })
                if ps.len() == 2 && args.is_empty() =>
            {
                let ExprKind::Field { base, name: m, .. } = &callee.kind else {
                    return Ok(false);
                };
                let (Some(i), Some(x)) = (name(&ps[0]), name(&ps[1])) else {
                    return Ok(false);
                };
                if m.name != "enumerate" {
                    return Ok(false);
                }
                (Walk::List(base), i, x)
            }
            _ => {
                let list = matches!(
                    self.c.expr_types.get(&(iter.span.start, iter.span.end)),
                    Some(kavach_checker::Ty::List(_))
                );
                let Some(x) = name(pattern).filter(|_| list) else {
                    return Ok(false);
                };
                (Walk::List(iter), None, x)
            }
        };
        // The round's slot: the index, or the loop's name.
        let slot_id = index.or(item);
        // Kept for the build: the targets are lowered as the source's own.
        let writes: &'a RoundWrites = Box::leak(Box::new(round_writes(
            body,
            slot_id.map(|id| id.name.as_str()),
        )));
        if writes.leaves {
            return Ok(false);
        }
        let line = self.line(iter.span);
        let int = |n: i128| at(K::Int(n), Type::I64, line);
        // What is walked, and the round's number `k`.
        let (start, end, list) = match walk {
            Walk::Range(lo, hi, inclusive) => {
                let (lo, hi) = (self.expr(lo)?, self.expr(hi)?);
                let hi = match inclusive {
                    true => at(
                        K::Binary(ir::BinOp::Add, Box::new(hi), Box::new(int(1))),
                        Type::I64,
                        line,
                    ),
                    false => hi,
                };
                (lo, hi, None)
            }
            Walk::List(l) => {
                let v = self.expr(l)?;
                let (ty, held) = (v.ty, self.new_local(v.ty));
                out.push(Stmt::Let(held, v));
                let len = at(
                    K::Rt(RtOp::ListLen, vec![self.local_expr(held, line)]),
                    Type::I64,
                    line,
                );
                (int(0), len, Some((held, ty)))
            }
        };
        let k = self.new_local(start.ty);
        if let Some(i) = index {
            self.bind(i, k);
        }
        let item_local = match (list, item) {
            (Some((held, ty)), Some(x)) => {
                let Some(Shape::List(t)) = self.shapes.shape(ty).cloned() else {
                    return Ok(false);
                };
                let l = self.new_local(t);
                self.bind(x, l);
                Some((l, held, t))
            }
            _ => None,
        };
        let round = self.round_task(body, writes, line)?;
        let (task_ty, value_ty) = (round.task_ty, round.value_ty);
        let tasks_ty = self.list_type(task_ty);
        let slots_ty = self.list_type(Type::I64);
        let (tasks, slots, applied) = (
            self.new_local(tasks_ty),
            self.new_local(slots_ty),
            self.new_local(Type::I64),
        );
        out.push(Stmt::Let(tasks, at(K::List(Vec::new()), tasks_ty, line)));
        out.push(Stmt::Let(slots, at(K::List(Vec::new()), slots_ty, line)));
        out.push(Stmt::Let(applied, int(0)));
        let slot_now = self.new_local(Type::I64);
        let apply = self.apply_round(
            writes,
            slot_id,
            (tasks, tasks_ty, slots, slots_ty, applied),
            (task_ty, value_ty, slot_now),
            &round.written,
            line,
        )?;
        let mut each = Vec::new();
        if let Some((l, held, t)) = item_local {
            let read = at(
                K::Index(
                    Box::new(self.local_expr(held, line)),
                    Box::new(self.local_expr(k, line)),
                ),
                t,
                line,
            );
            each.push(Stmt::Let(l, read));
        }
        // At most `n` rounds at once, and at most `[run] threads` in all
        // the program's loops: before this one, the oldest waited for and
        // applied. A loop with none running starts one anyway (spec 4.9).
        let running = at(
            K::Binary(
                ir::BinOp::Sub,
                Box::new(at(
                    K::Rt(RtOp::ListLen, vec![self.local_expr(tasks, line)]),
                    Type::I64,
                    line,
                )),
                Box::new(self.local_expr(applied, line)),
            ),
            Type::I64,
            line,
        );
        let (held, held_ty) = (self.new_local(Type::I64), Type::I64);
        each.push(Stmt::Let(held, running));
        let all_full = at(
            K::And(
                Box::new(at(
                    K::Binary(
                        ir::BinOp::Gt,
                        Box::new(self.local_expr(held, line)),
                        Box::new(int(0)),
                    ),
                    Type::Bool,
                    line,
                )),
                Box::new(at(K::Rt(RtOp::RunRoundsFull, Vec::new()), Type::Bool, line)),
            ),
            Type::Bool,
            line,
        );
        let full = match count {
            Some(n) => at(
                K::Or(
                    Box::new(at(
                        K::Binary(
                            ir::BinOp::Ge,
                            Box::new(at(K::Local(held), held_ty, line)),
                            Box::new(self.local_expr(n, line)),
                        ),
                        Type::Bool,
                        line,
                    )),
                    Box::new(all_full),
                ),
                Type::Bool,
                line,
            ),
            None => all_full,
        };
        each.push(Stmt::Expr(at(
            K::If(Box::new(full), apply.clone(), Block::default()),
            Type::Unit,
            line,
        )));
        // Each round's sums start at 0 (`round_task`), the loop's after.
        let mut saved = Vec::new();
        for s in &writes.sums {
            let v = self.expr(&s.target)?;
            let keep = self.new_local(v.ty);
            each.push(Stmt::Let(keep, v));
            let zero = self.coerce(int(0), self.locals[keep as usize])?;
            each.push(self.assign_path(&s.target, zero)?);
            saved.push((s, keep));
        }
        let spawned = self.spawn_round(&round, line);
        each.push(push(tasks, tasks_ty, spawned, line));
        each.push(Stmt::Expr(at(
            K::Rt(RtOp::RunRoundsAdd, vec![int(1)]),
            Type::Unit,
            line,
        )));
        for (s, keep) in saved {
            let v = self.local_expr(keep, line);
            each.push(self.assign_path(&s.target, v)?);
        }
        if let Some(id) = slot_id {
            let local = self.bound(id)?;
            let v = self.coerce(self.local_expr(local, line), Type::I64)?;
            each.push(push(slots, slots_ty, v, line));
        }
        out.push(Stmt::ForRange {
            var: k,
            start,
            end,
            inclusive: false,
            body: Block {
                stmts: each,
                value: None,
            },
        });
        // Then every round not applied yet, in order.
        let left = at(
            K::Binary(
                ir::BinOp::Lt,
                Box::new(self.local_expr(applied, line)),
                Box::new(at(
                    K::Rt(RtOp::ListLen, vec![self.local_expr(tasks, line)]),
                    Type::I64,
                    line,
                )),
            ),
            Type::Bool,
            line,
        );
        out.push(Stmt::While {
            cond: left,
            body: apply,
        });
        Ok(true)
    }
}

/// `list = list.push(v)` for local `list` of type `ty`.
fn push(list: Local, ty: Type, v: ir::Expr, line: u32) -> Stmt {
    let pushed = at(
        K::Rt(RtOp::ListPush, vec![at(K::Local(list), ty, line), v]),
        ty,
        line,
    );
    Stmt::Expr(at(K::Assign(list, Box::new(pushed)), Type::Unit, line))
}
