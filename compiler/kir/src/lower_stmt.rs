// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Lowering statements: `let`, assignment (including to fields), loops,
//! and expressions used as statements.

use crate::ir::{self, PrintPart, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, ExprKind, PatternKind, StmtKind};

impl<'a> Lowerer<'a> {
    pub(crate) fn stmt(&mut self, s: &'a ast::Stmt, out: &mut Vec<Stmt>) -> Lowered<()> {
        let line = self.line(s.span);
        match &s.kind {
            StmtKind::Let {
                pattern, ty, value, ..
            } => {
                let declared = match ty {
                    Some(t) => Some(self.declared_type(t, pattern)?),
                    None => None,
                };
                // How many items a list literal shows (spec 1.12).
                let known = match &value.kind {
                    ExprKind::List(xs) => xs.len(),
                    _ => 0,
                };
                let mut value = self.expr(value)?;
                if let Some(want) = declared {
                    value = self.coerce(value, want)?;
                }
                match &pattern.kind {
                    PatternKind::Binding(id) => {
                        let local = self.new_local(declared.unwrap_or(value.ty));
                        self.bind(id, local);
                        out.push(Stmt::Let(local, value));
                    }
                    PatternKind::Wildcard => out.push(Stmt::Expr(value)),
                    _ => {
                        let ty = value.ty;
                        let whole = self.new_local(ty);
                        out.push(Stmt::Let(whole, value));
                        if !self.destructure_parts(pattern, whole, ty, known, out)? {
                            self.destructure(pattern, whole, ty, out)?;
                        }
                    }
                }
            }
            StmtKind::Assign { target, op, value } => {
                if matches!(target.kind, ExprKind::Tuple(_)) {
                    return self.assign_tuple(target, value, out);
                }
                // `ps = ps.map(f)`: each item replaced in place.
                if *op == ast::AssignOp::Set
                    && let Some(s) = self.map_in_place(target, value)?
                {
                    out.push(s);
                    return Ok(());
                }
                // The place's keys once, before the value (spec 3.10). In
                // `m[k] = v`, with one key and a value that is a name or a
                // literal, the key is worked out once and first anyway, so
                // it stays where it is: `m[xs[i]] = v` reads its key in its
                // list (`rt_maps/item_keys.rs`).
                let settled = matches!(
                    value.kind,
                    ExprKind::Name(_)
                        | ExprKind::Int(_)
                        | ExprKind::Float(_)
                        | ExprKind::Bool(_)
                        | ExprKind::Char(_)
                ) && matches!(&target.kind, ExprKind::Index { base, .. }
                    if matches!(base.kind, ExprKind::Name(_)));
                let held = if *op == ast::AssignOp::Set && settled {
                    Vec::new()
                } else {
                    self.hold_keys(target, out)?
                };
                let done = self.assign_stmt(target, *op, value, line, s.span, out);
                self.release(held);
                done?;
            }
            StmtKind::For {
                label,
                place,
                pattern,
                iter,
                body,
            } => {
                // `on cores for …` (spec 4.9): the one kind spread over cores.
                if place.as_ref().is_some_and(|p| !p.threads) {
                    let line = self.line(iter.span);
                    self.on_cores.insert(line);
                }
                // `cores(n)`, `threads(n)`: the count, worked out first and
                // checked; `par.rs` reads it from the check just before the
                // loop.
                let mut count = None;
                if let Some(place) = place
                    && let Some(n) = &place.count
                {
                    let n = self.expr(n)?;
                    let c = self.new_local(Type::I64);
                    out.push(Stmt::Let(c, n));
                    count = Some(c);
                    let at = |kind| ir::Expr {
                        kind,
                        ty: Type::I64,
                        line,
                    };
                    let args = vec![
                        at(ir::ExprKind::Local(c)),
                        at(ir::ExprKind::Int(place.threads.into())),
                    ];
                    out.push(Stmt::Expr(ir::Expr {
                        kind: ir::ExprKind::Rt(RtOp::ParCores, args),
                        ty: Type::Unit,
                        line,
                    }));
                }
                // `on threads`: each round a task (`lower_threads.rs`).
                let threads = place.as_ref().is_some_and(|p| p.threads);
                if !(threads && self.threads_loop(count, pattern, iter, body, out)?) {
                    self.labeled_loop(label, out, |l, out| l.for_loop(pattern, iter, body, out))?
                }
            }
            StmtKind::While { label, cond, body } => self.labeled_loop(label, out, |l, out| {
                let cond = l.expr(cond)?;
                let mut b = ir::Block::default();
                l.block_into(body, false, &mut b)?;
                out.push(Stmt::While { cond, body: b });
                Ok(())
            })?,
            StmtKind::Assert(e) => self.assert_stmt(e, out)?,
            StmtKind::Expr(e) => self.expr_stmt(e, out)?,
        }
        Ok(())
    }

