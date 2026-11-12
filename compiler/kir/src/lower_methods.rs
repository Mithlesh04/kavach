//! Method calls (spec 2.5, 2.15, 3.4): user methods, methods through
//! interface values, and fields holding functions. Split from
//! `lower_calls.rs` (named functions and arguments).

use crate::ir::{self, ExprKind as K, Shape, Type, UnOp};
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
        if name.name == "timeout"
            && let Some(timed) = self.timeout(base, args, ty, line)?
        {
            return Ok(timed);
        }
        if let Some(access) = self.ptr_method(base, &name.name, args, line, span) {
            return access;
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
            if self.c.resolution.defs[def].kind == DefKind::Builtin
                && id.name == "Set"
                && name.name == "of"
            {
                return self.set_of(args, ty, line);
            }
            if self.c.resolution.defs[def].kind == DefKind::Builtin
                && matches!(id.name.as_str(), "Map" | "Set")
                && name.name == "with_capacity"
            {
                let n = args
                    .first()
                    .ok_or_else(|| self.unsupported("`with_capacity()`", span))?;
                let n = self.expr(&n.value)?;
                return Ok(ir::Expr {
                    kind: K::Rt(RtOp::MapWithCapacity, vec![n]),
                    ty,
                    line,
                });
            }
            if id.name == "Ptr" && name.name == "from_int" {
                let n = args
                    .first()
                    .ok_or_else(|| self.unsupported("`Ptr.from_int()`", span))?;
                let n = self.expr(&n.value)?;
                return Ok(ir::Expr { ty, ..n });
            }
            if id.name == "Float" && name.name == "from_bits" {
                let bits = self.expr(&args[0].value)?;
                let kind = K::Rt(RtOp::FloatFromBits, vec![bits]);
                return Ok(ir::Expr { kind, ty, line });
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
        let receiver = self.expr(base)?;
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
                let v = self.expr(&a.value)?;
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
        let is_list = matches!(self.shapes.shape(receiver.ty), Some(Shape::List(_)));
        // A `map` that may run on the GPU (spec 15.2).
        if let ([f], true) = (
            args,
            is_list && name.name == "map" && self.c.gpu_maps.contains(&(span.start, span.end)),
        ) {
            return self.gpu_map(receiver, &f.value, ty, span);
        }
        if let ([f], true) = (
            args,
            is_list && name.name == "filter" && self.c.gpu_maps.contains(&(span.start, span.end)),
        ) {
            return self.gpu_list_op(RtOp::GpuFilter, "filter", receiver, &f.value, ty, span);
        }
        // `sum`, `min` and `max` that may run on the GPU (spec 15.2).
        if is_list
            && matches!(name.name.as_str(), "sum" | "min" | "max")
            && self.c.gpu_maps.contains(&(span.start, span.end))
        {
            let f = args.first().map(|a| &a.value);
            return self.gpu_reduce(&name.name, receiver, f, ty, span);
        }
        if let ([f], true) = (args, is_list && LIST_LOOPS.contains(&name.name.as_str())) {
            return self.list_loop(&name.name, receiver, &f.value, ty, span);
        }
        if let (true, "parallel_map") = (is_list, name.name.as_str()) {
            return self.parallel_map(receiver, args, ty, span);
        }
        if let ([f], true, "sort_by") = (args, is_list, name.name.as_str()) {
            return self.sort_by(receiver, &f.value, ty, span);
        }
        if let (Some(Shape::Shared(_)), "update" | "update_with", [f]) =
            (self.shapes.shape(receiver.ty), name.name.as_str(), args)
        {
            let with = name.name == "update_with";
            return self.shared_update(receiver, &f.value, with, ty, line);
        }
        let mut values = vec![receiver];
        for a in args {
            values.push(self.expr(&a.value)?);
        }
        let shape = self.shapes.shape(values[0].ty).cloned();
        let is_int = values[0].ty.is_int();
        // `b.is_empty()` on `Bytes` is `b.len() == 0`.
        if matches!(shape, Some(Shape::Bytes)) && name.name == "is_empty" {
            let len = ir::Expr {
                kind: K::Rt(RtOp::BytesLen, values),
                ty: Type::I64,
                line,
            };
            let zero = ir::Expr {
                kind: K::Int(0),
                ty: Type::I64,
                line,
            };
            return Ok(ir::Expr {
                kind: K::Binary(ir::BinOp::Eq, Box::new(len), Box::new(zero)),
                ty: Type::Bool,
                line,
            });
        }
        let op = match (shape, name.name.as_str()) {
            (Some(Shape::Str), "len") => RtOp::StrLen,
            (Some(Shape::Str), "is_empty") => RtOp::StrIsEmpty,
            (Some(Shape::Str), "upper") => RtOp::StrUpper,
            (Some(Shape::Str), "lower") => RtOp::StrLower,
            (Some(Shape::Str), "trim") => RtOp::StrTrim,
            (Some(Shape::Str), "contains") => RtOp::StrContains,
            (Some(Shape::Str), "starts_with") => RtOp::StrStartsWith,
            (Some(Shape::Str), "ends_with") => RtOp::StrEndsWith,
            (Some(Shape::Str), "repeat") => RtOp::StrRepeat,
            (Some(Shape::List(_)), "len") => RtOp::ListLen,
            (Some(Shape::List(_)), "is_empty") => RtOp::ListIsEmpty,
            (Some(Shape::List(item)), "push") => {
                let v = values.pop().expect("an item");
                values.push(self.coerce(v, item)?);
                RtOp::ListPush
            }
            (Some(Shape::List(_)), "get") => RtOp::ListGet,
            (Some(Shape::List(_)), "first") => RtOp::ListFirst,
            (Some(Shape::List(_)), "last") => RtOp::ListLast,
            (Some(Shape::List(_)), "contains") => RtOp::ListContains,
            (Some(Shape::Map(..)), "get") => RtOp::MapGet,
            (Some(Shape::Map(k, _)), "remove") => {
                let key = values.pop().expect("a key");
                values.push(self.coerce(key, k)?);
                RtOp::MapRemove
            }
            (Some(Shape::Map(..)), "contains_key") => RtOp::MapContains,
            (Some(Shape::Map(..)), "len") => RtOp::MapLen,
            (Some(Shape::Map(..)), "is_empty") => RtOp::MapIsEmpty,
            (Some(Shape::Map(..)), "keys") => RtOp::MapKeys,
            (Some(Shape::Map(..)), "values") => RtOp::MapValues,
            (Some(Shape::Shared(_)), "get") => RtOp::SharedGet,
            (Some(Shape::Shared(_)), "weak") => RtOp::WeakMake,
            (Some(Shape::Shared(_)), "shared") => RtOp::WeakShared,
            (Some(Shape::Channel(t)), "put") => {
                let value = values.pop().expect("a value");
                values.push(self.coerce(value, t)?);
                RtOp::ChanPut
            }
            (Some(Shape::Channel(t)), "try_put") => {
                let value = values.pop().expect("a value");
                values.push(self.coerce(value, t)?);
                RtOp::ChanTryPut
            }
            (Some(Shape::Channel(_)), "close") => RtOp::ChanClose,
            (Some(Shape::Str), "bytes") => RtOp::StrBytes,
            (Some(Shape::Bytes), "len") => RtOp::BytesLen,
            (Some(Shape::Socket), "accept") => RtOp::NetAccept,
            (Some(Shape::Socket), "address") => RtOp::NetAddress,
            (Some(Shape::Socket), "read") => RtOp::NetRead,
            (Some(Shape::Socket), "write") => RtOp::NetWrite,
            (Some(Shape::Socket), "close") => RtOp::NetClose,
            (Some(Shape::Socket), "protocol") => RtOp::NetProtocol,
            // A connection's address is the other side's.
            (Some(Shape::Socket), "peer") => RtOp::NetAddress,
            (Some(Shape::Socket), "client_cert") => RtOp::NetClientCert,
            (Some(Shape::Socket), "open_stream") => RtOp::NetOpenStream,
            (Some(Shape::Socket), "read_stream") => RtOp::NetReadStream,
            (Some(Shape::Socket), "write_stream") if values.len() == 4 => {
                // Three operands at most: the stream and `done` in one.
                let done = values.pop().expect("done");
                let data = values.pop().expect("data");
                let stream = values.pop().expect("stream");
                let int = |n: i128| ir::Expr {
                    kind: K::Int(n),
                    ty: Type::I64,
                    line,
                };
                let flag = ir::Expr {
                    kind: K::If(
                        Box::new(done),
                        crate::lower_expr::wrap(int(1), true),
                        crate::lower_expr::wrap(int(0), true),
                    ),
                    ty: Type::I64,
                    line,
                };
                let doubled = ir::Expr {
                    kind: K::Binary(ir::BinOp::Mul, Box::new(stream), Box::new(int(2))),
                    ty: Type::I64,
                    line,
                };
                values.push(ir::Expr {
                    kind: K::Binary(ir::BinOp::Add, Box::new(doubled), Box::new(flag)),
                    ty: Type::I64,
                    line,
                });
                values.push(data);
                RtOp::NetWriteStream
            }
            (Some(Shape::Socket), "send") => RtOp::NetSend,
            (Some(Shape::Socket), "receive") => RtOp::NetReceive,
            (Some(Shape::Bytes), "get") => RtOp::BytesGet,
            (Some(Shape::Bytes), "slice") => RtOp::BytesSlice,
            (Some(Shape::Bytes), "index_of") => RtOp::BytesIndexOf,
            (Some(Shape::Bytes), "text") => RtOp::BytesText,
            (Some(Shape::Bytes), "to_list") => RtOp::BytesToList,
            (Some(Shape::Bytes), "as_list") => RtOp::BytesAsList,
            (Some(Shape::Bytes), "crc32c") => RtOp::BytesCrc32c,
            (Some(Shape::Bytes), "xxhash64") => RtOp::BytesXxhash64,
            (Some(Shape::Channel(_)), "take") => RtOp::ChanTake,
            (None, "wrapping_add") if is_int => RtOp::WrappingAdd,
            (None, "wrapping_sub") if is_int => RtOp::WrappingSub,
            (None, "wrapping_mul") if is_int => RtOp::WrappingMul,
            (None, "checked_add") if is_int => RtOp::CheckedAdd,
            (None, "checked_sub") if is_int => RtOp::CheckedSub,
            (None, "checked_mul") if is_int => RtOp::CheckedMul,
            (None, "checked_div") if is_int => RtOp::CheckedDiv,
            (Some(Shape::Optional(_)), "unwrap") => {
                return Ok(ir::Expr {
                    kind: K::Unwrap(Box::new(values.remove(0))),
                    ty,
                    line,
                });
            }
            (Some(Shape::Optional(_)), "is_some" | "is_none") => {
                let present = ir::Expr {
                    kind: K::IsSome(Box::new(values.remove(0))),
                    ty: Type::Bool,
                    line,
                };
                if name.name == "is_some" {
                    return Ok(present);
                }
                return Ok(ir::Expr {
                    kind: K::Unary(UnOp::Not, Box::new(present)),
                    ty,
                    line,
                });
            }
            _ => {
                let unsupported = self.unsupported(format!("the method `{}`", name.name), span);
                return self
                    .more_method(&name.name, values, ty, line)
                    .unwrap_or(Err(unsupported));
            }
        };
        // A method that changes its receiver gives back the changed value,
        // which the caller stores (`lower_stmt.rs`).
        let ty = if op.changes_receiver() {
            values[0].ty
        } else {
            ty
        };
        Ok(ir::Expr {
            kind: K::Rt(op, values),
            ty,
            line,
        })
    }
}
