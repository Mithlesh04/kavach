// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Method calls (spec 2.5, 2.15, 3.4): user methods, methods through
//! interface values, and fields holding functions. Split from
//! `lower_calls.rs` (named functions and arguments).

use crate::ir::{self, ExprKind as K, Shape, Type};
use crate::lower::{Lowered, Lowerer, Owner};
use crate::lower_lambda::LIST_LOOPS;
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, ExprKind};
use kavach_checker::Ty;
use kavach_resolver::DefKind;

impl<'a> Lowerer<'a> {
    pub(crate) fn method_call(
        &mut self,
        base: &'a ast::Expr,
        name: &'a ast::Ident,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        call: &'a ast::Expr,
    ) -> Lowered<ir::Expr> {
        let span = call.span;
        // A method of an error that stands in for one never made: this
        // code never runs (`lower_generic.rs`).
        if let Some(kavach_checker::Ty::Param(p)) =
            self.c.expr_types.get(&(base.span.start, base.span.end))
            && self.stand_ins.contains(p)
        {
            let why = "an error that is never made was used".to_string();
            return Ok(ir::Expr {
                kind: K::Panic(why),
                ty,
                line,
            });
        }
        if let Some(on) = self.map_on_device(base, name, args, ty, line, call) {
            return on;
        }
        if name.name == "unfreeze" && self.c.sealed_unfreezes.contains(&(span.start, span.end)) {
            return self.sealed_unfreeze(base, args, ty, line);
        }
        if name.name == "pop"
            && let Some(popped) = self.list_pop(base, ty, line)?
        {
            return Ok(popped);
        }
        if name.name == "draw"
            && let Some(drawn) = self.frame_draw(base, args, ty, line)?
        {
            return Ok(drawn);
        }
        if name.name == "timeout"
            && let Some(timed) = self.timeout(base, args, ty, line)?
        {
            return Ok(timed);
        }
        if let Some(access) = self.ptr_method(base, &name.name, args, line, span) {
            return access;
        }
        // TLS begun on a plain connection (spec 14.10).
        if matches!(name.name.as_str(), "start_tls" | "accept_tls")
            && matches!(
                self.c.expr_types.get(&(base.span.start, base.span.end)),
                Some(kavach_checker::Ty::Builtin("Connection", _))
            )
        {
            return self.start_tls_call(base, &name.name, args, ty, line);
        }
        // `Rect.square(3)` and `Shape.Circle(radius: 1)`: through a type name.
        if let ExprKind::Name(id) = &base.kind
            && let Some(&def) = self.c.resolution.uses.get(&id.span.start)
        {
            if self.c.resolution.defs[def].kind == DefKind::Type {
                if let Some(f) = self.generic_methods.get(&(def, name.name.clone())).copied() {
                    return self.generic_method_call(def, f, None, args, ty, line, call);
                }
                if let Some(&fid) = self.method_ids.get(&(Owner::Adt(def), name.name.clone())) {
                    let values = self.arguments(self.decl_of(fid), args, span)?;
                    return Ok(ir::Expr {
                        kind: K::Call(fid, values),
                        ty,
                        line,
                    });
                }
                if let Some(e) = self.enum_call(def, &name.name, args, ty, line, span) {
                    return e;
                }
                return self.construct(def, Some(&name.name), args, ty, line, span);
            }
            match self.c.resolution.defs[def].kind {
                DefKind::Agent if name.name == "start" => {
                    return self.agent_start(def, args, line, span);
                }
                DefKind::Capability => {
                    return self.capability_call(def, name, args, ty, line, span);
                }
                _ => {}
            }
            if let Some(module) = self.std_modules.get(&def).cloned() {
                return self.std_call(&module, &name.name, args, ty, line, span);
            }
            if let Some(e) = self.builtin_static(def, &id.name, &name.name, args, ty, line, span) {
                return e;
            }
            // A `Field` is one named in an `invariant` (spec 7.3).
            if !matches!(
                self.c.resolution.defs[def].kind,
                DefKind::Local { .. }
                    | DefKind::Param
                    | DefKind::Result
                    | DefKind::Variant { .. }
                    | DefKind::Field
            ) {
                return Err(self.unsupported(format!("`{}.{}`", id.name, name.name), span));
            }
        }
        if let Some(fused) = self.fused_gpu_maps(base, name, args, ty, span) {
            return fused;
        }
        if name.name == "text"
            && let Some(text) = self.bytes_text(base, args, ty, line)?
        {
            return Ok(text);
        }
        let receiver = self.expr(base)?;
        if let Some(m) = self.collection_method(base, &name.name) {
            return self.collection_method_call(m, receiver, args, ty, line, call);
        }
        let owner = match (
            self.shapes.def_of(receiver.ty),
            self.shapes.shape(receiver.ty),
            receiver.ty,
        ) {
            (Some(def), _, _) => Some(Owner::Adt(def)),
            (None, Some(Shape::Str), _) => Some(Owner::Builtin("String".into())),
            (None, None, t) if t != Type::Unit => {
                Some(Owner::Builtin(crate::lower_calls::scalar_name(t).into()))
            }
            _ => None,
        };
        if let Some(Owner::Adt(def)) = &owner
            && let Some(f) = self
                .generic_methods
                .get(&(*def, name.name.clone()))
                .copied()
        {
            return self.generic_method_call(*def, f, Some(receiver), args, ty, line, call);
        }
        // `task.wait()` (spec 9.1): it can fail, with the task's error.
        // `task.cancel()` (9.6): it stops at once.
        let task = matches!(
            self.c.expr_types.get(&(base.span.start, base.span.end)),
            Some(Ty::Builtin("Task", _))
        );
        if task && (name.name == "wait" || name.name == "cancel") {
            let op = if name.name == "wait" {
                RtOp::TaskWait
            } else {
                RtOp::TaskCancel
            };
            return Ok(ir::Expr {
                kind: K::Rt(op, vec![receiver]),
                ty,
                line,
            });
        }
        if let Some((op, operands)) =
            self.json_method(owner.as_ref(), &name.name, &receiver, args)?
        {
            return Ok(ir::Expr {
                kind: K::Rt(op, operands),
                ty,
                line,
            });
        }
        if let Some(&fid) = owner
            .clone()
            .and_then(|o| self.method_ids.get(&(o, name.name.clone())))
        {
            let mut values = vec![receiver];
            values.extend(self.arguments(self.decl_of(fid), args, span)?);
            let call = ir::Expr {
                kind: K::Call(fid, values),
                ty,
                line,
            };
            return self.write_back(fid, call, args, 1, ty);
        }
        // `v.name()` and `v.value()` on a union value (spec 2.6).
        if let Some(Owner::Adt(def)) = owner
            && let Some(e) = self.enum_method(&receiver, def, &name.name, line)
        {
            return Ok(e);
        }
        // A method through an interface value (spec 2.15): the stored
        // closure for it, called.
        if let Some(Shape::Iface { methods, .. }) = self.shapes.shape(receiver.ty).cloned()
            && let Some(i) = methods.iter().position(|(n, _)| *n == name.name)
        {
            let fty = methods[i].1;
            let receiver_ty = receiver.ty;
            let get = ir::Expr {
                kind: K::GetField(Box::new(receiver), (2 + i) as u32),
                ty: fty,
                line,
            };
            let Some(Shape::Func(wants, ..)) = self.shapes.shape(fty).cloned() else {
                return Err(self.unsupported("this interface method", span));
            };
            let mut values = Vec::new();
            for (a, want) in args.iter().zip(wants) {
                let v = self.arg_of(self.iface_method_decl(receiver_ty, &name.name), args, a)?;
                values.push(self.coerce(v, want)?);
            }
            let call = ir::Expr {
                kind: K::CallValue(Box::new(get), values),
                ty,
                line,
            };
            return match self.iface_method_decl(receiver_ty, &name.name) {
                Some(decl) => self.write_back_of(decl, call, args, 0, ty),
                None => Ok(call),
            };
        }
        // A field holding a function: `h.on(x)` calls it (spec 3.4).
        if let Some(Shape::Record { fields, .. }) = self.shapes.shape(receiver.ty).cloned() {
            let found = fields.iter().enumerate().find(|(_, f)| f.name == name.name);
            if let Some((i, f)) = found
                && matches!(self.shapes.shape(f.ty), Some(Shape::Func(..)))
            {
                let get = ir::Expr {
                    kind: K::GetField(Box::new(receiver), i as u32),
                    ty: f.ty,
                    line,
                };
                let mut values = Vec::new();
                for a in args {
                    values.push(self.expr(&a.value)?);
                }
                return Ok(ir::Expr {
                    kind: K::CallValue(Box::new(get), values),
                    ty,
                    line,
                });
            }
        }
        if name.name == "read" && matches!(self.shapes.shape(receiver.ty), Some(Shape::Bytes)) {
            return self.bytes_read(receiver, args, ty, line);
        }
        let is_list = matches!(self.shapes.shape(receiver.ty), Some(Shape::List(_)));
        // `map`, `filter`, `sum`, `min` and `max` on the GPU (spec 15.2).
        if is_list && self.c.gpu_maps.contains(&(span.start, span.end)) {
            match (name.name.as_str(), args) {
                ("map", [f]) => return self.gpu_map(receiver, &f.value, ty, span),
                ("filter", [f]) => {
                    let op = RtOp::GpuFilter;
                    return self.gpu_list_op(op, "filter", receiver, &f.value, ty, span);
                }
                ("sum" | "min" | "max", _) => {
                    let f = args.first().map(|a| &a.value);
                    return self.gpu_reduce(&name.name, receiver, f, ty, span);
                }
                _ => {}
            }
        }
        if let ([f], true) = (args, is_list && LIST_LOOPS.contains(&name.name.as_str())) {
            return self.list_loop(&name.name, receiver, &f.value, ty, span);
        }
        if let ([f], Some(Shape::Set(item))) = (args, self.shapes.shape(receiver.ty).cloned())
            && crate::lower_set_loops::SET_LOOPS.contains(&name.name.as_str())
        {
            return self.set_loop(&name.name, receiver, item, &f.value, ty, span);
        }
        if let (true, "parallel_map") = (is_list, name.name.as_str()) {
            return self.parallel_map(receiver, args, ty, span);
        }
        if let ([f], true, Some(op)) = (args, is_list, crate::lower_std::by_keys(&name.name)) {
            return self.sort_by(op, receiver, &f.value, ty, span);
        }
        if let (Some(Shape::Shared(_)), "update" | "update_with", [f]) =
            (self.shapes.shape(receiver.ty), name.name.as_str(), args)
        {
            let with = name.name == "update_with";
            let fn_ty = self.ty_of(&f.value)?;
            if self.fn_error(fn_ty).is_some() {
                let failing = self.failing_update(receiver, &f.value, with, ty, line)?;
                return failing.ok_or_else(|| self.unsupported("`update` here", span));
            }
            return self.shared_update(receiver, &f.value, with, ty, line);
        }
        let mut values = vec![receiver];
        for a in args {
            values.push(self.expr(&a.value)?);
        }
        if let Some(made) = self.view_method(base, &name.name, &mut values, ty, line)? {
            return Ok(made);
        }
        if let Some(p) = self.whole_of(base)
            && let Some(made) = self.whole_method(&name.name, &mut values, p, ty, line)
        {
            return Ok(made);
        }
        self.value_method(&name.name, values, ty, line, span)
    }
}
