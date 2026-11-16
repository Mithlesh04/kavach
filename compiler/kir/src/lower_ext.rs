//! The type a method's `self` has, and methods added to lists, maps, sets
//! and rings with `extend<T> [T]` (spec 2.5). Those are made once per item
//! type when called, as generic functions are (`lower_generic.rs`).

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::{Lowered, Lowerer, Owner};
use crate::types::prim_type;
use kavach_ast::{self as ast, TypeKind};
use kavach_checker::Ty;
use kavach_resolver::DefKind;
use std::collections::HashMap;

/// Methods added to collections, found by the kind of the receiver.
#[derive(Default)]
pub(crate) struct CollectionMethods<'a> {
    /// By kind (`List`, `Map`, `Set`, `Ring`) and name: the extend that
    /// adds the method, and the method.
    by_name: HashMap<(&'static str, String), (&'a ast::ExtendDecl, &'a ast::FnDecl)>,
    /// By where a method's name starts: the type its `self` is written as.
    selves: HashMap<u32, &'a ast::TypeExpr>,
}

impl<'a> Lowerer<'a> {
    /// The type a method's `self` has.
    pub(crate) fn owner_type(&mut self, owner: &Owner, span: ast::Span) -> Lowered<Type> {
        match owner {
            Owner::Adt(def) => {
                // In an instance of a generic type's method, `self` has
                // the instance's type arguments.
                let args = self
                    .type_generics(*def)
                    .iter()
                    .map(|g| self.subst.get(g).copied())
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| self.unsupported("this method", span))?;
                self.adt_instance(*def, args, span)
            }
            Owner::Builtin(name) if name == "String" => Ok(self.str_type()),
            Owner::Builtin(name) => kavach_checker::Prim::from_name(name)
                .and_then(prim_type)
                .ok_or_else(|| self.unsupported(format!("methods on `{name}`"), span)),
        }
    }

    /// Records the methods of `e` when it extends a list, map, set or
    /// ring; false for any other extend.
    pub(crate) fn add_collection_methods(&mut self, e: &'a ast::ExtendDecl) -> bool {
        let kind = match &e.target.kind {
            TypeKind::List(_) => "List",
            TypeKind::Map(..) => "Map",
            TypeKind::Named { path, .. } if path.segments.len() == 1 => {
                let first = &path.segments[0];
                let res = &self.c.resolution;
                let builtin = res
                    .uses
                    .get(&first.span.start)
                    .is_some_and(|d| res.defs[*d].kind == DefKind::Builtin);
                match first.name.as_str() {
                    "List" if builtin => "List",
                    "Map" if builtin => "Map",
                    "Set" if builtin => "Set",
                    "Ring" if builtin => "Ring",
                    _ => return false,
                }
            }
            _ => return false,
        };
        for m in &e.methods {
            if let ast::ItemKind::Fn(f) = &m.kind {
                self.ext.by_name.insert((kind, f.name.name.clone()), (e, f));
                self.ext.selves.insert(f.name.span.start, &e.target);
            }
        }
        true
    }

    /// The type `self` is written as in a method added to a collection.
    pub(crate) fn collection_self(&self, f: &ast::FnDecl) -> Option<&'a ast::TypeExpr> {
        self.ext.selves.get(&f.name.span.start).copied()
    }

    /// Method `name` added to the kind of collection `base` is.
    pub(crate) fn collection_method(
        &self,
        base: &ast::Expr,
        name: &str,
    ) -> Option<(&'a ast::ExtendDecl, &'a ast::FnDecl)> {
        let kind = match self.c.expr_types.get(&(base.span.start, base.span.end))? {
            Ty::List(_) => "List",
            Ty::Map(..) => "Map",
            Ty::Set(_) => "Set",
            Ty::Builtin("Ring", _) => "Ring",
            _ => return None,
        };
        self.ext.by_name.get(&(kind, name.to_string())).copied()
    }

    /// The call `call` of method `f`, added by `e`, on `receiver`: the
    /// copy made for the item types the receiver has.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn collection_method_call(
        &mut self,
        (e, f): (&'a ast::ExtendDecl, &'a ast::FnDecl),
        receiver: ir::Expr,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        call: &'a ast::Expr,
    ) -> Lowered<ir::Expr> {
        let span = call.span;
        let mut subst = HashMap::new();
        self.infer_args(&e.target, receiver.ty, &mut subst);
        if let ast::ExprKind::Call { generics, .. } = &call.kind {
            for (g, written) in f.generics.iter().zip(generics) {
                if let Some(&p) = self.decl.get(&g.name.span.start) {
                    subst.insert(p, self.written_type(written)?);
                }
            }
        }
        let values = self.argument_values(f, args, span)?;
        let params = f.params.iter().filter_map(|p| p.ty.as_ref());
        for (value, written) in values.iter().zip(params.clone()) {
            self.infer_args(written, value.ty, &mut subst);
        }
        if let Some(ret) = &f.ret {
            self.infer_args(ret, ty, &mut subst);
        }
        let mut type_args = Vec::new();
        for g in e.generics.iter().chain(&f.generics) {
            let p = self.decl.get(&g.name.span.start);
            match p.and_then(|p| subst.get(p)) {
                Some(t) => type_args.push(*t),
                None => return Err(self.unsupported("this method call", span)),
            }
        }
        // The extend's first name tells its methods apart from another
        // extend's of the same name.
        let first = e.generics.first();
        let Some(&def) = first.and_then(|g| self.decl.get(&g.name.span.start)) else {
            return Err(self.unsupported("this method call", span));
        };
        let outer = std::mem::replace(&mut self.subst, subst.clone());
        let wants = params
            .map(|t| self.written_type(t))
            .collect::<Lowered<Vec<_>>>();
        self.subst = outer;
        let mut all = vec![receiver];
        for (v, want) in values.into_iter().zip(wants?) {
            all.push(self.coerce(v, want)?);
        }
        let fid = self.instance(def, f, type_args, subst, None);
        let call = ir::Expr {
            kind: K::Call(fid, all),
            ty,
            line,
        };
        self.write_back(fid, call, args, 1, ty)
    }
}
