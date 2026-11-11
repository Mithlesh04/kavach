//! Lowering calls: functions, built-ins, and methods of records, text and
//! lists.

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use crate::types::prim_type;
use kavach_ast::{self as ast, ExprKind, StrPart};
use kavach_checker::{Prim, Ty};
use kavach_resolver::DefKind;

impl<'a> Lowerer<'a> {
    pub(crate) fn call(
        &mut self,
        e: &'a ast::Expr,
        callee: &'a ast::Expr,
        args: &'a [ast::Arg],
        ty: Type,
    ) -> Lowered<ir::Expr> {
        if let Some(schema) = self.schema_call(e, callee, ty) {
            return schema;
        }
        let line = self.line(e.span);
        let res = &self.c.resolution;
        match &callee.kind {
            ExprKind::Name(id) => {
                let Some(&def) = res.uses.get(&id.span.start) else {
                    return Err(self.unsupported("this call", e.span));
                };
                match res.defs[def].kind {
                    DefKind::Fn if self.generic_fns.contains_key(&def) => {
                        self.generic_call(def, e, args, ty, line)
                    }
                    DefKind::Fn => {
                        let fid = self.fn_ids[&def];
                        if self.decl_of(fid).c_library.is_some() {
                            return self.c_call(fid, args, ty, line);
                        }
                        let values = self.arguments(self.decl_of(fid), args, e.span)?;
                        let call = ir::Expr {
                            kind: K::Call(fid, values),
                            ty,
                            line,
                        };
                        self.write_back(fid, call, args, 0, ty)
                    }
                    DefKind::Type => self.construct(def, None, args, ty, line, e.span),
                    DefKind::Variant { of } => {
                        self.construct(of, Some(&id.name), args, ty, line, e.span)
                    }
                    DefKind::Builtin => self.builtin_call(&id.name, args, ty, line, e.span),
                    DefKind::Local { .. } | DefKind::Param => {
                        self.call_through(callee, args, e.span)
                    }
                    _ => Err(self.unsupported("calling a function value", e.span)),
                }
            }
            ExprKind::Field {
                base,
                name,
                optional: false,
            } => self.method_call(base, name, args, ty, line, e.span),
            _ => self.call_through(callee, args, e.span),
        }
    }

    /// Calls whatever function value `callee` gives (spec 3.4).
    fn call_through(
        &mut self,
        callee: &'a ast::Expr,
        args: &'a [ast::Arg],
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let f = self.expr(callee)?;
        let values = args
            .iter()
            .map(|a| self.expr(&a.value))
            .collect::<Lowered<Vec<_>>>()?;
        self.call_value(f, values, span)
    }

    /// Arguments in parameter order, converted to the parameter types.
    pub(crate) fn arguments(
        &mut self,
        f: &'a ast::FnDecl,
        args: &'a [ast::Arg],
        span: ast::Span,
    ) -> Lowered<Vec<ir::Expr>> {
        let values = self.argument_values(f, args, span)?;
        let params = f.params.iter().filter_map(|p| p.ty.as_ref());
        let mut out = Vec::new();
        for (value, ty) in values.into_iter().zip(params) {
            let want = self.written_type(ty)?;
            out.push(self.coerce(value, want)?);
        }
        Ok(out)
    }

    /// Arguments in parameter order: positional, named, then defaults.
    pub(crate) fn argument_values(
        &mut self,
        f: &'a ast::FnDecl,
        args: &'a [ast::Arg],
        span: ast::Span,
    ) -> Lowered<Vec<ir::Expr>> {
        let params: Vec<&ast::Param> = f.params.iter().filter(|p| p.ty.is_some()).collect();
        let mut values = Vec::new();
        for (i, p) in params.iter().enumerate() {
            let given = args
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|n| n.name == p.name.name))
                .or_else(|| args.get(i).filter(|a| a.name.is_none()));
            let value = match (given, &p.default) {
                (Some(a), _) => self.expr(&a.value)?,
                (None, Some(d)) => self.expr(d)?,
                (None, None) => return Err(self.unsupported("this call", span)),
            };
            values.push(value);
        }
        Ok(values)
    }

    fn builtin_call(
        &mut self,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        if let ("log_write", [level, text, event]) = (name, args) {
            return self.log_write_call(level, text, event, line);
        }
        if name == "random_bits" {
            return Ok(ir::Expr {
                kind: K::Rt(RtOp::RandomBits, Vec::new()),
                ty,
                line,
            });
        }
        let arg = args
            .first()
            .map(|a| &a.value)
            .ok_or_else(|| self.unsupported(format!("`{name}()`"), span))?;
        let kind = match name {
            "panic" => match &arg.kind {
                ExprKind::Str(parts) if parts.iter().all(|p| matches!(p, StrPart::Text(_))) => {
                    K::Panic(
                        parts
                            .iter()
                            .map(|p| {
                                if let StrPart::Text(t) = p {
                                    t.as_str()
                                } else {
                                    ""
                                }
                            })
                            .collect(),
                    )
                }
                _ => K::Rt(RtOp::PanicText, vec![self.expr(arg)?]),
            },
            "alloc" | "free" => return self.ptr_builtin(name, arg, line, span),
            "Shared" => K::Rt(RtOp::SharedNew, vec![self.expr(arg)?]),
            "Channel" => K::Rt(RtOp::ChanNew, vec![self.expr(arg)?]),
            "Bytes" => K::Rt(RtOp::BytesNew, vec![self.expr(arg)?]),
            "env" => K::Rt(RtOp::EnvGet, vec![self.expr(arg)?]),
            // `Error("disk full")`: an error that is only its message (spec 5.5).
            "Error" => {
                let message = self.expr(arg)?;
                return self.coerce(message, ty);
            }
            // `reveal` only lets a secret through the compiler's checks
            // (spec 8.7); at run time it is the value itself.
            "reveal" => return self.expr(arg),
            "ask" => return self.ask_call(span, arg, line),
            "sum" => match self.rule_sum(arg, ty, line)? {
                Some(total) => return Ok(total),
                None => return Err(self.unsupported("`sum` outside a `limit` rule", span)),
            },
            // `String(x)` (spec 2.1): its text, as `"{x}"` gives it.
            "String" => K::Format(vec![self.expr(arg)?]),
            // `Int(text)` (spec 2.1): read, so it can fail.
            _ if name != "String" && (Prim::from_name(name).is_some()) && self.is_text(arg) => {
                return self.convert_text(arg, ty, line, span);
            }
            _ if Prim::from_name(name).and_then(prim_type).is_some()
                || matches!(name, "Decimal" | "BigInt") =>
            {
                K::Convert(Box::new(self.expr(arg)?), true)
            }
            _ => return Err(self.unsupported(format!("`{name}`"), span)),
        };
        Ok(ir::Expr { kind, ty, line })
    }

    pub(crate) fn is_mutating_call(&self, callee: &ast::Expr) -> bool {
        let ExprKind::Field { base, name, .. } = &callee.kind else {
            return false;
        };
        matches!(
            (
                name.name.as_str(),
                self.c.expr_types.get(&(base.span.start, base.span.end))
            ),
            ("push", Some(Ty::List(_)))
                | ("remove", Some(Ty::Map(..) | Ty::Set(_)))
                | ("add", Some(Ty::Set(_)))
        )
    }
}

/// The name `extend` uses for a scalar type.
pub(crate) fn scalar_name(t: Type) -> &'static str {
    match t {
        Type::F64 => "Float",
        Type::F32 => "Float32",
        Type::Bool => "Bool",
        Type::Char => "Char",
        t => t.kavach_name(),
    }
}
