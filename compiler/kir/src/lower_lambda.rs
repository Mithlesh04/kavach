//! Lowering lambdas written where they are used (spec 3.3): they are
//! inlined. `items.filter(.paid)` becomes a loop that builds the new
//! list, with the lambda's body in place, so it needs no closure object.
//! Other function values are closures (`lower_closure.rs`).

use crate::ir::{self, BinOp, Block, ExprKind as K, Local, Shape, Stmt, Type, UnOp};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, ExprKind};
use kavach_checker::Ty;
use kavach_resolver::DefKind;

mod scan;
use scan::{assigned, mentions};

/// List methods that take a lambda and become a loop.
pub(crate) const LIST_LOOPS: [&str; 8] = [
    "filter", "map", "flat_map", "any", "all", "sum", "find", "count",
];

impl<'a> Lowerer<'a> {
    /// The function `f` names, when calling it with `n` values needs
    /// nothing a plain call doesn't: not generic, not from C, no errors,
    /// no `inout` and no defaults left out.
    fn plain_fn(&self, f: &ast::Expr, n: usize) -> Option<ir::FuncId> {
        let ExprKind::Name(id) = &f.kind else {
            return None;
        };
        let res = &self.c.resolution;
        let def = *res.uses.get(&id.span.start)?;
        if !matches!(res.defs[def].kind, DefKind::Fn) || self.generic_fns.contains_key(&def) {
            return None;
        }
        let fid = *self.fn_ids.get(&def)?;
        let decl = self.decl_of(fid);
        let plain = decl.c_library.is_none()
            && decl.errors.is_none()
            && decl.params.len() == n
            && decl.params.iter().all(|p| !p.is_inout);
        plain.then_some(fid)
    }

