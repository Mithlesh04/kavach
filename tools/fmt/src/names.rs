//! `kavach fmt` fixes names (spec 1.5): a declaration in the wrong style
//! (`orderCount`, `order_status` for a type) is renamed, with every use.
//! The resolver links most uses to their declaration; fields and methods
//! are found through the checked type of the value they are read from,
//! and named arguments through the function or type they are passed to.
//!
//! A `pub` name may be used by other files, so it is left for a person to
//! rename. Each rename is kept only if the file gets no new error from it
//! (a new name that is already taken); the others are reported.

use kavach_ast::visit::{self, VisitMut};
use kavach_ast::{Expr, ExprKind, Item, ItemKind, Span, TypeBody};
use kavach_checker::Ty;
use kavach_driver::{Compiled, Options, Severity, compile};
use kavach_resolver::DefKind;
use std::collections::HashMap;

/// One declaration to rename, and where its name is written.
struct Group {
    old: String,
    new: String,
    at: Vec<u32>,
}

/// `src` with its names fixed, and the names left for a person to fix.
pub fn fix_names(src: &str, path: &str) -> (String, Vec<String>) {
    let c = compile(src, path, Options::default());
    let mut left = Vec::new();
    let members = members(&c);
    let params = params(&c.file.items, &c);
    let mut groups: Vec<Group> = Vec::new();
    // Fields and methods by (type, old name); parameters by (function,
    // old name): their group.
    let (mut member_groups, mut param_groups) = (HashMap::new(), HashMap::new());
    for (span, fixed) in &c.resolution.name_fixes {
        if span.start as usize >= src.len() {
            continue; // the standard library's own files
        }
        let old = src[span.start as usize..span.end as usize].to_string();
        let member = members.get(&span.start);
        let def = c.resolution.defs.iter().position(|d| d.span == Some(*span));
        let public = member.map_or(is_pub(&c.file.items, *span), |m| m.is_pub);
        if public {
            left.push(format!("`{old}` is `pub`: rename it to `{fixed}` by hand"));
            continue;
        }
        let mut at = vec![span.start];
        if let (None, Some(def)) = (member, def) {
            at.extend(
                c.resolution
                    .uses
                    .iter()
                    .filter(|(_, d)| **d == def)
                    .map(|(a, _)| *a),
            );
        }
        if let Some(m) = member {
            member_groups.insert((m.owner, old.clone()), groups.len());
        }
        if let Some(f) = params.get(&span.start) {
            param_groups.insert((*f, old.clone()), groups.len());
        }
        groups.push(Group {
            old,
            new: fixed.clone(),
            at,
        });
    }
    if groups.is_empty() {
        return (src.to_string(), left);
    }
    let mut finder = Uses {
        c: &c,
        members: &member_groups,
        params: &param_groups,
        found: Vec::new(),
    };
    visit::walk_file(&mut finder, &mut c.file.clone());
    for (g, at) in finder.found {
        groups[g].at.push(at);
    }
    // Keep every rename that adds no error; try them all at once first.
    let errors = |c: &Compiled| {
        let style = |t: &str| t.contains(" use snake_case: ") || t.contains(" use PascalCase: ");
        c.messages
            .iter()
            .filter(|m| m.severity == Severity::Error && !style(&m.text))
            .count()
    };
    let before = errors(&c);
    let edits = |chosen: &[&Group]| -> Vec<(u32, String, String)> {
        chosen
            .iter()
            .flat_map(|g| g.at.iter().map(|a| (*a, g.old.clone(), g.new.clone())))
            .collect()
    };
    let all: Vec<&Group> = groups.iter().collect();
    let out = apply(src, edits(&all));
    if errors(&compile(&out, path, Options::default())) <= before {
        return (out, left);
    }
    let mut kept: Vec<&Group> = Vec::new();
    for g in &groups {
        kept.push(g);
        let out = apply(src, edits(&kept));
        if errors(&compile(&out, path, Options::default())) > before {
            kept.pop();
            left.push(format!(
                "`{}` can't become `{}`: that name is already in use",
                g.old, g.new
            ));
        }
    }
    (apply(src, edits(&kept)), left)
}

/// The edits applied from the end, so earlier offsets stay right. An edit
/// whose text isn't the old name (twice listed, or a bug) is skipped.
fn apply(src: &str, mut edits: Vec<(u32, String, String)>) -> String {
    edits.sort_by_key(|a| std::cmp::Reverse(a.0));
    edits.dedup_by_key(|e| e.0);
    let mut out = src.to_string();
    for (at, old, new) in edits {
        let at = at as usize;
        if out.get(at..at + old.len()) == Some(old.as_str()) {
            out.replace_range(at..at + old.len(), &new);
        }
    }
    out
}

/// A field or method of a record or union.
struct Member {
    /// The record or union it belongs to.
    owner: usize,
    is_pub: bool,
}

