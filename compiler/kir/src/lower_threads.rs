//! `on threads(n) for …` (spec 4.9) in native code: each round a task of
//! its own, as `spawn thread` makes it, at most `n` at once; before round
//! `k` starts, round `k - n` is waited for, as `parallel_map` does (9.3).
//! A round starts with its sums at 0 and gives back its slot values and
//! sums, which are applied when it is waited for, in round order. The
//! interpreter does the same (`compiler/interp/src/on_threads.rs`).

use crate::ir::{self, Block, ExprKind as K, Function, Local, Shape, Stmt, Type};
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
        // At most `n` rounds at once: before this one, the oldest waited
        // for and applied.
        if let Some(n) = count {
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
            let full = at(
                K::Binary(
                    ir::BinOp::Ge,
                    Box::new(running),
                    Box::new(self.local_expr(n, line)),
                ),
                Type::Bool,
                line,
            );
            each.push(Stmt::Expr(at(
                K::If(Box::new(full), apply.clone(), Block::default()),
                Type::Unit,
                line,
            )));
        }
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

pub(crate) struct Round {
    pub(crate) task_ty: Type,
    pub(crate) value_ty: Type,
    thunk: ir::FuncId,
    env_ty: Type,
    captures: Vec<ir::Expr>,
}

impl<'a> Lowerer<'a> {
    /// The task a round runs: the body, then its slot values and sums.
    fn round_task(
        &mut self,
        body: &'a ast::Block,
        writes: &'a RoundWrites,
        line: u32,
    ) -> Lowered<Round> {
        let span = ast::Expr {
            kind: ExprKind::None,
            span: body.span,
        };
        let (captured, outer, types) = self.captures_skipping(&span, &[]);
        let captures: Vec<ir::Expr> = outer.iter().map(|l| self.local_expr(*l, line)).collect();
        let env_ty = self.shapes_intern(Shape::Closure(types.clone()));
        // The value's type: the targets' types, read in the outer function.
        let reads: Vec<&ast::Expr> = (writes.slots.iter())
            .chain(writes.sums.iter().map(|s| &s.target))
            .collect();
        let mut read_types = Vec::new();
        for r in &reads {
            match self.c.expr_types.get(&(r.span.start, r.span.end)) {
                Some(t) => read_types.push(self.kir_type(&t.clone(), r.span)?),
                None => return Err(self.unsupported("this `on threads` loop", r.span)),
            }
        }
        let value_ty = self.shapes_intern(Shape::Tuple(read_types));
        let saved = self.save_function(value_ty);
        let lowered = (|| -> Lowered<Function> {
            let (env, mut b) = self.unpack_env(env_ty, &captured, &types, line);
            // In a loop of one round, so `continue` ends the round.
            let once = self.new_local(Type::I64);
            let mut inner = Block::default();
            self.block_into(body, false, &mut inner)?;
            b.stmts.push(Stmt::ForRange {
                var: once,
                start: at(K::Int(0), Type::I64, line),
                end: at(K::Int(1), Type::I64, line),
                inclusive: false,
                body: inner,
            });
            let mut values = Vec::new();
            for r in &reads {
                values.push(self.expr(r)?);
            }
            b.value = Some(Box::new(at(K::Tuple(values), value_ty, line)));
            Ok(Function {
                name: "on threads round".into(),
                params: vec![env],
                ret: value_ty,
                locals: self.locals.clone(),
                body: b,
                line,
                fails: true,
                external: None,
                variadic: false,
            })
        })();
        self.restore_function(saved);
        let body_fn = self.add_function(lowered?);
        let saved = self.save_function(Type::Unit);
        let error_ty = self.error_type();
        let thunk = self.task_thunk(body_fn, env_ty, value_ty, error_ty, line);
        self.restore_function(saved);
        let thunk = self.add_function(thunk);
        let task_ty = self.shapes_intern(Shape::Task(value_ty));
        Ok(Round {
            task_ty,
            value_ty,
            thunk,
            env_ty,
            captures,
        })
    }

    fn spawn_round(&mut self, round: &Round, line: u32) -> ir::Expr {
        let func_ty = self.shapes_intern(Shape::Func(Vec::new(), Type::Unit, None));
        let closure = K::Closure {
            func: round.thunk,
            env: round.env_ty,
            captures: round.captures.clone(),
        };
        // A thread (kind 1), as `spawn thread` and `parallel_map` make.
        let thread = at(K::Int(1), Type::I64, line);
        at(
            K::Rt(RtOp::TaskSpawn, vec![at(closure, func_ty, line), thread]),
            round.task_ty,
            line,
        )
    }

