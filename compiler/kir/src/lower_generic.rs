//! Generic functions (spec 2.9), compiled once per set of type arguments
//! (monomorphization): `first([3, 4])` and `first(["a"])` call two
//! functions, one for `Int` and one for `String`. Each copy is as fast as
//! a function written for that type.

use crate::ir::{self, ExprKind as K, FuncId, Shape, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast::{self as ast, TypeKind};
use kavach_resolver::{DefId, DefKind};
use std::collections::HashMap;

impl<'a> Lowerer<'a> {
    /// The call `e` of generic function `def`, whose result has type `ty`.
    pub(crate) fn generic_call(
        &mut self,
        def: DefId,
        e: &'a ast::Expr,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let span = e.span;
        let f = self.generic_fns[&def];
        let values = self.argument_values(f, args, span)?;
        // The type arguments written at the call (`read<Point>(…)`), then
        // the ones the argument and result types show.
        let mut subst = HashMap::new();
        if let ast::ExprKind::Call { generics, .. } = &e.kind {
            for (g, written) in f.generics.iter().zip(generics) {
                if let Some(&p) = self.decl.get(&g.name.span.start) {
                    subst.insert(p, self.written_type(written)?);
                }
            }
        }
        let params = f.params.iter().filter_map(|p| p.ty.as_ref());
        for (value, written) in values.iter().zip(params.clone()) {
            self.infer_args(written, value.ty, &mut subst);
        }
        if let Some(ret) = &f.ret {
            self.infer_args(ret, ty, &mut subst);
        }
        let mut type_args = Vec::new();
        for g in &f.generics {
            let p = self.decl.get(&g.name.span.start).copied();
            match p.and_then(|p| subst.get(&p)) {
                Some(t) => type_args.push(*t),
                // `! E` where no function given here can fail (spec 3.4):
                // the error is never made, so any type stands in for it.
                None if p.is_some() && self.names_an_error(f, &g.name.name) => {
                    subst.insert(p.expect("checked above"), Type::I64);
                    type_args.push(Type::I64);
                }
                None => return Err(self.unsupported("this generic call", span)),
            }
        }
        // The parameter types of this instance, to convert the arguments.
        let outer = std::mem::replace(&mut self.subst, subst.clone());
        let wants = params
            .map(|t| self.written_type(t))
            .collect::<Lowered<Vec<_>>>();
        self.subst = outer;
        let values = values
            .into_iter()
            .zip(wants?)
            .map(|(v, want)| self.coerce(v, want))
            .collect::<Lowered<Vec<_>>>()?;
        let fid = self.instance(def, f, type_args, subst, None);
        let call = ir::Expr {
            kind: K::Call(fid, values),
            ty,
            line,
        };
        // An instance that can't fail here still returns an error.
        if f.errors.is_some() && !self.can_fail(e) {
            // Its error is the stand-in above.
            let err = self.held_error(Type::I64);
            let (value, error) = (self.new_local(ty), self.new_local(err));
            let at = |kind| ir::Expr { kind, ty, line };
            return Ok(at(K::Block(ir::Block {
                stmts: vec![ir::Stmt::LetResult { value, error, call }],
                value: (ty != Type::Unit).then(|| Box::new(self.local_expr(value, line))),
            })));
        }
        self.write_back(fid, call, args, 0, ty)
    }

    /// Whether type parameter `name` of `f` is an error type: in its
    /// `! E`, or in a function type's among its parameters. Left unknown
    /// by the arguments, it is in no other place.
    fn names_an_error(&self, f: &ast::FnDecl, name: &str) -> bool {
        let after_bang = |text: &str| {
            text.split('!').skip(1).any(|e| {
                e.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|w| w == name)
            })
        };
        f.errors
            .as_ref()
            .is_some_and(|t| self.text(t.span).trim() == name)
            || f.params
                .iter()
                .filter_map(|p| p.ty.as_ref())
                .any(|t| after_bang(self.text(t.span)))
    }

    /// Generic function `def` used as a value of function type `ty`
    /// (`const f = first`): the copy for the types `ty` shows.
    pub(crate) fn generic_fn_value(
        &mut self,
        def: DefId,
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let f = self.generic_fns[&def];
        let Some(Shape::Func(params, ret, _)) = self.shapes.shape(ty).cloned() else {
            return Err(self.unsupported("generic functions as values", span));
        };
        let mut subst = HashMap::new();
        let written = f.params.iter().filter_map(|p| p.ty.as_ref());
        for (w, t) in written.zip(params) {
            self.infer_args(w, t, &mut subst);
        }
        if let Some(r) = &f.ret {
            self.infer_args(r, ret, &mut subst);
        }
        let mut type_args = Vec::new();
        for g in &f.generics {
            let found = self.decl.get(&g.name.span.start).and_then(|p| subst.get(p));
            match found {
                Some(t) => type_args.push(*t),
                None => return Err(self.unsupported("generic functions as values", span)),
            }
        }
        let fid = self.instance(def, f, type_args, subst, None);
        self.function_value(fid, ty, span)
    }

    /// The generic parameters of a record or union declared in this file.
    pub(crate) fn type_generics(&self, def: DefId) -> Vec<DefId> {
        let span = self.c.resolution.defs[def].span;
        self.c
            .file
            .items
            .iter()
            .find_map(|item| match &item.kind {
                ast::ItemKind::Type(t) if Some(t.name.span) == span => Some(t),
                _ => None,
            })
            .map(|t| {
                t.generics
                    .iter()
                    .filter_map(|g| self.decl.get(&g.name.span.start).copied())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A method `f` of generic type `owner`, called on `receiver` (or
    /// through the type, without one).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn generic_method_call(
        &mut self,
        owner: DefId,
        f: &'a ast::FnDecl,
        receiver: Option<ir::Expr>,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let type_params = self.type_generics(owner);
        let mut subst = HashMap::new();
        if let Some(r) = &receiver {
            let args_of = self.shapes.args_of(r.ty).unwrap_or_default().to_vec();
            subst.extend(type_params.iter().copied().zip(args_of));
        }
        let values = self.argument_values(f, args, span)?;
        let params = f.params.iter().filter_map(|p| p.ty.as_ref());
        for (value, written) in values.iter().zip(params.clone()) {
            self.infer_args(written, value.ty, &mut subst);
        }
        if let Some(ret) = &f.ret {
            self.infer_args(ret, ty, &mut subst);
        }
        let own = f
            .generics
            .iter()
            .filter_map(|g| self.decl.get(&g.name.span.start).copied());
        let mut type_args = Vec::new();
        for g in type_params.iter().copied().chain(own) {
            match subst.get(&g) {
                Some(t) => type_args.push(*t),
                None => return Err(self.unsupported("this generic method call", span)),
            }
        }
        let outer = std::mem::replace(&mut self.subst, subst.clone());
        let wants = params
            .map(|t| self.written_type(t))
            .collect::<Lowered<Vec<_>>>();
        self.subst = outer;
        let receiver_given = receiver.is_some();
        let mut all: Vec<ir::Expr> = receiver.into_iter().collect();
        for (v, want) in values.into_iter().zip(wants?) {
            all.push(self.coerce(v, want)?);
        }
        let fid = self.instance(
            owner,
            f,
            type_args,
            subst,
            Some(crate::lower::Owner::Adt(owner)),
        );
        let call = ir::Expr {
            kind: K::Call(fid, all),
            ty,
            line,
        };
        // The receiver, when there is one, comes before the arguments.
        let offset = usize::from(receiver_given);
        self.write_back(fid, call, args, offset, ty)
    }

    /// Method `name` of generic type `def`, made for the type arguments of
    /// `concrete`, for an interface value's box (`lower_iface.rs`). `None`
    /// when the method has type parameters of its own.
    pub(crate) fn generic_method_for(
        &mut self,
        def: DefId,
        concrete: Type,
        name: &str,
    ) -> Option<FuncId> {
        let f = self
            .generic_methods
            .get(&(def, name.to_string()))
            .copied()?;
        if !f.generics.is_empty() {
            return None;
        }
        let params = self.type_generics(def);
        let args = self.shapes.args_of(concrete)?.to_vec();
        if params.len() != args.len() {
            return None;
        }
        let subst = params.iter().copied().zip(args.iter().copied()).collect();
        let owner = Some(crate::lower::Owner::Adt(def));
        Some(self.instance(def, f, args, subst, owner))
    }

    /// The function for `def` with these type arguments, made on first use.
    fn instance(
        &mut self,
        def: DefId,
        f: &'a ast::FnDecl,
        type_args: Vec<Type>,
        subst: HashMap<DefId, Type>,
        owner: Option<crate::lower::Owner>,
    ) -> FuncId {
        let key = (def, f.name.name.clone(), type_args);
        if let Some(fid) = self.instances.get(&key) {
            return *fid;
        }
        let fid = self.fns.len() as FuncId;
        self.fns.push((Some(f), owner));
        self.fn_subst.insert(fid, subst);
        self.instances.insert(key, fid);
        fid
    }

    /// Matches written type `written` against actual type `ty`, recording
    /// what each generic parameter stands for.
    fn infer_args(&self, written: &ast::TypeExpr, ty: Type, out: &mut HashMap<DefId, Type>) {
        let shape = self.shapes.shape(ty).cloned();
        match (&written.kind, shape) {
            (TypeKind::Named { path, args }, shape) => {
                let first = &path.segments[0];
                let Some(&d) = self.c.resolution.uses.get(&first.span.start) else {
                    return;
                };
                let inner: Vec<Type> = match (self.c.resolution.defs[d].kind, shape) {
                    (DefKind::GenericParam, _) => {
                        out.entry(d).or_insert(ty);
                        return;
                    }
                    (DefKind::Builtin, Some(Shape::List(t) | Shape::Shared(t))) => vec![t],
                    (DefKind::Builtin, Some(Shape::Map(k, v))) => vec![k, v],
                    (DefKind::Type, _) => self.shapes.args_of(ty).unwrap_or_default().to_vec(),
                    _ => return,
                };
                for (a, t) in args.iter().zip(inner) {
                    self.infer_args(a, t, out);
                }
            }
            (TypeKind::List(item), Some(Shape::List(t))) => self.infer_args(item, t, out),
            (TypeKind::Optional(inner), Some(Shape::Optional(t))) => self.infer_args(inner, t, out),
            // A `T` passed where `T?` is expected.
            (TypeKind::Optional(inner), _) => self.infer_args(inner, ty, out),
            (TypeKind::Map(k, v), Some(Shape::Map(kt, vt))) => {
                self.infer_args(k, kt, out);
                self.infer_args(v, vt, out);
            }
            (
                TypeKind::Fn {
                    params,
                    ret,
                    errors,
                    ..
                },
                Some(Shape::Func(ps, r, err)),
            ) => {
                for (a, t) in params.iter().zip(ps) {
                    self.infer_args(a, t, out);
                }
                if let (Some(e), Some(t)) = (errors, err) {
                    self.infer_args(e, t, out);
                }
                if let Some(ret) = ret {
                    self.infer_args(ret, r, out);
                }
            }
            (TypeKind::Tuple(items), Some(Shape::Tuple(ts))) => {
                for (a, t) in items.iter().zip(ts) {
                    self.infer_args(a, t, out);
                }
            }
            _ => {}
        }
    }
}