/// Every field and method, by where its name is declared.
fn members(c: &Compiled) -> HashMap<u32, Member> {
    let mut out = HashMap::new();
    for item in &c.file.items {
        let ItemKind::Type(t) = &item.kind else {
            continue;
        };
        let Some(owner) = c
            .resolution
            .defs
            .iter()
            .position(|d| d.span == Some(t.name.span))
        else {
            continue;
        };
        let mut add = |span: Span, is_pub: bool| {
            out.insert(span.start, Member { owner, is_pub });
        };
        match &t.body {
            TypeBody::Record(r) => {
                r.fields
                    .iter()
                    .for_each(|f| add(f.name.span, f.is_pub || item.is_pub));
                for m in &r.methods {
                    if let ItemKind::Fn(f) = &m.kind {
                        add(f.name.span, m.is_pub);
                    }
                }
            }
            TypeBody::Alias(_) => {}
            TypeBody::Union(vs) => {
                for v in vs {
                    v.fields.iter().for_each(|f| add(f.name.span, item.is_pub));
                }
            }
        }
    }
    out
}

/// The function each parameter belongs to, by where its name is declared.
fn params(items: &[Item], c: &Compiled) -> HashMap<u32, usize> {
    let mut out = HashMap::new();
    for item in items {
        let fns: Vec<&kavach_ast::FnDecl> = match &item.kind {
            ItemKind::Fn(f) => vec![f],
            ItemKind::Type(t) => match &t.body {
                TypeBody::Record(r) => r
                    .methods
                    .iter()
                    .filter_map(|m| match &m.kind {
                        ItemKind::Fn(f) => Some(f),
                        _ => None,
                    })
                    .collect(),
                TypeBody::Union(_) | TypeBody::Alias(_) => Vec::new(),
            },
            _ => Vec::new(),
        };
        for f in fns {
            let Some(def) = c
                .resolution
                .defs
                .iter()
                .position(|d| d.span == Some(f.name.span))
            else {
                continue;
            };
            for p in &f.params {
                out.insert(p.name.span.start, def);
            }
        }
    }
    out
}

/// Whether the declaration whose name is at `span` is a `pub` item.
fn is_pub(items: &[Item], span: Span) -> bool {
    items.iter().any(|item| {
        let name = match &item.kind {
            ItemKind::Fn(f) => Some(f.name.span),
            ItemKind::Type(t) => Some(t.name.span),
            ItemKind::Interface(i) => Some(i.name.span),
            ItemKind::Capability(c) => Some(c.name.span),
            ItemKind::Agent(a) => Some(a.name.span),
            _ => None,
        };
        item.is_pub && name == Some(span)
    })
}

/// The record or union a value of type `t` is (through `?`).
fn owner(t: Option<&Ty>) -> Option<usize> {
    match t? {
        Ty::Adt(def, _) => Some(*def),
        Ty::Optional(inner) => owner(Some(inner)),
        _ => None,
    }
}

/// Finds reads of renamed fields and methods (`order.totalAmount`,
/// `order.payNow()`), named arguments that build a record or are passed
/// to a function, and `with`: each with its rename group.
struct Uses<'c> {
    c: &'c Compiled,
    members: &'c HashMap<(usize, String), usize>,
    params: &'c HashMap<(usize, String), usize>,
    found: Vec<(usize, u32)>,
}

impl Uses<'_> {
    /// Whether calling `callee` builds a record or a union variant.
    fn builds(&self, callee: &Expr) -> bool {
        let ExprKind::Name(id) = &callee.kind else {
            return false;
        };
        self.c
            .resolution
            .def_at(id.span.start)
            .is_some_and(|d| matches!(d.kind, DefKind::Type | DefKind::Variant { .. }))
    }

    fn type_of(&self, e: &Expr) -> Option<&Ty> {
        self.c.expr_types.get(&(e.span.start, e.span.end))
    }

    fn rename(&mut self, owner: Option<usize>, name: &kavach_ast::Ident) {
        let key = owner.map(|o| (o, name.name.clone()));
        if let Some(g) = key.and_then(|k| self.members.get(&k)) {
            self.found.push((*g, name.span.start));
        }
    }

    /// The function `callee` names, if it is one.
    fn function(&self, callee: &Expr) -> Option<usize> {
        let ExprKind::Name(id) = &callee.kind else {
            return None;
        };
        let def = *self.c.resolution.uses.get(&id.span.start)?;
        (self.c.resolution.defs[def].kind == DefKind::Fn).then_some(def)
    }
}

impl VisitMut for Uses<'_> {
    fn expr(&mut self, e: &mut Expr) {
        match &e.kind {
            ExprKind::Field { base, name, .. } => {
                let owner = owner(self.type_of(base));
                self.rename(owner, name);
            }
            ExprKind::With { base, fields } => {
                let owner = owner(self.type_of(base));
                for (name, _) in fields {
                    self.rename(owner, name);
                }
            }
            ExprKind::Call { callee, args, .. } if self.builds(callee) => {
                // A record or variant built with named arguments.
                let owner = owner(self.type_of(e));
                for a in args {
                    if let Some(name) = &a.name {
                        self.rename(owner, name);
                    }
                }
            }
            ExprKind::Call { callee, args, .. } => {
                // Named arguments of a function whose parameter is renamed.
                if let Some(f) = self.function(callee) {
                    for name in args.iter().filter_map(|a| a.name.as_ref()) {
                        if let Some(g) = self.params.get(&(f, name.name.clone())) {
                            self.found.push((*g, name.span.start));
                        }
                    }
                }
            }
            _ => {}
        }
        visit::walk_expr(self, e);
    }
}
