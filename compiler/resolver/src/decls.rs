//! Declarations. Pass 1 (`collect`) records every top-level name, so they
//! can be used before they appear (spec 1.4). Pass 2 (`item`) resolves the
//! inside of each declaration.

use crate::DefKind;
use crate::prelude::LIMIT_FUNCTIONS;
use crate::resolver::Resolver;
use kavach_ast::*;
use std::collections::{HashMap, HashSet};

impl Resolver {
    pub(crate) fn collect(&mut self, items: &[Item]) {
        for item in items {
            match &item.kind {
                ItemKind::Fn(f) => {
                    self.check_snake(&f.name, "functions");
                    self.define(&f.name, DefKind::Fn);
                }
                ItemKind::Type(t) => {
                    self.check_pascal(&t.name, "types");
                    let Some(id) = self.define(&t.name, DefKind::Type) else {
                        continue;
                    };
                    self.type_ids.insert(t.name.span.start, id);
                    if let TypeBody::Union(variants) = &t.body {
                        let mut own = HashMap::new();
                        for v in variants {
                            self.check_pascal(&v.name, "variants");
                            if own.contains_key(&v.name.name) {
                                self.error(
                                    v.name.span,
                                    format!(
                                        "`{}` is already a variant of `{}`",
                                        v.name.name, t.name.name
                                    ),
                                );
                            }
                            let vid = self.add_def(
                                &v.name.name,
                                DefKind::Variant { of: id },
                                Some(v.name.span),
                            );
                            own.insert(v.name.name.clone(), vid);
                            self.variants
                                .entry(v.name.name.clone())
                                .or_default()
                                .push(vid);
                        }
                        self.union_variants.insert(id, own);
                    }
                }
                ItemKind::Interface(i) => {
                    self.check_pascal(&i.name, "interfaces");
                    self.define(&i.name, DefKind::Interface);
                }
                ItemKind::Capability(c) => {
                    self.check_pascal(&c.name, "capabilities");
                    self.define(&c.name, DefKind::Capability);
                    let methods = c
                        .methods
                        .iter()
                        .filter_map(|m| match &m.kind {
                            ItemKind::Fn(f) => Some((
                                f.name.name.clone(),
                                f.params.iter().map(|p| p.name.name.clone()).collect(),
                            )),
                            _ => None,
                        })
                        .collect();
                    self.capability_params.insert(c.name.name.clone(), methods);
                }
                ItemKind::Agent(a) => {
                    self.check_pascal(&a.name, "agents");
                    self.define(&a.name, DefKind::Agent);
                }
                // `use std.…`; other modules are imported later
                // (`modules.rs`), once every module's names are known.
                ItemKind::Use(u) if u.path.segments.first().is_some_and(|s| s.name == "std") => {
                    if !u.names.is_empty() {
                        for name in &u.names {
                            self.define(name, DefKind::Import);
                        }
                    } else if let Some(name) = u.alias.as_ref().or(u.path.segments.last()) {
                        self.define(name, DefKind::Import);
                    }
                }
                ItemKind::Use(_) | ItemKind::Extend(_) | ItemKind::Test(_) | ItemKind::Stmt(_) => {}
            }
        }
    }

