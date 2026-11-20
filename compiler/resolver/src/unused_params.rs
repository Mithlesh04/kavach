// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Parameters a function never reads (spec 1.4): a warning, as for
//! variables. None where something else fixes the signature: a method
//! that implements an interface or a capability, a function used as a
//! value, `main`, an agent's handler, a `gpu fn` and a function with no
//! body. Only the program's own modules are looked at.

use crate::resolver::Resolver;
use crate::{DefId, Diagnostic};
use kavach_ast::*;
use std::collections::{HashMap, HashSet};

/// The methods of the built-in interfaces (spec 1.11, 2.16, 6.1).
const BUILT_IN: &[(&str, &[&str])] = &[
    ("Add", &["add"]),
    ("Sub", &["sub"]),
    ("Mul", &["mul"]),
    ("Div", &["div"]),
    ("Rem", &["rem"]),
    ("Pow", &["pow"]),
    ("Neg", &["neg"]),
    ("MatMul", &["matmul"]),
    ("Comparable", &["compare"]),
    ("Compare", &["lt", "le", "gt", "ge"]),
    ("Index", &["index"]),
    ("IndexSet", &["index_set"]),
    ("Slice", &["slice"]),
    ("Display", &["display"]),
    ("Drop", &["drop"]),
];

/// The methods each type must have, by the type's name: those of what it
/// implements. `None` when it implements something whose methods aren't
/// known here (a built-in capability): then every method is fixed.
type Fixed = HashMap<String, Option<HashSet<String>>>;

impl Resolver {
    /// A name read as a value: a function named here is used as one.
    pub(crate) fn use_value(&mut self, ident: &Ident) {
        self.use_name(ident);
        self.values.insert(ident.span.start);
    }

    /// The function a call names: `f(x)` or `orders.total(x)` doesn't use
    /// `f` or `total` as a value.
    pub(crate) fn callee(&mut self, callee: &Expr) {
        match &callee.kind {
            ExprKind::Name(ident) => self.use_name(ident),
            ExprKind::Field { base, name, .. } if self.module_of(base).is_some() => {
                let m = self.module_of(base).unwrap_or(0);
                self.member(m, name);
            }
            _ => self.expr(callee),
        }
    }

    pub(crate) fn warn_unused_params(&mut self, file: &File) {
        let fns = functions(file);
        for (f, _) in &fns {
            if f.body.is_some() {
                let params = f.params.iter().filter(|p| p.name.name != "self");
                self.param_spans.extend(params.map(|p| p.name.span.start));
            }
        }
        // A misspelled use leaves its name unused: the error says it all.
        if !self.res.errors.is_empty() {
            return;
        }
        let used: HashSet<DefId> = self.res.uses.values().copied().collect();
        let valued: HashSet<DefId> = (self.values.iter())
            .filter_map(|at| self.res.uses.get(at).copied())
            .collect();
        let at: HashMap<u32, DefId> = (self.res.defs.iter().enumerate())
            .filter_map(|(id, d)| Some((d.span?.start, id)))
            .collect();
        let mut found = Vec::new();
        for (f, fixed) in fns {
            let value = at
                .get(&f.name.span.start)
                .is_some_and(|d| valued.contains(d));
            if fixed
                || value
                || f.body.is_none()
                || f.is_gpu
                || f.gpu_stage != GpuStage::Compute
                || !self.own_module(f.name.span.start)
            {
                continue;
            }
            for p in &f.params {
                let name = &p.name.name;
                if name == "self" || name.starts_with('_') {
                    continue;
                }
                let Some(id) = at.get(&p.name.span.start) else {
                    continue;
                };
                if !used.contains(id) {
                    found.push(Diagnostic {
                        message: format!(
                            "`{name}` is never used: remove it, or name it `_{name}` if it must stay"
                        ),
                        span: p.name.span,
                    });
                }
            }
        }
        self.res.warnings.extend(found);
    }

    /// Whether the source at `offset` is in the program's own modules, not
    /// the standard library's or a dependency's.
    fn own_module(&self, offset: u32) -> bool {
        let module = &self.res.modules[self.res.module_at(offset)];
        !module.name.starts_with("std.") && module.package.is_empty()
    }
}

/// Every function, method and handler of `file`, and whether something
/// else fixes its signature (`main`, an interface, an agent).
fn functions(file: &File) -> Vec<(&FnDecl, bool)> {
    let fixed = fixed_methods(file);
    let forced = |ty: Option<String>, name: &str| {
        ty.and_then(|t| fixed.get(&t))
            .is_some_and(|m| m.as_ref().is_none_or(|m| m.contains(name)))
    };
    let mut out = Vec::new();
    for item in &file.items {
        let (methods, ty, all): (&[Item], _, _) = match &item.kind {
            ItemKind::Fn(f) => {
                out.push((f, f.name.name == "main"));
                continue;
            }
            ItemKind::Type(t) => match &t.body {
                TypeBody::Record(r) => (&r.methods, Some(t.name.name.clone()), false),
                _ => continue,
            },
            ItemKind::Extend(e) => (&e.methods, type_key(&e.target), false),
            ItemKind::Interface(i) => (&i.methods, None, true),
            ItemKind::Capability(c) => (&c.methods, None, true),
            ItemKind::Agent(a) => {
                for m in &a.members {
                    if let AgentMember::On(f) = m {
                        out.push((f, true));
                    }
                }
                continue;
            }
            _ => continue,
        };
        for m in methods {
            if let ItemKind::Fn(f) = &m.kind {
                out.push((f, all || forced(ty.clone(), &f.name.name)));
            }
        }
    }
    out
}

/// Each type's methods that an interface or capability it implements
/// fixes, by the type's name. A type may say so in its declaration or in
/// any `extend` of it (spec 2.8).
fn fixed_methods(file: &File) -> Fixed {
    let mut declared: HashMap<&str, HashSet<String>> = HashMap::new();
    for item in &file.items {
        let (name, methods) = match &item.kind {
            ItemKind::Interface(i) => (&i.name, &i.methods),
            ItemKind::Capability(c) => (&c.name, &c.methods),
            _ => continue,
        };
        let names = methods.iter().filter_map(|m| match &m.kind {
            ItemKind::Fn(f) => Some(f.name.name.clone()),
            _ => None,
        });
        declared.entry(&name.name).or_default().extend(names);
    }
    let mut fixed = Fixed::new();
    for item in &file.items {
        let (ty, implements) = match &item.kind {
            ItemKind::Type(t) => (Some(t.name.name.clone()), &t.implements),
            ItemKind::Extend(e) => (type_key(&e.target), &e.implements),
            _ => continue,
        };
        let Some(ty) = ty else { continue };
        for i in implements {
            let Some(name) = type_key(i) else { continue };
            let known = declared.get(name.as_str()).cloned().or_else(|| {
                let (_, methods) = BUILT_IN.iter().find(|(n, _)| *n == name)?;
                Some(methods.iter().map(|m| m.to_string()).collect())
            });
            let entry = fixed.entry(ty.clone()).or_insert(Some(HashSet::new()));
            match (entry, known) {
                (Some(have), Some(more)) => have.extend(more),
                (entry, _) => *entry = None,
            }
        }
    }
    fixed
}

/// The name a type is known by here: `Rect` for `shapes.Rect<T>`, and one
/// name for every list and every dict.
fn type_key(t: &TypeExpr) -> Option<String> {
    match &t.kind {
        TypeKind::Named { path, .. } => path.segments.last().map(|s| s.name.clone()),
        TypeKind::List(_) => Some("[]".to_string()),
        TypeKind::Map(..) => Some("[:]".to_string()),
        _ => None,
    }
}
