//! Modules (spec 10.1, 10.2): one file is one module. Each module has its
//! own top-level scope; `use` brings names in from other modules, and only
//! `pub` names can be used from outside.
//!
//! All files of a project are parsed together, so each module is a range
//! of the source (and of the file's items).

use crate::resolver::Resolver;
use crate::{DefId, DefKind, Resolution};
use kavach_ast::{File, Ident, Item, ItemKind, Path, TypeBody};
use std::collections::{HashMap, HashSet};

/// One module: its name (`billing.invoice`), where its source is, and
/// its package: empty for the program's own modules, else the name of the
/// dependency it comes from (spec 10.12).
#[derive(Clone, Debug, Default)]
pub struct Module {
    pub name: String,
    pub start: u32,
    pub end: u32,
    pub package: String,
}

/// What resolving needs to know about the modules.
#[derive(Default)]
pub(crate) struct Modules {
    pub(crate) names: Vec<String>,
    /// Each module's own top-level names (not what it imports).
    pub(crate) own: Vec<HashMap<String, DefId>>,
    /// Names other modules may use.
    pub(crate) public: HashSet<DefId>,
    /// The module whose items are being resolved.
    pub(crate) current: usize,
}

/// The name an item declares, if any.
fn item_name(item: &Item) -> Option<&Ident> {
    match &item.kind {
        ItemKind::Fn(f) => Some(&f.name),
        ItemKind::Type(t) => Some(&t.name),
        ItemKind::Interface(i) => Some(&i.name),
        ItemKind::Capability(c) => Some(&c.name),
        ItemKind::Agent(a) => Some(&a.name),
        ItemKind::Literal(l) => Some(&l.name),
        _ => None,
    }
}

impl Resolver {
    pub(crate) fn run_modules(mut self, file: &File, modules: &[Module]) -> Resolution {
        // Each module's items: a range of `file.items`.
        let ranges: Vec<&[Item]> = modules
            .iter()
            .map(|m| {
                let a = file.items.partition_point(|i| i.span.start < m.start);
                let b = file.items.partition_point(|i| i.span.start < m.end);
                &file.items[a..b]
            })
            .collect();
        self.modules.names = modules.iter().map(|m| m.name.clone()).collect();
        self.res.modules = modules.to_vec();
        // 1. Every module's own names, so any module can use any other.
        let mut scopes = Vec::new();
        for items in &ranges {
            self.reset_scopes();
            self.collect(items);
            scopes.push(self.take_module_scope());
            for item in items.iter().filter(|i| i.is_pub) {
                let Some(def) = item_name(item).and_then(|n| {
                    scopes
                        .last()
                        .and_then(|s: &HashMap<String, DefId>| s.get(&n.name).copied())
                }) else {
                    continue;
                };
                self.modules.public.insert(def);
                // A public union's variants are public too.
                if let (ItemKind::Type(t), Some(vs)) = (&item.kind, self.union_variants.get(&def))
                    && matches!(t.body, TypeBody::Union(_))
                {
                    self.modules.public.extend(vs.values().copied());
                }
            }
        }
        self.modules.own = scopes.clone();
        self.share_builtin_types(&mut scopes);
        // 2. `use` of other modules. `use std.math` names the standard
        // library module written in Kavach, when there is one.
        for (m, items) in ranges.iter().enumerate() {
            self.modules.current = m;
            for item in items.iter() {
                let ItemKind::Use(u) = &item.kind else {
                    continue;
                };
                if u.path.segments.first().is_some_and(|s| s.name != "std") {
                    self.import(&mut scopes[m], &u.path, &u.names, u.alias.as_ref());
                } else if !u.names.is_empty() {
                    self.std_names(&mut scopes[m], &u.path.segments, &u.names);
                } else if let (Some(std), true) =
                    (self.module_named(&u.path.segments), u.names.is_empty())
                {
                    let bound = u.alias.as_ref().or(u.path.segments.last());
                    if let Some(def) = bound.and_then(|b| scopes[m].get(&b.name)) {
                        self.res.module_defs.insert(*def, std);
                    }
                }
            }
        }
        // 3. Every item, in its own module's scope.
        for (m, items) in ranges.iter().enumerate() {
            self.modules.current = m;
            self.reset_scopes();
            self.set_module_scope(scopes[m].clone());
            for item in items
                .iter()
                .filter(|i| !matches!(i.kind, ItemKind::Stmt(_)))
            {
                self.item(item);
            }
            let stmts: Vec<_> = items
                .iter()
                .filter_map(|i| match &i.kind {
                    ItemKind::Stmt(s) => Some(s.clone()),
                    _ => None,
                })
                .collect();
            if m == 0 {
                self.stmts(&stmts);
            } else if let Some(s) = stmts.first() {
                self.error(s.span, "only the entry file (`src/main.kav`) can have statements outside functions (spec 1.3)");
            }
            if m > 0 {
                for item in items.iter() {
                    if let ItemKind::Fn(f) = &item.kind
                        && f.name.name == "main"
                    {
                        self.error(
                            f.name.span,
                            "only the entry file (`src/main.kav`) can have `main` (spec 10.3)",
                        );
                    }
                }
            }
        }
        self.warn_unused();
        self.finish()
    }

