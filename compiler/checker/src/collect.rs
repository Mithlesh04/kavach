//! Before checking bodies: turn written types into checker types, and
//! collect the signature of every function, type and method.

use crate::checker::{Checker, FieldInfo, FnSig, ParamInfo, Shape, TypeInfo, VariantInfo};
use crate::ty::{Prim, Ty, builtin_name};
use kavach_ast::*;
use kavach_resolver::DefKind;

impl Checker<'_> {
    /// A written type as a checker type. `self_ty` is what `Self` means.
    pub(crate) fn lower(&mut self, t: &TypeExpr, self_ty: Option<&Ty>) -> Ty {
        let boxed = |c: &mut Self, t: &TypeExpr| Box::new(c.lower(t, self_ty));
        match &t.kind {
            TypeKind::Named { path, args } => {
                let args: Vec<Ty> = args.iter().map(|a| self.lower(a, self_ty)).collect();
                // `regex.Regex`, `fs.FsError`: the type is the last name.
                let named = path.segments.last().unwrap_or(&path.segments[0]);
                let Some(&def) = self.res.uses.get(&named.span.start) else {
                    return Ty::Error;
                };
                if let Some(t) = self.aliases.get(&def) {
                    return t.clone();
                }
                let d = &self.res.defs[def];
                match d.kind {
                    DefKind::Type | DefKind::Interface | DefKind::Capability | DefKind::Agent => {
                        Ty::Adt(def, args)
                    }
                    DefKind::GenericParam => Ty::Param(def),
                    DefKind::Builtin => builtin_type(&d.name, args, self_ty),
                    _ => Ty::Error,
                }
            }
            TypeKind::List(t) => Ty::List(boxed(self, t)),
            TypeKind::Map(k, v) => Ty::Map(boxed(self, k), boxed(self, v)),
            TypeKind::Optional(t) => Ty::Optional(boxed(self, t)),
            TypeKind::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| self.lower(t, self_ty)).collect()),
            TypeKind::AnonRecord(fields) => {
                let mut fs: Vec<(String, Ty)> = Vec::new();
                for (name, ft) in fields {
                    let ty = self.lower(ft, self_ty);
                    if fs.iter().any(|(n, _)| *n == name.name) {
                        self.error(
                            name.span,
                            format!("the field `{}` is written twice", name.name),
                        );
                        continue;
                    }
                    fs.push((name.name.clone(), ty));
                }
                fs.sort_by(|a, b| a.0.cmp(&b.0));
                Ty::Anon(fs)
            }
            // A function that can fail returns a `Result` (spec 3.4).
            TypeKind::Fn {
                params,
                ret,
                errors,
                uses,
            } => {
                let r = ret.as_ref().map_or(Ty::Unit, |r| self.lower(r, self_ty));
                let r = match errors {
                    Some(e) => Ty::Result(Box::new(r), Box::new(self.lower(e, self_ty))),
                    None => r,
                };
                let limit = uses.as_ref().map(|paths| {
                    paths
                        .iter()
                        .map(|p| {
                            let names: Vec<&str> =
                                p.segments.iter().map(|s| s.name.as_str()).collect();
                            names.join(".")
                        })
                        .collect()
                });
                Ty::Fn(
                    params.iter().map(|t| self.lower(t, self_ty)).collect(),
                    Box::new(r),
                    limit,
                )
            }
            TypeKind::Error => Ty::Error,
        }
    }

    pub(crate) fn sig_of(&mut self, f: &FnDecl, self_ty: Option<&Ty>) -> FnSig {
        let mut generics = Vec::new();
        for g in &f.generics {
            let Some(&def) = self.decl.get(&g.name.span.start) else {
                continue;
            };
            let bounds: Vec<_> = g
                .bounds
                .iter()
                .filter_map(|b| match self.lower(b, self_ty) {
                    Ty::Adt(d, _) => Some(d),
                    _ => None,
                })
                .collect();
            self.bounds.insert(def, bounds.clone());
            generics.push((def, bounds));
        }
        let params = f
            .params
            .iter()
            .filter(|p| p.name.name != "self")
            .map(|p| ParamInfo {
                name: p.name.name.clone(),
                ty: p.ty.as_ref().map_or(Ty::Error, |t| self.lower(t, self_ty)),
                has_default: p.default.is_some(),
                secret: p.is_secret,
                inout: p.is_inout,
            })
            .collect();
        FnSig {
            generics,
            params,
            ret: f.ret.as_ref().map_or(Ty::Unit, |t| self.lower(t, self_ty)),
            ret_secret: f.ret_secret,
            errors: f.errors.as_ref().map(|t| self.lower(t, self_ty)),
            uses: f
                .uses
                .iter()
                .map(|p| {
                    p.segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(".")
                })
                .collect(),
            has_self: f.params.iter().any(|p| p.name.name == "self"),
            def: self.decl.get(&f.name.span.start).copied(),
        }
    }

    /// Notes the methods among `methods` that aren't `pub` (spec 10.1).
    fn note_private(&mut self, methods: &[Item]) {
        for m in methods.iter().filter(|m| !m.is_pub) {
            if let ItemKind::Fn(f) = &m.kind
                && let Some(&def) = self.decl.get(&f.name.span.start)
            {
                self.private_methods.insert(def);
            }
        }
    }

    fn method_sigs(&mut self, methods: &[Item], self_ty: Option<&Ty>) -> Vec<(String, FnSig)> {
        methods
            .iter()
            .filter_map(|m| match &m.kind {
                ItemKind::Fn(f) => Some((f.name.name.clone(), self.sig_of(f, self_ty))),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn collect(&mut self, file: &File) {
        // Aliases first, so a signature below can use one (spec 2.14).
        for item in &file.items {
            if let ItemKind::Type(t) = &item.kind
                && let TypeBody::Alias(ty) = &t.body
                && let Some(&def) = self.decl.get(&t.name.span.start)
            {
                let lowered = self.lower(ty, None);
                self.aliases.insert(def, lowered);
            }
        }
        for item in &file.items {
            match &item.kind {
                ItemKind::Type(t) => self.collect_type(t),
                ItemKind::Literal(l) => self.collect_literal(l),
                ItemKind::Interface(i) => {
                    let Some(&def) = self.decl.get(&i.name.span.start) else {
                        continue;
                    };
                    let sigs = self.method_sigs(&i.methods, Some(&Ty::Param(def)));
                    self.methods.entry(def).or_default().extend(sigs);
                    let generics = i
                        .generics
                        .iter()
                        .filter_map(|g| self.decl.get(&g.name.span.start).copied())
                        .collect();
                    self.iface_generics.insert(def, generics);
                }
                ItemKind::Capability(c) => {
                    let Some(&def) = self.decl.get(&c.name.span.start) else {
                        continue;
                    };
                    let mut sigs = self.method_sigs(&c.methods, None);
                    for (_, sig) in &mut sigs {
                        sig.uses = vec![c.name.name.clone()];
                    }
                    self.methods.entry(def).or_default().extend(sigs);
                }
                ItemKind::Agent(a) => {
                    let Some(&def) = self.decl.get(&a.name.span.start) else {
                        continue;
                    };
                    self.collect_agent(def, &a.members);
                    for member in &a.members {
                        if let AgentMember::On(f) = member {
                            let mut sig = self.sig_of(f, None);
                            sig.has_self = true; // handlers are called on a running agent
                            // A send can always fail: the handler's own
                            // error, a refused call, or its panic (8.8,
                            // decision 0076).
                            sig.errors = Some(Ty::Builtin("Error", Vec::new()));
                            self.methods
                                .entry(def)
                                .or_default()
                                .insert(f.name.name.clone(), sig);
                        }
                    }
                }
                ItemKind::Extend(e) => {
                    let target = self.lower(&e.target, None);
                    let sigs = self.method_sigs(&e.methods, Some(&target));
                    self.note_private(&e.methods);
                    let interfaces: Vec<_> =
                        e.implements.iter().map(|t| self.lower(t, None)).collect();
                    match target {
                        Ty::Adt(def, _) => {
                            self.methods.entry(def).or_default().extend(sigs);
                            for i in interfaces {
                                if let Ty::Adt(i, args) = i {
                                    self.implements.entry(def).or_default().insert(i);
                                    let key = crate::checker::impl_key_adt(def);
                                    self.impl_args.insert((key, i), args);
                                }
                            }
                        }
                        Ty::Prim(p) => {
                            let name = p.name().to_string();
                            self.ext_methods
                                .entry(name.clone())
                                .or_default()
                                .extend(sigs);
                            for i in interfaces {
                                if let Ty::Adt(i, args) = i {
                                    self.ext_implements
                                        .entry(name.clone())
                                        .or_default()
                                        .insert(i);
                                    self.impl_args.insert((name.clone(), i), args);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                ItemKind::Fn(f) => {
                    if let Some(&def) = self.decl.get(&f.name.span.start) {
                        let sig = self.sig_of(f, None);
                        if f.c_library.is_some() {
                            self.check_c_signature(f, &sig);
                            self.c_fns.insert(def);
                        }
                        self.fns.insert(def, sig);
                    }
                }
                ItemKind::Use(_) | ItemKind::Test(_) | ItemKind::Stmt(_) => {}
            }
        }
    }

    fn collect_type(&mut self, t: &TypeDecl) {
        let Some(&def) = self.decl.get(&t.name.span.start) else {
            return;
        };
        let generics: Vec<_> = t
            .generics
            .iter()
            .filter_map(|g| self.decl.get(&g.name.span.start).copied())
            .collect();
        let self_ty = Ty::Adt(def, generics.iter().map(|g| Ty::Param(*g)).collect());
        // Register the generics first, so fields can be lowered.
        self.types.insert(
            def,
            TypeInfo {
                generics: generics.clone(),
                shape: Shape::Other,
            },
        );
        for i in &t.implements {
            // `Drop` is built in (spec 6.1).
            if matches!(&i.kind, kavach_ast::TypeKind::Named { path, .. }
                if path.segments.len() == 1 && path.segments[0].name == "Drop")
            {
                self.drop_types.insert(def);
                continue;
            }
            if let Ty::Adt(i, args) = self.lower(i, Some(&self_ty)) {
                self.implements.entry(def).or_default().insert(i);
                let key = crate::checker::impl_key_adt(def);
                self.impl_args.insert((key, i), args);
            }
        }
        let shape = match &t.body {
            TypeBody::Record(r) => {
                let fields = self.fields(&r.fields, &self_ty);
                let sigs = self.method_sigs(&r.methods, Some(&self_ty));
                self.note_private(&r.methods);
                self.methods.entry(def).or_default().extend(sigs);
                Shape::Record(fields)
            }
            TypeBody::Alias(_) => Shape::Other,
            TypeBody::Union(variants) => Shape::Union(
                variants
                    .iter()
                    .map(|v| VariantInfo {
                        name: v.name.name.clone(),
                        def: self
                            .decl
                            .get(&v.name.span.start)
                            .copied()
                            .unwrap_or(usize::MAX),
                        fields: self.fields(&v.fields, &self_ty),
                        value: v.value,
                    })
                    .collect(),
            ),
        };
        if let Shape::Union(vs) = &shape {
            self.check_enum_values(t, &vs.clone());
        }
        self.types.insert(def, TypeInfo { generics, shape });
    }

    fn fields(&mut self, fields: &[Field], self_ty: &Ty) -> Vec<FieldInfo> {
        fields
            .iter()
            .map(|f| FieldInfo {
                name: f.name.name.clone(),
                ty: self.lower(&f.ty, Some(self_ty)),
                has_default: f.default.is_some(),
                secret: f.is_secret,
                is_pub: f.is_pub,
            })
            .collect()
    }
}

fn builtin_type(name: &str, args: Vec<Ty>, self_ty: Option<&Ty>) -> Ty {
    if let Some(p) = Prim::from_name(name) {
        return Ty::Prim(p);
    }
    let mut args = args.into_iter();
    let mut next = || Box::new(args.next().unwrap_or(Ty::Error));
    match name {
        "List" => Ty::List(next()),
        "Set" => Ty::Set(next()),
        "Map" => {
            let k = next();
            Ty::Map(k, next())
        }
        "Self" => self_ty.cloned().unwrap_or(Ty::Error),
        other => Ty::Builtin(builtin_name(other), args.collect()),
    }
}