    /// Waits for round `applied` and applies it: its slot values into the
    /// lists at its slot, its sums added in. A round's panic is a panic
    /// here, as `parallel_map`'s is (9.3).
    fn apply_round(
        &mut self,
        writes: &'a RoundWrites,
        slot_id: Option<&'a ast::Ident>,
        (tasks, _, slots, _, applied): (Local, Type, Local, Type, Local),
        (task_ty, value_ty, slot_now): (Type, Type, Local),
        line: u32,
    ) -> Lowered<Block> {
        let error_ty = self.error_type();
        let (value, error) = (self.new_local(value_ty), self.new_local(error_ty));
        let nth = |l: Local, item: Type, this: &Self| {
            at(
                K::Index(
                    Box::new(this.local_expr(l, line)),
                    Box::new(this.local_expr(applied, line)),
                ),
                item,
                line,
            )
        };
        let mut stmts = vec![Stmt::LetResult {
            value,
            error,
            call: at(
                K::Rt(RtOp::TaskWait, vec![nth(tasks, task_ty, self)]),
                value_ty,
                line,
            ),
        }];
        let text = self.str_type();
        let message = at(
            K::Format(vec![
                at(K::Str("a round of `on threads` ".into()), text, line),
                self.local_expr(error, line),
            ]),
            text,
            line,
        );
        stmts.push(Stmt::Expr(at(
            K::If(
                Box::new(self.failed(error, line)),
                Block {
                    stmts: vec![Stmt::Expr(at(
                        K::Rt(RtOp::PanicText, vec![message]),
                        Type::Unit,
                        line,
                    ))],
                    value: None,
                },
                Block::default(),
            ),
            Type::Unit,
            line,
        )));
        let Some(Shape::Tuple(items)) = self.shapes.shape(value_ty).cloned() else {
            return Err(self.unsupported("this `on threads` loop", writes_span(writes)));
        };
        // The slot's name stands for the applied round's slot meanwhile.
        if let Some(id) = slot_id {
            stmts.push(Stmt::Let(slot_now, nth(slots, Type::I64, self)));
            let restore = self.bound(id)?;
            let slot_ty = self.locals[restore as usize];
            let typed = self.new_local(slot_ty);
            let v = self.coerce(self.local_expr(slot_now, line), slot_ty)?;
            stmts.push(Stmt::Let(typed, v));
            self.bind(id, typed);
            for (j, target) in writes.slots.iter().enumerate() {
                let v = self.tuple_item(value, value_ty, j, items[j], line);
                stmts.push(self.assign_path(target, v)?);
            }
            self.bind(id, restore);
        }
        for (n, s) in writes.sums.iter().enumerate() {
            let j = writes.slots.len() + n;
            let part = self.tuple_item(value, value_ty, j, items[j], line);
            let total = self.expr(&s.target)?;
            let ty = total.ty;
            let mut sum = at(
                K::Binary(ir::BinOp::Add, Box::new(total), Box::new(part)),
                ty,
                line,
            );
            if let Some(m) = s.modulo {
                let m = self.coerce(at(K::Int(i128::from(m)), Type::I64, line), ty)?;
                sum = at(
                    K::Binary(ir::BinOp::Rem, Box::new(sum), Box::new(m)),
                    ty,
                    line,
                );
            }
            stmts.push(self.assign_path(&s.target, sum)?);
        }
        let next = at(
            K::Binary(
                ir::BinOp::Add,
                Box::new(self.local_expr(applied, line)),
                Box::new(at(K::Int(1), Type::I64, line)),
            ),
            Type::I64,
            line,
        );
        stmts.push(Stmt::Expr(at(
            K::Assign(applied, Box::new(next)),
            Type::Unit,
            line,
        )));
        Ok(Block { stmts, value: None })
    }
}

fn writes_span(w: &RoundWrites) -> ast::Span {
    (w.slots.first())
        .or(w.sums.first().map(|s| &s.target))
        .map_or(ast::Span::new(0, 0), |e| e.span)
}

impl Lowerer<'_> {
    /// The local a pattern's name was bound to (`bind`).
    fn bound(&self, id: &ast::Ident) -> Lowered<Local> {
        (self.decl.get(&id.span.start))
            .and_then(|d| self.local_ids.get(d).copied())
            .ok_or_else(|| self.unsupported(format!("the name `{}`", id.name), id.span))
    }
}