    pub(crate) fn item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Fn(f) => self.function(f),
            ItemKind::Type(t) => self.type_decl(t),
            ItemKind::Interface(i) => {
                self.push();
                self.generics(&i.generics);
                self.methods(&i.methods);
                self.pop();
            }
            ItemKind::Extend(e) => {
                self.push();
                self.generics(&e.generics);
                self.type_expr(&e.target);
                self.extend_generics(e);
                for t in &e.implements {
                    self.type_expr(t);
                }
                self.methods(&e.methods);
                self.pop();
            }
            ItemKind::Capability(c) => self.methods(&c.methods),
            ItemKind::Agent(a) => self.agent(a),
            ItemKind::Test(t) => {
                for path in &t.uses {
                    self.use_type(&path.segments[0]);
                }
                for (capability, value) in &t.with {
                    self.use_type(capability);
                    self.expr(value);
                }
                self.block(&t.body);
            }
            ItemKind::Use(_) | ItemKind::Stmt(_) => {}
        }
    }

    fn methods(&mut self, methods: &[Item]) {
        for m in methods {
            if let ItemKind::Fn(f) = &m.kind {
                self.check_snake(&f.name, "functions");
                // A definition of its own, in no scope (a method is reached
                // through its value), so the checker can follow what it
                // uses (spec 8.1).
                self.add_def(&f.name.name, DefKind::Fn, Some(f.name.span));
                self.function(f);
            }
        }
    }

    /// `extend<S> App<S>`: each name stands for the parameter of `App` in
    /// its place (spec 2.5); `finish` makes it that parameter.
    fn extend_generics(&mut self, e: &ExtendDecl) {
        let TypeKind::Named { path, args } = &e.target.kind else {
            return;
        };
        let ty = path.segments.iter().rev().find_map(|s| {
            let d = *self.res.uses.get(&s.span.start)?;
            (self.res.defs[d].kind == DefKind::Type).then_some(d)
        });
        for g in &e.generics {
            let place = args.iter().position(|a| match &a.kind {
                TypeKind::Named { path, args } => {
                    args.is_empty()
                        && path.segments.len() == 1
                        && path.segments[0].name == g.name.name
                }
                _ => false,
            });
            match (ty, place, self.in_scope(&g.name.name)) {
                (Some(ty), Some(i), Some(own)) => self.extend_params.push((own, ty, i)),
                _ => self.error(
                    g.name.span,
                    format!("`{}` isn't one of the type's parameters: write it in the type, as `extend<{0}> App<{0}>` (spec 2.5)", g.name.name),
                ),
            }
        }
    }

    fn generics(&mut self, generics: &[GenericParam]) {
        for g in generics {
            self.check_pascal(&g.name, "type parameters");
            self.define(&g.name, DefKind::GenericParam);
            for bound in &g.bounds {
                self.type_expr(bound);
            }
        }
    }

    pub(crate) fn function(&mut self, f: &FnDecl) {
        self.push();
        self.generics(&f.generics);
        for p in &f.params {
            if p.name.name == "self" {
                continue;
            }
            if let Some(ty) = &p.ty {
                self.type_expr(ty);
            }
            if let Some(default) = &p.default {
                self.expr(default);
            }
            self.check_snake(&p.name, "parameters");
            // An `inout` parameter is a variable in the function (spec 3.9);
            // with no body (an interface's method) it is only a parameter.
            let kind = if p.is_inout && f.body.is_some() {
                DefKind::Local { mutable: true }
            } else {
                DefKind::Param
            };
            self.define(&p.name, kind);
        }
        for ty in f.ret.iter().chain(&f.errors) {
            self.type_expr(ty);
        }
        for path in &f.uses {
            self.use_type(&path.segments[0]);
        }
        // A `fix` (spec 12.6) reads the parameters, like a `require`.
        for r in f
            .requires
            .iter()
            .chain(f.deprecated.iter().filter_map(|d| d.fix.as_ref()))
        {
            self.expr(r);
        }
        self.push();
        self.define_quiet("result", DefKind::Result);
        for e in &f.ensures {
            self.expr(e);
        }
        self.pop();
        for x in &f.examples {
            self.expr(&x.expr);
            if let Some(fails) = &x.fails {
                self.expr(fails);
            }
        }
        if let Some(body) = &f.body {
            self.block(body);
        }
        self.pop();
    }

    fn type_decl(&mut self, t: &TypeDecl) {
        self.push();
        self.generics(&t.generics);
        if let Some(&id) = self.type_ids.get(&t.name.span.start) {
            let params = t
                .generics
                .iter()
                .filter_map(|g| self.in_scope(&g.name.name));
            let params = params.collect();
            self.type_params.insert(id, params);
        }
        for ty in &t.implements {
            self.type_expr(ty);
        }
        match &t.body {
            TypeBody::Union(variants) => {
                for v in variants {
                    self.fields(&v.fields);
                }
            }
            TypeBody::Record(r) => {
                self.fields(&r.fields);
                // Inside an invariant, the record's fields are plain names (spec 7.3).
                self.push();
                for f in &r.fields {
                    self.define_quiet(&f.name.name, DefKind::Field);
                }
                for inv in &r.invariants {
                    self.expr(inv);
                }
                self.pop();
                self.methods(&r.methods);
            }
            TypeBody::Alias(ty) => self.type_expr(ty),
        }
        self.pop();
    }

    fn fields(&mut self, fields: &[Field]) {
        let mut seen = HashSet::new();
        for f in fields {
            self.check_snake(&f.name, "fields");
            if !seen.insert(f.name.name.clone()) {
                self.error(
                    f.name.span,
                    format!("the field `{}` is declared twice", f.name.name),
                );
            }
            self.type_expr(&f.ty);
            if let Some(default) = &f.default {
                self.expr(default);
            }
        }
    }

    fn agent(&mut self, a: &AgentDecl) {
        for member in &a.members {
            match member {
                AgentMember::Can(paths) => {
                    for path in paths {
                        self.use_type(&path.segments[0]);
                    }
                }
                AgentMember::Limit { target, rules } => {
                    self.rule_scope(target, true);
                    for rule in rules {
                        self.expr(&rule.expr);
                    }
                    self.pop();
                }
                AgentMember::Approve {
                    target,
                    when,
                    by,
                    within,
                } => {
                    self.rule_scope(target, false);
                    self.expr(when);
                    self.pop();
                    self.expr(by);
                    if let Some(w) = within {
                        self.expr(w);
                    }
                }
                AgentMember::On(f) => {
                    // A handler is a method of the running agent, not a name
                    // in scope; its definition lets the checker see what
                    // capabilities it uses (spec 8.3).
                    self.check_snake(&f.name, "handlers");
                    self.add_def(&f.name.name, DefKind::Fn, Some(f.name.span));
                    self.function(f);
                }
                AgentMember::Model(e) | AgentMember::Settings(e) => self.expr(e),
            }
        }
    }

    /// Opens a scope with the parameters of the limited method, and `sum`
    /// and `count` in `limit` rules (spec 8.3).
    fn rule_scope(&mut self, target: &Path, is_limit: bool) {
        self.use_type(&target.segments[0]);
        self.push();
        let params = target.segments.get(1).and_then(|method| {
            self.capability_params
                .get(&target.segments[0].name)?
                .get(&method.name)
                .cloned()
        });
        for name in params.unwrap_or_default() {
            self.define_quiet(&name, DefKind::Param);
        }
        if is_limit {
            for name in LIMIT_FUNCTIONS {
                self.define_quiet(name, DefKind::Builtin);
            }
        }
    }
}