    /// The lambda's body with `args` bound to its parameters.
    pub(crate) fn inline_lambda(
        &mut self,
        f: &'a ast::Expr,
        args: Vec<ir::Expr>,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let (stmts, body) = match &f.kind {
            ExprKind::Lambda { params, body } => {
                let mut stmts = Vec::new();
                for (p, arg) in params.iter().zip(args) {
                    // A parameter never changes, so one given a local (a
                    // loop's item) is that local: no copy, and a flat
                    // list's item is read in place instead of boxed.
                    if let K::Local(l) = arg.kind {
                        self.bind(&p.name, l);
                        continue;
                    }
                    let local = self.new_local(arg.ty);
                    self.bind(&p.name, local);
                    stmts.push(Stmt::Let(local, arg));
                }
                // A lambda keeps a copy of each outside variable (spec
                // 3.3): one it changes is a copy for this call, so the
                // change never reaches the variable.
                let mut outside = Vec::new();
                for def in assigned(body, &self.c.resolution) {
                    let Some(&l) = self.local_ids.get(&def) else {
                        continue;
                    };
                    let copy = self.new_local(self.locals[l as usize]);
                    let read = ir::Expr {
                        kind: K::Local(l),
                        ty: self.locals[l as usize],
                        line,
                    };
                    stmts.push(Stmt::Let(copy, read));
                    self.local_ids.insert(def, copy);
                    outside.push((def, l));
                }
                let body = self.expr(body);
                for (def, l) in outside {
                    self.local_ids.insert(def, l);
                }
                (stmts, body?)
            }
            ExprKind::DotLambda(body) => {
                let Some(arg) = args.into_iter().next() else {
                    return Err(self.unsupported("this lambda", f.span));
                };
                let ret = match self.c.expr_types.get(&(f.span.start, f.span.end)) {
                    Some(Ty::Fn(_, ret, _)) => self.kir_type(&ret.clone(), f.span)?,
                    _ => return Err(self.unsupported("this lambda", f.span)),
                };
                let local = self.new_local(arg.ty);
                self.its.push(local);
                let value = self.expr_as(body, ret);
                self.its.pop();
                (vec![Stmt::Let(local, arg)], value?)
            }
            // A named function (`items.map(double)`): called directly, as
            // in `x => double(x)`, so the backend can inline it.
            _ if self.plain_fn(f, args.len()).is_some() => {
                let fid = self.plain_fn(f, args.len()).expect("checked above");
                let fn_ty = self.ty_of(f)?;
                let (params, ret) = self.fn_parts(fn_ty, f.span)?;
                let args = args
                    .into_iter()
                    .zip(params)
                    .map(|(a, t)| self.coerce(a, t))
                    .collect::<Lowered<Vec<_>>>()?;
                return Ok(ir::Expr {
                    kind: K::Call(fid, args),
                    ty: ret,
                    line,
                });
            }
            // Any other function value (a variable, a parameter…): called.
            _ => {
                let value = self.expr(f)?;
                return self.call_value(value, args, f.span);
            }
        };
        let ty = body.ty;
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts,
                value: Some(Box::new(body)),
            }),
            ty,
            line,
        })
    }

    /// `filter`, `map`, `flat_map`, `any`, `all` and `sum` with lambda `f`,
    /// as a loop over the list.
    pub(crate) fn list_loop(
        &mut self,
        method: &str,
        list: ir::Expr,
        f: &'a ast::Expr,
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let Some(Shape::List(item_ty)) = self.shapes.shape(list.ty).cloned() else {
            return Err(self.unsupported(format!("`{method}` here"), span));
        };
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let src = self.new_local(list.ty);
        let (item, index, acc) = (
            self.new_local(item_ty),
            self.new_local(Type::I64),
            self.new_local(ty),
        );
        let start = match method {
            "filter" | "map" | "flat_map" => K::List(Vec::new()),
            "find" => K::None,
            "any" => K::Bool(false),
            "all" => K::Bool(true),
            _ if ty.is_float() => K::Float(0.0),
            _ if matches!(self.shapes.shape(ty), Some(Shape::Decimal)) => K::Decimal("0".into()),
            _ => K::Int(0),
        };
        let mut stmts = vec![Stmt::Let(src, list), Stmt::Let(acc, at(start, ty))];
        let item_value = self.local_expr(item, line);
        // A function that can fail is called, not inlined: its first
        // error ends the loop (spec 5.2).
        let (value, round) = match self.failing_rounds(f, item_value.clone())? {
            Some((setup, round, value)) => {
                stmts.extend(setup);
                (value, round)
            }
            None => (self.inline_lambda(f, vec![item_value], line)?, Vec::new()),
        };
        let body = match method {
            "filter" => {
                let keep = self.push_to(acc, self.local_expr(item, line));
                at(K::If(Box::new(value), keep, Block::default()), Type::Unit)
            }
            "map" => {
                let pushed = self.push_to(acc, value);
                at(K::Block(pushed), Type::Unit)
            }
            "any" | "all" => {
                // Every item is visited, like the interpreter, so a lambda's
                // side effects happen the same way.
                let flip = method == "all";
                let cond = if flip {
                    at(K::Unary(UnOp::Not, Box::new(value)), Type::Bool)
                } else {
                    value
                };
                let flag = at(K::Bool(!flip), Type::Bool);
                let set = self.assign(acc, flag);
                at(K::If(Box::new(cond), set, Block::default()), Type::Unit)
            }
            "count" => {
                let one = at(K::Int(1), ty);
                let total = self.local_expr(acc, line);
                let added = at(K::Binary(BinOp::Add, Box::new(total), Box::new(one)), ty);
                at(
                    K::If(Box::new(value), self.assign(acc, added), Block::default()),
                    Type::Unit,
                )
            }
            "find" => {
                // The first match is kept; every item is still visited, like
                // the interpreter, so a lambda's side effects match.
                let present = at(K::IsSome(Box::new(self.local_expr(acc, line))), Type::Bool);
                let first = at(K::Unary(UnOp::Not, Box::new(present)), Type::Bool);
                let cond = at(K::And(Box::new(value), Box::new(first)), Type::Bool);
                let found = self.coerce(self.local_expr(item, line), ty)?;
                at(
                    K::If(Box::new(cond), self.assign(acc, found), Block::default()),
                    Type::Unit,
                )
            }
            "sum" => {
                let total = self.local_expr(acc, line);
                let added = at(K::Binary(BinOp::Add, Box::new(total), Box::new(value)), ty);
                at(K::Block(self.assign(acc, added)), Type::Unit)
            }
            _ => self.flat_push(acc, value, line)?,
        };
        let mut each = round;
        each.push(Stmt::Expr(body));
        stmts.push(Stmt::ForEach {
            var: item,
            index,
            list: src,
            body: Block {
                stmts: each,
                value: None,
            },
        });
        Ok(at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(self.local_expr(acc, line))),
            }),
            ty,
        ))
    }

    /// `flat_map`: pushes every item of the inner list `value` onto `acc`.
    fn flat_push(&mut self, acc: Local, value: ir::Expr, line: u32) -> Lowered<ir::Expr> {
        let Some(Shape::List(inner_item)) = self.shapes.shape(value.ty).cloned() else {
            return Err(crate::Unsupported {
                feature: "`flat_map` with a lambda that doesn't give a list".into(),
                line,
            });
        };
        let (inner, var, index) = (
            self.new_local(value.ty),
            self.new_local(inner_item),
            self.new_local(Type::I64),
        );
        let walk = Stmt::ForEach {
            var,
            index,
            list: inner,
            body: self.push_to(acc, self.local_expr(var, line)),
        };
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts: vec![Stmt::Let(inner, value), walk],
                value: None,
            }),
            ty: Type::Unit,
            line,
        })
    }

    /// A block that sets local `acc` to `value`.
    fn assign(&self, acc: Local, value: ir::Expr) -> Block {
        let line = value.line;
        Block {
            stmts: vec![Stmt::Expr(ir::Expr {
                kind: K::Assign(acc, Box::new(value)),
                ty: Type::Unit,
                line,
            })],
            value: None,
        }
    }

    /// A block that pushes `value` onto the list in local `acc`.
    fn push_to(&self, acc: Local, value: ir::Expr) -> Block {
        let line = value.line;
        let list_ty = self.locals[acc as usize];
        let pushed = ir::Expr {
            kind: K::Rt(RtOp::ListPush, vec![self.local_expr(acc, line), value]),
            ty: list_ty,
            line,
        };
        self.assign(acc, pushed)
    }

    /// `ps = ps.map(f)` with items of the same type: each item replaced
    /// where it is (`ps[i] = f(ps[i])`), so no new list is made. Unshared,
    /// the list changes in place; shared, the first change copies it, as
    /// any change does (2.2). `f` must not read `ps`, which it would see
    /// half changed; a `map` that may run on the GPU is left to it.
    pub(crate) fn map_in_place(
        &mut self,
        target: &'a ast::Expr,
        value: &'a ast::Expr,
    ) -> Lowered<Option<Stmt>> {
        let (ExprKind::Name(var), ExprKind::Call { callee, args, .. }) =
            (&target.kind, &value.kind)
        else {
            return Ok(None);
        };
        let ExprKind::Field {
            base,
            name,
            optional: false,
        } = &callee.kind
        else {
            return Ok(None);
        };
        let (ExprKind::Name(list), [f]) = (&base.kind, &args[..]) else {
            return Ok(None);
        };
        let res = &self.c.resolution;
        let def = res.uses.get(&var.span.start);
        let gpu = self
            .c
            .gpu_maps
            .contains(&(value.span.start, value.span.end));
        if name.name != "map" || def.is_none() || res.uses.get(&list.span.start) != def {
            return Ok(None);
        }
        // A named function can't read the caller's `ps`; a lambda might.
        let named = self.plain_fn(&f.value, 1).is_some();
        let lambda = matches!(
            f.value.kind,
            ExprKind::Lambda { .. } | ExprKind::DotLambda(_)
        );
        if !(named || lambda) || mentions(&f.value, def.copied(), res) {
            return Ok(None);
        }
        let local = self.local(var)?;
        let list_ty = self.locals[local as usize];
        let Some(Shape::List(item_ty)) = self.shapes.shape(list_ty).cloned() else {
            return Ok(None);
        };
        if self.ty_of(value)? != list_ty {
            return Ok(None);
        }
        let line = self.line(value.span);
        // A `map` that may run on the GPU changes its own local `work`:
        // `ps` may hold a list that is only on the GPU, so each read of it
        // fetches it (`gpu_stay.rs`); the list is taken out of `ps` once.
        let work = if gpu { self.new_local(list_ty) } else { local };
        let (item, index) = (self.new_local(item_ty), self.new_local(Type::I64));
        let new = self.inline_lambda(&f.value, vec![self.local_expr(item, line)], line)?;
        let new = self.coerce(new, item_ty)?;
        let set = ir::Expr {
            kind: K::Rt(
                RtOp::ListSet,
                vec![
                    self.local_expr(work, line),
                    self.local_expr(index, line),
                    new,
                ],
            ),
            ty: list_ty,
            line,
        };
        let in_place = Stmt::ForEach {
            var: item,
            index,
            list: work,
            body: self.assign(work, set),
        };
        if !gpu {
            return Ok(Some(in_place));
        }
        // A `map` that may run on the GPU (`lower_gpu.rs`): the GPU's try
        // stays as it is, so its list may stay on the GPU (`gpu_stay.rs`);
        // when the CPU does it, it does it in place.
        let mut tried =
            self.gpu_map(self.local_expr(local, line), &f.value, list_ty, value.span)?;
        let K::Block(b) = &mut tried.kind else {
            return Ok(None);
        };
        let Some(K::If(_, _, cpu)) = b.value.as_deref_mut().map(|v| &mut v.kind) else {
            return Ok(None);
        };
        // `KAVACH_GPU=check` compares the CPU's list with the GPU's.
        let checked = ir::Expr {
            kind: K::Rt(RtOp::GpuCheck, vec![self.local_expr(work, line)]),
            ty: list_ty,
            line,
        };
        // Moved, so `work` is the list's one holder and changes in place.
        let taken = ir::Expr {
            kind: K::Move(local),
            ty: list_ty,
            line,
        };
        *cpu = Block {
            stmts: vec![Stmt::Let(work, taken), in_place],
            value: Some(Box::new(checked)),
        };
        Ok(Some(Stmt::Expr(ir::Expr {
            kind: K::Assign(local, Box::new(tried)),
            ty: Type::Unit,
            line,
        })))
    }

    /// `shared.update(f)`: stores `f` of the current value (spec 6.2).
    /// `s.update(f)`, which gives the new value, and `s.update_with(f)`,
    /// whose `f` gives a pair: the new value and the call's result
    /// (spec 6.2). `ty` is the call's type.
    pub(crate) fn shared_update(
        &mut self,
        shared: ir::Expr,
        f: &'a ast::Expr,
        with: bool,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let Some(Shape::Shared(inner)) = self.shapes.shape(shared.ty).cloned() else {
            return Err(self.unsupported("`update` here", f.span));
        };
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let local = self.new_local(shared.ty);
        let current = at(
            K::Rt(RtOp::SharedGet, vec![self.local_expr(local, line)]),
            inner,
        );
        let given = self.inline_lambda(f, vec![current], line)?;
        // The lock is held from before the read to after the write, so no
        // other task's update comes between (decision 0052).
        let lock = |l: &Self, op| at(K::Rt(op, vec![l.local_expr(local, line)]), Type::Unit);
        let (take, let_go) = (lock(self, RtOp::SharedLock), lock(self, RtOp::SharedUnlock));
        let mut stmts = vec![Stmt::Let(local, shared), Stmt::Expr(take)];
        let result = self.new_local(ty);
        if with {
            let Some(Shape::Tuple(parts)) = self.shapes.shape(given.ty).cloned() else {
                return Err(self.unsupported("`update_with` here", f.span));
            };
            let pair_ty = given.ty;
            let pair = self.new_local(pair_ty);
            stmts.push(Stmt::Let(pair, given));
            let new = self.tuple_item(pair, pair_ty, 0, parts[0], line);
            let new = self.coerce(new, inner)?;
            let out = self.tuple_item(pair, pair_ty, 1, parts[1], line);
            stmts.push(Stmt::Expr(at(
                K::Rt(RtOp::SharedSet, vec![self.local_expr(local, line), new]),
                Type::Unit,
            )));
            stmts.push(Stmt::Let(result, self.coerce(out, ty)?));
        } else {
            let new = self.coerce(given, inner)?;
            stmts.push(Stmt::Expr(at(
                K::Rt(RtOp::SharedSet, vec![self.local_expr(local, line), new]),
                Type::Unit,
            )));
            // The new value, read while the lock is still held.
            let now = at(
                K::Rt(RtOp::SharedGet, vec![self.local_expr(local, line)]),
                inner,
            );
            stmts.push(Stmt::Let(result, self.coerce(now, ty)?));
        }
        stmts.push(Stmt::Expr(let_go));
        Ok(at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(self.local_expr(result, line))),
            }),
            ty,
        ))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_named_function_given_to_map_is_called_directly() {
        let src = "fn double(x: Int) -> Int {\n    x * 2\n}\n\nfn main() {\n    print([1, 2].map(double))\n}\n";
        let c = kavach_driver::compile(src, "main.kav", Default::default());
        let program = crate::lower(&c).expect("lowered");
        let main = &program.functions[program.entry as usize];
        let body = format!("{:?}", main.body);
        // Called directly, so inlined: no call through a function value.
        assert!(!body.contains("CallValue"), "{body}");
    }
}