    /// The standard library's built-in types, such as `Json` (spec 2.1),
    /// come from modules written in Kavach (`std.json`, added by the
    /// driver). Their public types and interfaces (`LogEvent`) are names in
    /// every other module, unless it has its own; and a variant name that a
    /// program also uses for its own variants stays the program's (write
    /// `Json.Text` then). A name two standard unions share (`FsError.Denied`
    /// and `AiError.Denied`) stays ambiguous: the checker settles it by type.
    fn share_builtin_types(&mut self, scopes: &mut [HashMap<String, DefId>]) {
        let std_variants: std::collections::HashSet<DefId> = (0..scopes.len())
            .filter(|&b| self.modules.names[b].starts_with("std."))
            .flat_map(|b| scopes[b].values().copied().collect::<Vec<_>>())
            .filter(|d| self.modules.public.contains(d))
            .filter_map(|d| self.union_variants.get(&d))
            .flat_map(|vs| vs.values().copied())
            .collect();
        for b in 0..scopes.len() {
            if !self.modules.names[b].starts_with("std.") {
                continue;
            }
            let types: Vec<(String, DefId)> = scopes[b]
                .iter()
                .filter(|(_, d)| {
                    self.modules.public.contains(d)
                        && matches!(self.res.defs[**d].kind, DefKind::Type | DefKind::Interface)
                })
                .map(|(n, d)| (n.clone(), *d))
                .collect();
            for (m, scope) in scopes.iter_mut().enumerate() {
                if m != b {
                    for (name, def) in &types {
                        scope.entry(name.clone()).or_insert(*def);
                    }
                }
            }
            for (_, def) in &types {
                let Some(variants) = self.union_variants.get(def) else {
                    continue;
                };
                for (name, v) in variants {
                    if let Some(all) = self.variants.get_mut(name)
                        && all.iter().any(|x| !std_variants.contains(x))
                    {
                        all.retain(|x| x != v);
                    }
                }
            }
        }
    }

    /// `use std.time.{Date, utc}`: the public types and functions written
    /// in Kavach themselves (spec 14.1). A name the runtime provides is
    /// reached through its module.
    fn std_names(&mut self, scope: &mut HashMap<String, DefId>, path: &[Ident], names: &[Ident]) {
        let std = self.module_named(path);
        for n in names {
            let def = std.and_then(|s| self.modules.own[s].get(&n.name).copied());
            let def = def.filter(|d| {
                self.modules.public.contains(d)
                    && matches!(
                        self.res.defs[*d].kind,
                        DefKind::Type | DefKind::Interface | DefKind::Fn
                    )
            });
            if let Some(def) = def {
                scope.insert(n.name.clone(), def);
                self.res.uses.insert(n.span.start, def);
                continue;
            }
            let module: Vec<&str> = path.iter().map(|s| s.name.as_str()).collect();
            let last = module.last().copied().unwrap_or("std");
            self.error(
                n.span,
                format!(
                    "`{}` can't be named alone: write `use {}` and `{last}.{}` (spec 14.1)",
                    n.name,
                    module.join("."),
                    n.name
                ),
            );
        }
    }

