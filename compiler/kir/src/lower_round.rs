// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The task one round of an `on threads` loop runs, and what the loop
//! does with it when it is waited for (spec 4.9, `lower_threads.rs`).

use crate::ir::{self, Block, ExprKind as K, Function, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::rounds::RoundWrites;
use kavach_ast::{self as ast, ExprKind};

fn at(kind: K, ty: Type, line: u32) -> ir::Expr {
    ir::Expr { kind, ty, line }
}

pub(crate) struct Round {
    pub(crate) task_ty: Type,
    pub(crate) value_ty: Type,
    /// Per slot: the round wrote it into a local of its own, given back
    /// as an optional (`none`: not written), instead of into its copy of
    /// the list, which a write would copy whole.
    pub(crate) written: Vec<bool>,
    thunk: ir::FuncId,
    env_ty: Type,
    captures: Vec<ir::Expr>,
}

impl<'a> Lowerer<'a> {
    /// The task a round runs: the body, then its slot values and sums.
    pub(crate) fn round_task(
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
        let mut captures: Vec<ir::Expr> = outer.iter().map(|l| self.local_expr(*l, line)).collect();
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
        let saved = self.save_function(Type::Unit);
        let mut written = vec![false; writes.slots.len()];
        let mut unread = Vec::new();
        let lowered = (|| -> Lowered<Function> {
            let (env, mut b) = self.unpack_env(env_ty, &captured, &types, line);
            // In a loop of one round, so `continue` ends the round.
            let once = self.new_local(Type::I64);
            let mut inner = Block::default();
            self.block_into(body, false, &mut inner)?;
            let mut slot_locals = Vec::new();
            for (j, target) in writes.slots.iter().enumerate() {
                let Some(local) = self.slot_into_local(target, &mut inner, read_types[j], line)
                else {
                    slot_locals.push(None);
                    continue;
                };
                b.stmts.push(Stmt::Let(
                    local,
                    at(K::None, self.locals[local as usize], line),
                ));
                read_types[j] = self.locals[local as usize];
                written[j] = true;
                slot_locals.push(Some(local));
            }
            b.stmts.push(Stmt::ForRange {
                var: once,
                start: at(K::Int(0), Type::I64, line),
                end: at(K::Int(1), Type::I64, line),
                inclusive: false,
                body: inner,
            });
            let mut values = Vec::new();
            for (j, r) in reads.iter().enumerate() {
                values.push(match slot_locals.get(j).copied().flatten() {
                    Some(l) => self.local_expr(l, line),
                    None => self.expr(r)?,
                });
            }
            let value_ty = self.shapes_intern(Shape::Tuple(read_types.clone()));
            b.value = Some(Box::new(at(K::Tuple(values), value_ty, line)));
            // A list the round no longer reads (its slot is written into
            // a local) is not given to it: a round on a thread of its own
            // gets a copy of all it is given.
            for (i, stmt) in b.stmts.iter().enumerate().take(types.len()) {
                let Stmt::Let(local, _) = stmt else { continue };
                let read = |e: &ir::Expr| matches!(e.kind, K::Local(l) if l == *local);
                let rest = Block {
                    stmts: b.stmts[types.len()..].to_vec(),
                    value: b.value.clone(),
                };
                if !crate::walk::any_in_block(&rest, &mut { read }) {
                    unread.push(i);
                }
            }
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
        let value_ty = self.shapes_intern(Shape::Tuple(read_types));
        for i in unread {
            let ty = captures[i].ty;
            if let Some(Shape::List(_)) = self.shapes.shape(ty) {
                captures[i] = at(K::List(Vec::new()), ty, line);
            }
        }
        let saved = self.save_function(Type::Unit);
        let error_ty = self.error_type();
        let thunk = self.task_thunk(body_fn, env_ty, value_ty, error_ty, line);
        self.restore_function(saved);
        let thunk = self.add_function(thunk);
        let task_ty = self.shapes_intern(Shape::Task(value_ty));
        Ok(Round {
            task_ty,
            value_ty,
            written,
            thunk,
            env_ty,
            captures,
        })
    }

    pub(crate) fn spawn_round(&mut self, round: &Round, line: u32) -> ir::Expr {
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
    pub(crate) fn apply_round(
        &mut self,
        writes: &'a RoundWrites,
        slot_id: Option<&'a ast::Ident>,
        (tasks, _, slots, _, applied): (Local, Type, Local, Type, Local),
        (task_ty, value_ty, slot_now): (Type, Type, Local),
        written: &[bool],
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
        let minus_one = at(K::Int(-1), Type::I64, line);
        stmts.push(Stmt::Expr(at(
            K::Rt(RtOp::RunRoundsAdd, vec![minus_one]),
            Type::Unit,
            line,
        )));
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
                if !written.get(j).copied().unwrap_or(false) {
                    stmts.push(self.assign_path(target, v)?);
                    continue;
                }
                // Written into a local: set only when the round set it.
                let Some(Shape::Optional(inner)) = self.shapes.shape(items[j]).cloned() else {
                    stmts.push(self.assign_path(target, v)?);
                    continue;
                };
                let got = self.new_local(items[j]);
                stmts.push(Stmt::Let(got, v));
                let some = at(
                    K::IsSome(Box::new(self.local_expr(got, line))),
                    Type::Bool,
                    line,
                );
                let one = at(K::Unwrap(Box::new(self.local_expr(got, line))), inner, line);
                let set = self.assign_path(target, one)?;
                stmts.push(Stmt::Expr(at(
                    K::If(
                        Box::new(some),
                        Block {
                            stmts: vec![set],
                            value: None,
                        },
                        Block::default(),
                    ),
                    Type::Unit,
                    line,
                )));
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
    pub(crate) fn bound(&self, id: &ast::Ident) -> Lowered<Local> {
        (self.decl.get(&id.span.start))
            .and_then(|d| self.local_ids.get(d).copied())
            .ok_or_else(|| self.unsupported(format!("the name `{}`", id.name), id.span))
    }
}

impl Lowerer<'_> {
    /// `out[i] = v` in a round, with `out` a list from outside and `i`
    /// the round's slot: each such write in `body` set into a new local
    /// `T?` instead, when the round reads `out` nowhere else, so its copy
    /// of the list is never copied whole. The local, or `None`.
    fn slot_into_local(
        &mut self,
        target: &ast::Expr,
        body: &mut Block,
        item_ty: Type,
        line: u32,
    ) -> Option<Local> {
        let ExprKind::Index { base, index } = &target.kind else {
            return None;
        };
        let (ExprKind::Name(list), ExprKind::Name(slot)) = (&base.kind, &index.kind) else {
            return None;
        };
        let local_of = |this: &Self, id: &ast::Ident| {
            let def = this.c.resolution.uses.get(&id.span.start)?;
            this.local_ids.get(def).copied()
        };
        let (list, slot) = (local_of(self, list)?, local_of(self, slot)?);
        let opt = self.optional_type(item_ty);
        let local = self.new_local(opt);
        let mut tried = body.clone();
        let mut found = 0;
        crate::walk::exprs_mut(&mut tried, &mut |e| {
            let K::Assign(to, value) = &mut e.kind else {
                return;
            };
            let K::Rt(RtOp::ListSet, args) = &mut value.kind else {
                return;
            };
            let this_list = *to == list && matches!(args[0].kind, K::Local(l) if l == list);
            if !this_list || !matches!(args[1].kind, K::Local(l) if l == slot) {
                return;
            }
            let v = args.pop().expect("a value");
            **value = ir::Expr {
                kind: K::Some(Box::new(v)),
                ty: opt,
                line,
            };
            *to = local;
            found += 1;
        });
        let still_read =
            crate::walk::any_in_block(&tried, &mut |e| matches!(e.kind, K::Local(l) if l == list));
        if found == 0 || still_read {
            return None;
        }
        *body = tried;
        Some(local)
    }
}
