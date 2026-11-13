//! Printing declarations as the tests' trees: functions, types, fields
//! and agent members.

use super::{block, expr, list, path, stmt, ty};
use kavach_ast::*;

pub fn item(item: &Item) -> String {
    let p = if item.is_pub { "pub " } else { "" };
    match &item.kind {
        ItemKind::Fn(f) => format!("({p}{})", function("fn", f)),
        ItemKind::Literal(l) => format!(
            "({p}literal {} {} check {}{})",
            l.name.name,
            ty(&l.ty),
            l.check.name,
            l.language
                .as_ref()
                .map_or(String::new(), |x| format!(" language {x:?}"))
        ),
        ItemKind::Type(t) => {
            let imps = if t.implements.is_empty() {
                String::new()
            } else {
                format!(" : {}", list(&t.implements, ty))
            };
            let body = match &t.body {
                TypeBody::Alias(t) => format!("= {}", ty(t)),
                TypeBody::Union(vs) => format!(
                    "= {}",
                    list(vs, |v| {
                        if v.fields.is_empty() {
                            v.name.name.clone()
                        } else {
                            format!("({} {})", v.name.name, list(&v.fields, field))
                        }
                    })
                ),
                TypeBody::Record(r) => {
                    let mut parts: Vec<String> = r.fields.iter().map(field).collect();
                    parts.extend(
                        r.invariants
                            .iter()
                            .map(|e| format!("(invariant {})", expr(e))),
                    );
                    parts.extend(r.methods.iter().map(self::item));
                    format!("{{{}}}", parts.join(" "))
                }
            };
            format!("({p}type {}{imps} {body})", t.name.name)
        }
        ItemKind::Interface(i) => format!(
            "(interface {} {})",
            i.name.name,
            list(&i.methods, self::item)
        ),
        ItemKind::Extend(e) => format!(
            "(extend {} {})",
            ty(&e.target),
            list(&e.methods, self::item)
        ),
        ItemKind::Capability(c) => format!(
            "(capability {} {})",
            c.name.name,
            list(&c.methods, self::item)
        ),
        ItemKind::Agent(a) => format!("(agent {} {})", a.name.name, list(&a.members, member)),
        ItemKind::Use(u) => {
            let names = if u.names.is_empty() {
                String::new()
            } else {
                format!(" {{{}}}", list(&u.names, |n| n.name.clone()))
            };
            let alias = u
                .alias
                .as_ref()
                .map(|a| format!(" as {}", a.name))
                .unwrap_or_default();
            format!("(use {}{names}{alias})", path(&u.path))
        }
        ItemKind::Test(t) => format!("(test {} {})", expr(&t.name), block(&t.body)),
        ItemKind::Stmt(s) => stmt(s),
    }
}

fn function(head: &str, f: &FnDecl) -> String {
    let mut out = format!("{head} {}", f.name.name);
    if !f.generics.is_empty() {
        out += &format!(" <{}>", list(&f.generics, |g| g.name.name.clone()));
    }
    out += &format!(
        " ({})",
        list(&f.params, |p| match (&p.ty, &p.default) {
            (Some(t), Some(d)) => format!("{}:{}={}", p.name.name, ty(t), expr(d)),
            (Some(t), None) => format!("{}:{}", p.name.name, ty(t)),
            _ => p.name.name.clone(),
        })
    );
    if let Some(r) = &f.ret {
        out += &format!(" -> {}", ty(r));
    }
    if let Some(e) = &f.errors {
        out += &format!(" ! {}", ty(e));
    }
    if !f.uses.is_empty() {
        out += &format!(" uses {}", list(&f.uses, path));
    }
    for r in &f.requires {
        out += &format!(" (require {})", expr(r));
    }
    for e in &f.ensures {
        out += &format!(" (ensure {})", expr(e));
    }
    for x in &f.examples {
        match &x.fails {
            Some(err) => out += &format!(" (example {} fails {})", expr(&x.expr), expr(err)),
            None => out += &format!(" (example {})", expr(&x.expr)),
        }
    }
    if let Some(b) = &f.body {
        out += &format!(" {}", block(b));
    }
    out
}

fn field(f: &Field) -> String {
    let secret = if f.is_secret { "secret " } else { "" };
    let default = f
        .default
        .as_ref()
        .map(|d| format!("={}", expr(d)))
        .unwrap_or_default();
    format!("{secret}{}:{}{default}", f.name.name, ty(&f.ty))
}

fn member(m: &AgentMember) -> String {
    match m {
        AgentMember::Can(ps) => format!("(can {})", list(ps, path)),
        AgentMember::Limit { target, rules } => format!(
            "(limit {} {})",
            path(target),
            list(rules, |r| match &r.per {
                Some(w) => format!("({} per {})", expr(&r.expr), w.name),
                None => expr(&r.expr),
            })
        ),
        AgentMember::Approve {
            target,
            when,
            by,
            within,
        } => {
            let within = within
                .as_ref()
                .map(|w| format!(" (within {})", expr(w)))
                .unwrap_or_default();
            format!(
                "(approve {} {} {}{within})",
                path(target),
                expr(when),
                expr(by)
            )
        }
        AgentMember::On(f) => format!("({})", function("on", f)),
        AgentMember::Model(e) => format!("(model {})", expr(e)),
        AgentMember::Settings(e) => format!("(settings {})", expr(e)),
    }
}