    /// The module named by `path`, if there is one. Inside a dependency,
    /// its own modules come first: in package `geometry`, `shapes` is
    /// `geometry.shapes` (spec 10.12).
    fn module_named(&self, path: &[Ident]) -> Option<usize> {
        let name: Vec<&str> = path.iter().map(|s| s.name.as_str()).collect();
        let name = name.join(".");
        let package = self
            .res
            .modules
            .get(self.modules.current)
            .map_or("", |m| m.package.as_str());
        let own = format!("{package}.{name}");
        let names = &self.modules.names;
        (!package.is_empty())
            .then(|| names.iter().position(|n| *n == own))
            .flatten()
            .or_else(|| names.iter().position(|n| *n == name))
            .or_else(|| self.own_package_module(path))
    }

    /// `use shapes.circle` inside package `shapes` itself, whose root
    /// module is named `shapes`: the module `circle` (spec 10.12).
    fn own_package_module(&self, path: &[Ident]) -> Option<usize> {
        let (first, rest) = path.split_first()?;
        let own = |i: usize| {
            self.res
                .modules
                .get(i)
                .is_some_and(|m| m.package.is_empty())
        };
        let root = self.modules.names.iter().position(|n| *n == first.name)?;
        if rest.is_empty() || !own(root) || !own(self.modules.current) {
            return None;
        }
        let inner: Vec<&str> = rest.iter().map(|s| s.name.as_str()).collect();
        let inner = inner.join(".");
        self.modules
            .names
            .iter()
            .position(|n| *n == inner)
            .filter(|&i| own(i))
    }

    /// `name` in module `m`, if it may be used from the current module.
    pub(crate) fn member(&mut self, m: usize, name: &Ident) -> Option<DefId> {
        let module = self.modules.names[m].clone();
        let Some(&def) = self.modules.own[m].get(&name.name) else {
            // A standard library module written in Kavach has only part of
            // the module: the runtime provides the rest (`math.sqrt`), and
            // the type checker knows those names.
            if module.starts_with("std.") {
                return None;
            }
            self.error(
                name.span,
                format!("module `{module}` has no `{}`", name.name),
            );
            return None;
        };
        if m != self.modules.current && !self.modules.public.contains(&def) {
            self.error(
                name.span,
                format!("`{}` is private to module `{module}`: mark it `pub` there to use it here (spec 10.1)", name.name),
            );
        }
        self.res.uses.insert(name.span.start, def);
        Some(def)
    }

    /// `use orders`, `use billing.invoice.{Invoice, create}`,
    /// `use orders.total as sum`.
    fn import(
        &mut self,
        scope: &mut HashMap<String, DefId>,
        path: &Path,
        names: &[Ident],
        alias: Option<&Ident>,
    ) {
        let segs = &path.segments;
        if let Some(m) = self.module_named(segs) {
            if names.is_empty() {
                // The module itself: `orders.total(…)` (spec 10.2).
                let bound = alias.unwrap_or(&segs[segs.len() - 1]);
                let def = self.add_def(&bound.name, DefKind::Module, Some(bound.span));
                self.res.module_defs.insert(def, m);
                scope.insert(bound.name.clone(), def);
            }
            for n in names {
                if let Some(def) = self.member(m, n) {
                    scope.insert(n.name.clone(), def);
                }
            }
            return;
        }
        // `use orders.total`: one item of a module.
        if let (Some(m), Some(last)) = (
            self.module_named(&segs[..segs.len().saturating_sub(1)]),
            segs.last(),
        ) {
            if let Some(def) = self.member(m, last) {
                scope.insert(alias.unwrap_or(last).name.clone(), def);
            }
            return;
        }
        let shown: Vec<&str> = segs.iter().map(|s| s.name.as_str()).collect();
        self.error(
            path.span,
            format!(
                "there is no module `{}`: modules are the `.kav` files in `src/` (spec 10.1)",
                shown.join(".")
            ),
        );
    }
}
