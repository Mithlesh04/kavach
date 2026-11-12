//! Interface values (spec 2.15, decision 0029): a concrete value wrapped
//! into a box of `[value, its type code, one closure per method]`, and
//! method calls through the box. The closures reuse the machinery of
//! function values (`lower_closure.rs`), so both backends already know
//! how to build and call them. A failing method's closure fails too: a
//! call through it is a `LetResult`, whose backends then expect the
//! value and the error a failing function gives.

use crate::Unsupported;
use crate::ir::{self, ExprKind as K, FuncId, Shape, Stmt, Type, TypeId, type_code};
use crate::lower::{Lowered, Lowerer, Owner};
use kavach_ast::{ItemKind, Span};
use kavach_resolver::DefId;

impl Lowerer<'_> {
    /// `expr` (a concrete value) wrapped as interface type `want`.
    pub(crate) fn iface_value(&mut self, expr: ir::Expr, want: Type) -> Lowered<ir::Expr> {
        let Some(Shape::Iface { methods, .. }) = self.shapes.shape(want).cloned() else {
            return Ok(expr);
        };
        let line = expr.line;
        let concrete = expr.ty;
        let cant = |f: &str| Unsupported {
            feature: f.to_string(),
            line,
        };
        // Whose methods the box holds: a declared type's, or a built-in
        // type's from `extend Int: Describe` (spec 2.15).
        let def = self.shapes.def_of(concrete);
        let owner = match (def, self.shapes.shape(concrete)) {
            (Some(def), _) => Owner::Adt(def),
            (None, Some(Shape::Str)) => Owner::Builtin("String".into()),
            (None, None) if concrete != Type::Unit => {
                Owner::Builtin(crate::lower_calls::scalar_name(concrete).into())
            }
            _ => return Err(cant("an interface value of this type")),
        };
        // The value goes into a local once: the box's first slot and
        // every method closure hold it.
        let v = self.new_local(concrete);
        let mut fields = vec![
            self.local_expr(v, line),
            ir::Expr {
                kind: K::Int(type_code(concrete) as i128),
                ty: Type::I64,
                line,
            },
        ];
        let env_ty = self.shapes_intern(Shape::Closure(vec![concrete]));
        for (name, fty) in &methods {
            let fid = match (self.method_ids.get(&(owner.clone(), name.clone())), def) {
                (Some(&fid), _) => fid,
                (None, Some(def)) => self
                    .generic_method_for(def, concrete, name)
                    .ok_or_else(|| cant("this method of a generic type in an interface value"))?,
                (None, None) => return Err(cant("this method in an interface value")),
            };
            let shim = self.method_shim(fid, env_ty, concrete, *fty)?;
            fields.push(ir::Expr {
                kind: K::Closure {
                    func: shim,
                    env: env_ty,
                    captures: vec![self.local_expr(v, line)],
                },
                ty: *fty,
                line,
            });
        }
        Ok(ir::Expr {
            kind: K::Block(ir::Block {
                stmts: vec![Stmt::Let(v, expr)],
                value: Some(Box::new(ir::Expr {
                    kind: K::Record { variant: 0, fields },
                    ty: want,
                    line,
                })),
            }),
            ty: want,
            line,
        })
    }

    /// The error type of method `fid`, or `None` when it can't fail.
    fn error_type_of(&mut self, fid: FuncId) -> Lowered<Option<Type>> {
        let errors = self.fns.get(fid as usize).and_then(|(decl, _)| *decl);
        match errors.and_then(|f| f.errors.as_ref()) {
            Some(t) => {
                let own = self.written_type(t)?;
                Ok(Some(self.held_error(own)))
            }
            None => Ok(None),
        }
    }

    /// A small function `(closure, params…) -> ret` that reads the
    /// receiver out of the closure and calls method `fid` on it.
    fn method_shim(
        &mut self,
        fid: FuncId,
        env_ty: Type,
        concrete: Type,
        fty: Type,
    ) -> Lowered<FuncId> {
        if let Some(&shim) = self.iface_shims.get(&fid) {
            return Ok(shim);
        }
        let line = 0;
        let Some(Shape::Func(param_types, ret, _)) = self.shapes.shape(fty).cloned() else {
            return Err(Unsupported {
                feature: "this interface method".to_string(),
                line,
            });
        };
        let err = self.error_type_of(fid)?;
        let fails = err.is_some();
        let saved = self.save_function(ret);
        let env = self.new_local(env_ty);
        let receiver = self.new_local(concrete);
        let params: Vec<ir::Local> = param_types.iter().map(|t| self.new_local(*t)).collect();
        let read_self = ir::Expr {
            kind: K::GetField(Box::new(self.local_expr(env, line)), 1),
            ty: concrete,
            line,
        };
        let mut values = vec![self.local_expr(receiver, line)];
        values.extend(params.iter().map(|p| self.local_expr(*p, line)));
        let call = ir::Expr {
            kind: K::Call(fid, values),
            ty: ret,
            line,
        };
        let body = if let Some(err) = err {
            // The method's error is the shim's: passed on as `try` does.
            let (value, error) = (self.new_local(ret), self.new_local(err));
            let fail = ir::Expr {
                kind: K::Fail(Box::new(self.local_expr(error, line))),
                ty: Type::Unit,
                line,
            };
            let check = ir::Expr {
                kind: K::If(
                    Box::new(self.failed(error, line)),
                    ir::Block {
                        stmts: vec![Stmt::Expr(fail)],
                        value: None,
                    },
                    ir::Block::default(),
                ),
                ty: Type::Unit,
                line,
            };
            ir::Block {
                stmts: vec![
                    Stmt::Let(receiver, read_self),
                    Stmt::LetResult { value, error, call },
                    Stmt::Expr(check),
                ],
                value: (ret != Type::Unit).then(|| Box::new(self.local_expr(value, line))),
            }
        } else if ret != Type::Unit {
            ir::Block {
                stmts: vec![Stmt::Let(receiver, read_self)],
                value: Some(Box::new(call)),
            }
        } else {
            ir::Block {
                stmts: vec![Stmt::Let(receiver, read_self), Stmt::Expr(call)],
                value: None,
            }
        };
        let mut all_params = vec![env];
        all_params.extend(params);
        let f = ir::Function {
            name: "iface shim".into(),
            params: all_params,
            ret,
            locals: self.locals.clone(),
            body,
            line,
            fails,
            external: None,
        };
        self.restore_function(saved);
        let shim = self.add_function(f);
        self.iface_shims.insert(fid, shim);
        Ok(shim)
    }

    /// An interface as a value type (spec 2.15): its box shape, with
    /// one closure slot per method, in declared order. A generic one has
    /// a shape per set of type arguments, as a generic type does.
    pub(crate) fn iface_type(&mut self, def: DefId, args: Vec<Type>, span: Span) -> Lowered<Type> {
        if let Some(id) = self.shapes.adts.get(&(def, args.clone())) {
            return Ok(Type::Ref(*id));
        }
        let decl_span = self.c.resolution.defs[def].span;
        let decl = self.c.file.items.iter().find_map(|item| match &item.kind {
            ItemKind::Interface(i) if Some(i.name.span) == decl_span => Some(i),
            _ => None,
        });
        let Some(decl) = decl else {
            return Err(self.unsupported("this interface", span));
        };
        if decl.generics.len() != args.len() {
            return Err(self.unsupported("this generic interface", span));
        }
        let id = self.shapes.list.len() as TypeId;
        self.shapes.list.push(Shape::Str);
        self.shapes.adts.insert((def, args.clone()), id);
        // Method types see the interface's parameters as its arguments.
        let outer = self.subst.clone();
        for (g, t) in decl.generics.iter().zip(&args) {
            if let Some(&p) = self.decl.get(&g.name.span.start) {
                self.subst.insert(p, *t);
            }
        }
        let mut methods = Vec::new();
        for m in &decl.methods {
            let ItemKind::Fn(f) = &m.kind else { continue };
            let params = f
                .params
                .iter()
                .filter(|p| p.ty.is_some())
                .map(|p| self.written_type(p.ty.as_ref().unwrap()))
                .collect::<Lowered<Vec<_>>>()?;
            let ret = match &f.ret {
                Some(r) => self.written_type(r)?,
                None => Type::Unit,
            };
            // `inout` parameters come back with the value (spec 3.9).
            let inout: Vec<Type> = (f.params.iter().filter(|p| p.ty.is_some()))
                .zip(&params)
                .filter(|(p, _)| p.is_inout)
                .map(|(_, t)| *t)
                .collect();
            let ret = if inout.is_empty() {
                ret
            } else {
                self.inout_tuple(ret, &inout)
            };
            let fty = self.shapes.intern(Shape::Func(params, ret, None));
            methods.push((f.name.name.clone(), fty));
        }
        self.subst = outer;
        self.shapes.list[id as usize] = Shape::Iface {
            name: self.c.resolution.defs[def].name.clone(),
            methods,
        };
        Ok(Type::Ref(id))
    }
}