    /// `x = v`, `x.a.b = v`: a field change becomes a copy of the record
    /// with that field replaced, stored back into the variable.
    pub(crate) fn assign_path(&mut self, target: &'a ast::Expr, value: ir::Expr) -> Lowered<Stmt> {
        let line = value.line;
        match &target.kind {
            ExprKind::Name(id) => {
                let local = self.local(id)?;
                let value = self.coerce(value, self.locals[local as usize])?;
                Ok(Stmt::Expr(ir::Expr {
                    kind: ir::ExprKind::Assign(local, Box::new(value)),
                    ty: Type::Unit,
                    line,
                }))
            }
            ExprKind::Field {
                base,
                name,
                optional: false,
            } => {
                let base_value = self.expr(base)?;
                let (index, field_ty) = self.field_index(base_value.ty, &name.name, name.span)?;
                let value = self.coerce(value, field_ty)?;
                let ty = base_value.ty;
                let changed = ir::Expr {
                    kind: ir::ExprKind::SetField(Box::new(base_value), index, Box::new(value)),
                    ty,
                    line,
                };
                if let Some(local) = self.plain_local(base) {
                    return self.store_then_check(base, local, changed, &name.name);
                }
                let changed = self.with_invariants_on(changed, Some(&name.name))?;
                self.assign_path(base, changed)
            }
            ExprKind::TupleField { base, index } => self.set_tuple_field(base, *index, value),
            // `*p = v` and `p[i] = v` (spec 6.4).
            ExprKind::Unary(ast::UnaryOp::Deref, x) => {
                Ok(Stmt::Expr(self.ptr_store(x, None, value)?))
            }
            ExprKind::Index { base, index } if self.is_ptr(base) => {
                Ok(Stmt::Expr(self.ptr_store(base, Some(index), value)?))
            }
            ExprKind::Index { base, index } => {
                // `xs[#k] = v` and `xs[a:b] = ys` (spec 2.19).
                if matches!(index.kind, ExprKind::FromEnd(_) | ExprKind::Part { .. }) {
                    let (mut stmts, changed) = self.set_part(base, index, value, line)?;
                    self.change_in_place(base, changed, &mut stmts)?;
                    return Ok(Stmt::Expr(ir::Expr {
                        kind: ir::ExprKind::Block(ir::Block { stmts, value: None }),
                        ty: Type::Unit,
                        line,
                    }));
                }
                // `items[i] = v` and `ages[name] = v`: a changed copy of the
                // list or map, stored back (changed in place when unshared).
                let base_value = self.expr(base)?;
                let ty = base_value.ty;
                let (op, key, want) = match self.shapes.shape(ty).cloned() {
                    Some(Shape::List(item)) => (RtOp::ListSet, self.expr(index)?, item),
                    Some(Shape::Map(k, v)) => {
                        let key = self.expr(index)?;
                        (RtOp::MapInsert, self.coerce(key, k)?, v)
                    }
                    _ => return Err(self.unsupported("assigning to this", target.span)),
                };
                let value = self.coerce(value, want)?;
                let changed = ir::Expr {
                    kind: ir::ExprKind::Rt(op, vec![base_value, key, value]),
                    ty,
                    line,
                };
                // `r.names[k] = v`: changed where it is (Sakshi's S6).
                let mut stmts = Vec::new();
                self.change_in_place(base, changed, &mut stmts)?;
                Ok(match stmts.len() {
                    1 => stmts.pop().expect("one statement"),
                    _ => Stmt::Expr(ir::Expr {
                        kind: ir::ExprKind::Block(ir::Block { stmts, value: None }),
                        ty: Type::Unit,
                        line,
                    }),
                })
            }
            _ => Err(self.unsupported("assigning to this", target.span)),
        }
    }

