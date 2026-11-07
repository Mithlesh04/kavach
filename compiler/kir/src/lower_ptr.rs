//! Raw memory (`Ptr<T>`, spec 6.4) and text passed to C (10.7) in native
//! code. A `Ptr<T>` is an address (`I64`). `alloc` and `free` are C's
//! `calloc` and `free` (`runtime/native/src/rt_ptr.rs`); `p.read(i)` and
//! `p.write(i, v)` work out the address `p + i * size` here, and the
//! backends load or store there in one instruction.

use crate::ir::{self, Block, ExprKind as K, FastOp, FuncId, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;
use kavach_checker::Ty;

fn at(kind: K, ty: Type, line: u32) -> ir::Expr {
    ir::Expr { kind, ty, line }
}

/// A record's fields, each with its offset, as C lays them out.
pub(crate) type Fields = Option<Vec<(Type, i128)>>;

/// The bytes a value of `t` takes in memory, as C has it.
fn size_of(t: Type) -> Option<i128> {
    Some(match t {
        Type::I8 | Type::U8 | Type::Bool => 1,
        Type::I16 | Type::U16 => 2,
        Type::I32 | Type::U32 => 4,
        Type::I64 | Type::U64 | Type::F64 => 8,
        Type::F32 => 4,
        _ => return None,
    })
}

impl<'a> Lowerer<'a> {
    /// The type `Ptr<T>` points to, for the expression at `span`, with its
    /// size and, for a record, each field's type and offset.
    pub(crate) fn pointee(
        &mut self,
        ptr: Option<Ty>,
        span: ast::Span,
    ) -> Lowered<(Type, i128, Fields)> {
        let Some(Ty::Builtin("Ptr", args)) = ptr else {
            return Err(self.unsupported("this pointer", span));
        };
        let t = self.kir_type(&args[0], span)?;
        if let Some(size) = size_of(t) {
            return Ok((t, size, None));
        }
        let Some(Shape::Record { fields, .. }) = self.shapes.shape(t) else {
            return Err(self.unsupported("a `Ptr` to this type", span));
        };
        // As C lays out a struct (spec 6.4): each field aligned to its
        // size, the whole to its largest field.
        let (mut offset, mut align, mut out) = (0, 1, Vec::new());
        for f in fields {
            let Some(size) = size_of(f.ty) else {
                return Err(self.unsupported("a `Ptr` to a record with this field", span));
            };
            offset = (offset + size - 1) / size * size;
            out.push((f.ty, offset));
            offset += size;
            align = align.max(size);
        }
        Ok((t, (offset + align - 1) / align * align, Some(out)))
    }

    pub(crate) fn type_at(&self, span: ast::Span) -> Option<Ty> {
        self.c.expr_types.get(&(span.start, span.end)).cloned()
    }

    /// `alloc(n)` and `free(p)`.
    pub(crate) fn ptr_builtin(
        &mut self,
        name: &str,
        arg: &'a ast::Expr,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let value = self.expr(arg)?;
        if name == "free" {
            return Ok(at(K::Rt(RtOp::PtrFree, vec![value]), Type::Unit, line));
        }
        let (_, size, _) = self.pointee(self.type_at(span), span)?;
        let size = at(K::Int(size), Type::I64, line);
        Ok(at(
            K::Rt(RtOp::PtrAlloc, vec![value, size]),
            Type::I64,
            line,
        ))
    }

    /// `p.read(i)` and `p.write(i, v)`, or `None` when `base` isn't a
    /// pointer.
    pub(crate) fn ptr_method(
        &mut self,
        base: &'a ast::Expr,
        name: &str,
        args: &'a [ast::Arg],
        line: u32,
        span: ast::Span,
    ) -> Option<Lowered<ir::Expr>> {
        let ptr = self.type_at(base.span);
        if !matches!(ptr, Some(Ty::Builtin("Ptr", _))) {
            return None;
        }
        Some(self.ptr_access(ptr, base, name, args, line, span))
    }

    fn ptr_access(
        &mut self,
        ptr: Option<Ty>,
        base: &'a ast::Expr,
        name: &str,
        args: &'a [ast::Arg],
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let (t, size, fields) = self.pointee(ptr, span)?;
        if let ("text", []) = (name, args) {
            let text = self.str_type();
            let ty = self.optional_type(text);
            return Ok(at(
                K::Rt(RtOp::PtrReadText, vec![self.expr(base)?]),
                ty,
                line,
            ));
        }
        let (p, i) = match args.first() {
            Some(i) => (self.expr(base)?, self.expr(&i.value)?),
            None => return Err(self.unsupported(format!("`.{name}()`"), span)),
        };
        // No checks, as in C (spec 6.4): the address wraps like C's.
        let size = at(K::Int(size), Type::I64, line);
        let offset = at(
            K::Fast(FastOp::Mul, Box::new(i), Box::new(size)),
            Type::I64,
            line,
        );
        let addr = at(
            K::Fast(FastOp::Add, Box::new(p), Box::new(offset)),
            Type::I64,
            line,
        );
        if let Some(fields) = fields {
            let write = match (name, args) {
                ("read", [_]) => None,
                ("write", [_, v]) => Some(self.expr(&v.value)?),
                _ => return Err(self.unsupported(format!("`.{name}` of a `Ptr`"), span)),
            };
            return self.record_access(t, &fields, addr, write, line);
        }
        match (name, args) {
            ("read", [_]) => Ok(at(K::Rt(RtOp::PtrRead, vec![addr]), t, line)),
            ("write", [_, v]) => {
                let v = self.expr(&v.value)?;
                Ok(at(K::Rt(RtOp::PtrWrite, vec![addr, v]), Type::Unit, line))
            }
            _ => Err(self.unsupported(format!("`.{name}` of a `Ptr`"), span)),
        }
    }

    /// `read` and `write` of the record of type `t` at `addr`, field by
    /// field; a record read is checked against its invariants (spec 6.4).
    /// `write` is the record to store; `None` reads one.
    pub(crate) fn record_access(
        &mut self,
        t: Type,
        fields: &[(Type, i128)],
        addr: ir::Expr,
        write: Option<ir::Expr>,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let a = self.new_local(Type::I64);
        let mut stmts = vec![Stmt::Let(a, addr)];
        let field_at = |l: &Self, offset: i128| {
            let offset = Box::new(at(K::Int(offset), Type::I64, line));
            let base = Box::new(l.local_expr(a, line));
            at(K::Fast(FastOp::Add, base, offset), Type::I64, line)
        };
        match write {
            None => {
                let values = fields
                    .iter()
                    .map(|(ft, offset)| {
                        at(
                            K::Rt(RtOp::PtrRead, vec![field_at(self, *offset)]),
                            *ft,
                            line,
                        )
                    })
                    .collect();
                let record = at(
                    K::Record {
                        variant: 0,
                        fields: values,
                    },
                    t,
                    line,
                );
                let value = Some(Box::new(self.with_invariants(record)?));
                Ok(at(K::Block(Block { stmts, value }), t, line))
            }
            Some(v) => {
                let r = self.new_local(t);
                stmts.push(Stmt::Let(r, v));
                for (k, (ft, offset)) in fields.iter().enumerate() {
                    let base = Box::new(self.local_expr(r, line));
                    let field = at(K::GetField(base, k as u32), *ft, line);
                    let store = K::Rt(RtOp::PtrWrite, vec![field_at(self, *offset), field]);
                    stmts.push(Stmt::Expr(at(store, Type::Unit, line)));
                }
                Ok(at(K::Block(Block { stmts, value: None }), Type::Unit, line))
            }
        }
    }

    /// A call of C function `fid` (spec 10.7): each text argument goes as
    /// a copy with a zero byte after it, freed when the call returns, and
    /// each callback as the address of the Kavach function, which C calls
    /// directly (an `env` of `Unit`: no closure object).
    pub(crate) fn c_call(
        &mut self,
        fid: FuncId,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let decl = self.decl_of(fid);
        let mut values = Vec::new();
        for (p, a) in decl.params.iter().zip(args) {
            let callback =
                p.ty.as_ref()
                    .is_some_and(|t| matches!(t.kind, ast::TypeKind::Fn { .. }));
            let named = match &a.value.kind {
                ast::ExprKind::Name(id) => self.c.resolution.uses.get(&id.span.start),
                _ => None,
            };
            match (callback, named.and_then(|d| self.fn_ids.get(d))) {
                (true, Some(&func)) => {
                    let captures = Vec::new();
                    let address = K::Closure {
                        func,
                        env: Type::Unit,
                        captures,
                    };
                    values.push(at(address, Type::I64, line));
                }
                (true, None) => return Err(self.unsupported("this callback", a.value.span)),
                (false, _) => values.push(self.expr(&a.value)?),
            }
        }
        let text = self.str_type();
        if values.iter().all(|v| v.ty != text) {
            return Ok(at(K::Call(fid, values), ty, line));
        }
        let (mut stmts, mut copies, mut args) = (Vec::new(), Vec::new(), Vec::new());
        for v in values {
            if v.ty != text {
                args.push(v);
                continue;
            }
            let copy = self.new_local(Type::I64);
            stmts.push(Stmt::Let(
                copy,
                at(K::Rt(RtOp::PtrText, vec![v]), Type::I64, line),
            ));
            args.push(self.local_expr(copy, line));
            copies.push(copy);
        }
        let result = self.new_local(ty);
        stmts.push(Stmt::Let(result, at(K::Call(fid, args), ty, line)));
        for c in copies {
            let free = K::Rt(RtOp::PtrFree, vec![self.local_expr(c, line)]);
            stmts.push(Stmt::Expr(at(free, Type::Unit, line)));
        }
        let value = (ty != Type::Unit).then(|| Box::new(self.local_expr(result, line)));
        Ok(at(K::Block(Block { stmts, value }), ty, line))
    }
}