    /// Binds the names in pattern `p` to parts of local `whole` (of type
    /// `ty`): `let (code, message) = pair`.
    pub(crate) fn destructure(
        &mut self,
        p: &'a ast::Pattern,
        whole: ir::Local,
        ty: Type,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        let line = self.line(p.span);
        match &p.kind {
            PatternKind::Wildcard => {}
            PatternKind::Binding(id) => {
                let local = self.new_local(ty);
                self.bind(id, local);
                out.push(Stmt::Let(local, self.local_expr(whole, line)));
            }
            PatternKind::Tuple(items) => {
                let Some(Shape::Tuple(types)) = self.shapes.shape(ty).cloned() else {
                    return Err(self.unsupported("this pattern", p.span));
                };
                for (i, (item, t)) in items.iter().zip(types).enumerate() {
                    let at = ir::Expr {
                        kind: ir::ExprKind::Int(i as i128),
                        ty: Type::I64,
                        line,
                    };
                    let whole_value = Box::new(self.local_expr(whole, line));
                    let part = ir::Expr {
                        kind: ir::ExprKind::Index(whole_value, Box::new(at)),
                        ty: t,
                        line,
                    };
                    let local = self.new_local(t);
                    out.push(Stmt::Let(local, part));
                    self.destructure(item, local, t, out)?;
                }
            }
            _ => return Err(self.unsupported("this pattern", p.span)),
        }
        Ok(())
    }

    fn for_loop(
        &mut self,
        pattern: &'a ast::Pattern,
        iter: &'a ast::Expr,
        body: &'a ast::Block,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        if let (
            PatternKind::Binding(_) | PatternKind::Wildcard,
            ExprKind::Binary(op @ (ast::BinaryOp::Range | ast::BinaryOp::RangeInclusive), lo, hi),
        ) = (&pattern.kind, &iter.kind)
        {
            let (start, end) = (self.expr(lo)?, self.expr(hi)?);
            let var = self.new_local(start.ty);
            // `for _ in 0..n` counts without binding the round's number.
            if let PatternKind::Binding(id) = &pattern.kind {
                self.bind(id, var);
            }
            let mut b = ir::Block::default();
            self.block_into(body, false, &mut b)?;
            out.push(Stmt::ForRange {
                var,
                start,
                end,
                inclusive: *op == ast::BinaryOp::RangeInclusive,
                body: b,
            });
            return Ok(());
        }
        // `for tick in time.every_ms(ms)` (spec 14.4).
        if let Some(kavach_checker::Ty::Builtin("Ticker", _)) =
            self.c.expr_types.get(&(iter.span.start, iter.span.end))
        {
            let period = self.expr(iter)?;
            let tick = match &pattern.kind {
                PatternKind::Binding(id) => Some(id),
                _ => None,
            };
            let line = self.line(iter.span);
            // The loop binds the tick, so it is made before the body is
            // lowered; the body then goes at the end of each round.
            let stmts = self.every_loop(tick, period, ir::Block::default(), line);
            let mut b = ir::Block::default();
            self.block_into(body, false, &mut b)?;
            out.extend(stmts.into_iter().map(|s| match s {
                Stmt::While {
                    cond,
                    body: mut round,
                } => {
                    round.stmts.append(&mut b.stmts);
                    Stmt::While { cond, body: round }
                }
                other => other,
            }));
            return Ok(());
        }
        if let Some(kavach_checker::Ty::Builtin("Receiver" | "Lines" | "Chunks", _)) =
            self.c.expr_types.get(&(iter.span.start, iter.span.end))
        {
            return self.receive_loop(pattern, iter, body, out);
        }
        // `for (i, x) in list.enumerate()` walks the list with its index.
        let (list_expr, index_pattern, item_pattern) = match (&pattern.kind, &iter.kind) {
            (PatternKind::Tuple(ps), ExprKind::Call { callee, args, .. })
                if ps.len() == 2 && args.is_empty() =>
            {
                match &callee.kind {
                    ExprKind::Field { base, name, .. } if name.name == "enumerate" => {
                        (&**base, Some(&ps[0]), &ps[1])
                    }
                    // Another call, as `for (k, v) in cache.get()`: a plain
                    // loop over what it gives.
                    _ => (iter, None, pattern),
                }
            }
            _ => (iter, None, pattern),
        };
        let mut list_value = self.expr(list_expr)?;
        if let Some(Shape::Set(item)) = self.shapes.shape(list_value.ty).cloned() {
            // `for x in set`: over its values, in order.
            let ty = self.list_type(item);
            let line = list_value.line;
            list_value = ir::Expr {
                kind: ir::ExprKind::Rt(RtOp::MapKeys, vec![list_value]),
                ty,
                line,
            };
        }
        if let Some(Shape::Map(k, v)) = self.shapes.shape(list_value.ty).cloned() {
            // `for (key, value) in map`: over its entries, in order.
            let pair = self.tuple_type(vec![k, v]);
            let ty = self.list_type(pair);
            let line = list_value.line;
            list_value = ir::Expr {
                kind: ir::ExprKind::Rt(RtOp::MapEntries, vec![list_value]),
                ty,
                line,
            };
        }
        let Some(Shape::List(item_ty)) = self.shapes.shape(list_value.ty).cloned() else {
            return Err(self.unsupported("`for` over this", iter.span));
        };
        let list = self.new_local(list_value.ty);
        out.push(Stmt::Let(list, list_value));
        let var = self.new_local(item_ty);
        let index = self.new_local(Type::I64);
        let mut b = ir::Block::default();
        let parts = [
            (Some(item_pattern), var, item_ty),
            (index_pattern, index, Type::I64),
        ];
        for (p, local, ty) in parts {
            match p.map(|p| (p, &p.kind)) {
                Some((_, PatternKind::Binding(id))) => self.bind(id, local),
                Some((_, PatternKind::Wildcard)) | None => {}
                Some((p, _)) => self.destructure(p, local, ty, &mut b.stmts)?,
            }
        }
        self.block_into(body, false, &mut b)?;
        out.push(Stmt::ForEach {
            var,
            index,
            list,
            body: b,
        });
        Ok(())
    }

    /// An expression used as a statement.
    pub(crate) fn expr_stmt(&mut self, e: &'a ast::Expr, out: &mut Vec<Stmt>) -> Lowered<()> {
        match &e.kind {
            // `return print(e)` in a function that gives nothing: the call
            // as a statement, then a plain `return`.
            ExprKind::Return(Some(v)) if self.ret == Type::Unit && self.ty_of(v)? == Type::Unit => {
                self.expr_stmt(v, out)?;
                if self.result.is_some() {
                    self.ensure_checks(out)?;
                }
                out.push(Stmt::Return(None));
            }
            ExprKind::Return(value) => {
                let ret = self.ret;
                let value = value
                    .as_deref()
                    .map(|v| self.expr(v))
                    .transpose()?
                    .map(|v| self.coerce(v, ret))
                    .transpose()?;
                // With `ensure`, the value is checked on the way out too.
                if let (Some(result), Some(v)) = (self.result, &value) {
                    let line = v.line;
                    out.push(Stmt::Let(result, v.clone()));
                    self.ensure_checks(out)?;
                    out.push(Stmt::Return(Some(self.local_expr(result, line))));
                    return Ok(());
                }
                if value.is_none() && self.result.is_some() {
                    self.ensure_checks(out)?;
                }
                out.push(Stmt::Return(value));
            }
            ExprKind::Break(label) => self.jump(label, false, out),
            ExprKind::Continue(label) => self.jump(label, true, out),
            // Native code is a release build: `debug(…)` is left out (spec
            // 1.10), its argument not even computed.
            ExprKind::Call { callee, .. } if self.is_builtin(callee, "debug") => {}
            ExprKind::Call { callee, args, .. } if self.is_builtin(callee, "print") => {
                let arg = args
                    .first()
                    .map(|a| &a.value)
                    .ok_or_else(|| self.unsupported("`print` without an argument", e.span))?;
                // A text worked out before its task started is printed whole.
                let held = self
                    .held_parts
                    .contains_key(&(arg.span.start, arg.span.end));
                let parts = match &arg.kind {
                    ExprKind::Str(parts) if !held => parts
                        .iter()
                        .map(|p| match p {
                            ast::StrPart::Text(t) => Ok(PrintPart::Text(t.clone())),
                            ast::StrPart::Expr(x) => Ok(PrintPart::Value(self.expr(x)?)),
                        })
                        .collect::<Lowered<Vec<_>>>()?,
                    _ => vec![PrintPart::Value(self.expr(arg)?)],
                };
                out.push(Stmt::Print(parts));
            }
            ExprKind::Call { callee, .. } if self.is_mutating_call(callee) => {
                // `list.push(x)` changes the list stored in a variable or field.
                let ExprKind::Field { base, .. } = &callee.kind else {
                    return Err(self.unsupported("this call", e.span));
                };
                // The place's keys once, before the arguments (spec 3.10).
                let held = self.hold_keys(base, out)?;
                let changed = self.expr(e);
                let done = changed.and_then(|c| self.change_in_place(base, c, out));
                self.release(held);
                done?;
            }
            ExprKind::If { .. }
            | ExprKind::IfLet { .. }
            | ExprKind::Match { .. }
            | ExprKind::Block(_)
            | ExprKind::Arena(_)
            | ExprKind::Unsafe(_) => out.push(Stmt::Expr(self.branching(e, false)?)),
            _ => out.push(Stmt::Expr(self.expr(e)?)),
        }
        Ok(())
    }
}
