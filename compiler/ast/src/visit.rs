//! A mutable walk over the whole tree. A pass implements [`VisitMut`],
//! overrides the methods for the nodes it changes, and calls the `walk_*`
//! function inside them to keep going.

use crate::*;

pub trait VisitMut {
    fn expr(&mut self, e: &mut Expr) {
        walk_expr(self, e);
    }

    fn type_expr(&mut self, t: &mut TypeExpr) {
        walk_type(self, t);
    }

    fn pattern(&mut self, p: &mut Pattern) {
        walk_pattern(self, p);
    }

    fn stmt(&mut self, s: &mut Stmt) {
        walk_stmt(self, s);
    }
}

pub fn walk_file<V: VisitMut + ?Sized>(v: &mut V, f: &mut File) {
    for item in &mut f.items {
        walk_item(v, item);
    }
}

pub fn walk_item<V: VisitMut + ?Sized>(v: &mut V, item: &mut Item) {
    match &mut item.kind {
        ItemKind::Fn(f) => walk_fn(v, f),
        ItemKind::Type(t) => {
            walk_generics(v, &mut t.generics);
            t.implements.iter_mut().for_each(|x| v.type_expr(x));
            match &mut t.body {
                TypeBody::Record(r) => {
                    walk_fields(v, &mut r.fields);
                    r.methods.iter_mut().for_each(|m| walk_item(v, m));
                    r.invariants.iter_mut().for_each(|x| v.expr(x));
                }
                TypeBody::Union(vs) => vs.iter_mut().for_each(|x| walk_fields(v, &mut x.fields)),
                TypeBody::Alias(t) => v.type_expr(t),
            }
        }
        ItemKind::Interface(i) => {
            walk_generics(v, &mut i.generics);
            i.methods.iter_mut().for_each(|m| walk_item(v, m));
        }
        ItemKind::Extend(e) => {
            v.type_expr(&mut e.target);
            e.implements.iter_mut().for_each(|x| v.type_expr(x));
            e.methods.iter_mut().for_each(|m| walk_item(v, m));
        }
        ItemKind::Capability(c) => c.methods.iter_mut().for_each(|m| walk_item(v, m)),
        ItemKind::Agent(a) => {
            for m in &mut a.members {
                match m {
                    AgentMember::Can(_) => {}
                    AgentMember::Limit { rules, .. } => {
                        rules.iter_mut().for_each(|r| v.expr(&mut r.expr))
                    }
                    AgentMember::Approve {
                        when, by, within, ..
                    } => {
                        v.expr(when);
                        v.expr(by);
                        if let Some(w) = within {
                            v.expr(w);
                        }
                    }
                    AgentMember::On(f) => walk_fn(v, f),
                    AgentMember::Model(e) | AgentMember::Settings(e) => v.expr(e),
                }
            }
        }
        ItemKind::Use(_) => {}
        ItemKind::Test(t) => {
            v.expr(&mut t.name);
            t.with.iter_mut().for_each(|(_, x)| v.expr(x));
            walk_block(v, &mut t.body);
        }
        ItemKind::Stmt(s) => v.stmt(s),
    }
}

fn walk_generics<V: VisitMut + ?Sized>(v: &mut V, generics: &mut [GenericParam]) {
    for g in generics {
        g.bounds.iter_mut().for_each(|b| v.type_expr(b));
    }
}

fn walk_fields<V: VisitMut + ?Sized>(v: &mut V, fields: &mut [Field]) {
    for f in fields {
        v.type_expr(&mut f.ty);
        if let Some(d) = &mut f.default {
            v.expr(d);
        }
    }
}

pub fn walk_fn<V: VisitMut + ?Sized>(v: &mut V, f: &mut FnDecl) {
    walk_generics(v, &mut f.generics);
    for p in &mut f.params {
        if let Some(t) = &mut p.ty {
            v.type_expr(t);
        }
        if let Some(d) = &mut p.default {
            v.expr(d);
        }
    }
    for t in [&mut f.ret, &mut f.errors].into_iter().flatten() {
        v.type_expr(t);
    }
    f.requires
        .iter_mut()
        .chain(f.ensures.iter_mut())
        .chain(f.deprecated.iter_mut().filter_map(|d| d.fix.as_mut()))
        .for_each(|x| v.expr(x));
    for x in &mut f.examples {
        v.expr(&mut x.expr);
        if let Some(e) = &mut x.fails {
            v.expr(e);
        }
    }
    if let Some(b) = &mut f.body {
        walk_block(v, b);
    }
}

pub fn walk_block<V: VisitMut + ?Sized>(v: &mut V, b: &mut Block) {
    b.stmts.iter_mut().for_each(|s| v.stmt(s));
}

pub fn walk_stmt<V: VisitMut + ?Sized>(v: &mut V, s: &mut Stmt) {
    match &mut s.kind {
        StmtKind::Let {
            pattern, ty, value, ..
        } => {
            v.pattern(pattern);
            if let Some(t) = ty {
                v.type_expr(t);
            }
            v.expr(value);
        }
        StmtKind::Assign { target, value, .. } => {
            v.expr(target);
            v.expr(value);
        }
        StmtKind::For {
            pattern,
            iter,
            body,
            ..
        } => {
            v.pattern(pattern);
            v.expr(iter);
            walk_block(v, body);
        }
        StmtKind::While { cond, body, .. } => {
            v.expr(cond);
            walk_block(v, body);
        }
        StmtKind::Assert(e) | StmtKind::Expr(e) => v.expr(e),
    }
}

pub fn walk_pattern<V: VisitMut + ?Sized>(v: &mut V, p: &mut Pattern) {
    match &mut p.kind {
        PatternKind::Literal(e) => v.expr(e),
        PatternKind::Range { lo, hi, .. } => {
            v.expr(lo);
            v.expr(hi);
        }
        PatternKind::Tuple(items) => items.iter_mut().for_each(|x| v.pattern(x)),
        PatternKind::Variant { fields, .. } => {
            for f in fields.iter_mut().flatten() {
                v.pattern(&mut f.pattern);
            }
        }
        PatternKind::Ok(x) | PatternKind::Err(x) => v.pattern(x),
        PatternKind::List { items, .. } => {
            for item in items {
                if let Some(MapKey::Literal(k)) = &mut item.key {
                    v.expr(k);
                }
                v.pattern(&mut item.pattern);
                if let Some(d) = &mut item.default {
                    v.expr(d);
                }
            }
        }
        PatternKind::Record(fields) => {
            for f in fields {
                v.pattern(&mut f.pattern);
                if let Some(d) = &mut f.default {
                    v.expr(d);
                }
            }
        }
        PatternKind::Wildcard | PatternKind::Binding(_) | PatternKind::None => {}
    }
}

pub fn walk_type<V: VisitMut + ?Sized>(v: &mut V, t: &mut TypeExpr) {
    match &mut t.kind {
        TypeKind::Named { args, .. } => args.iter_mut().for_each(|a| v.type_expr(a)),
        TypeKind::List(x) | TypeKind::Optional(x) => v.type_expr(x),
        TypeKind::Map(k, x) => {
            v.type_expr(k);
            v.type_expr(x);
        }
        TypeKind::Tuple(items) => items.iter_mut().for_each(|x| v.type_expr(x)),
        TypeKind::AnonRecord(fields) => fields.iter_mut().for_each(|(_, x)| v.type_expr(x)),
        TypeKind::Fn {
            params,
            ret,
            errors,
            ..
        } => {
            params.iter_mut().for_each(|x| v.type_expr(x));
            for r in ret.iter_mut().chain(errors.iter_mut()) {
                v.type_expr(r);
            }
        }
        TypeKind::Error => {}
    }
}

pub fn walk_expr<V: VisitMut + ?Sized>(v: &mut V, e: &mut Expr) {
    match &mut e.kind {
        ExprKind::Str(parts) => {
            for p in parts {
                if let StrPart::Expr(x) = p {
                    v.expr(x);
                }
            }
        }
        ExprKind::List(items) | ExprKind::Tuple(items) => items.iter_mut().for_each(|x| v.expr(x)),
        ExprKind::AnonRecord(fields) => fields.iter_mut().for_each(|(_, x)| v.expr(x)),
        ExprKind::Map(entries) => {
            for (k, x) in entries {
                v.expr(k);
                v.expr(x);
            }
        }
        ExprKind::Unary(_, x)
        | ExprKind::DotLambda(x)
        | ExprKind::Fail(x)
        | ExprKind::Spawn(_, x, None) => v.expr(x),
        ExprKind::Spawn(_, x, Some(d)) => {
            v.expr(x);
            v.expr(d);
        }
        ExprKind::Binary(_, a, b) => {
            v.expr(a);
            v.expr(b);
        }
        ExprKind::Compare(first, links) => {
            v.expr(first);
            links.iter_mut().for_each(|(_, x)| v.expr(x));
        }
        ExprKind::Call {
            callee,
            generics,
            args,
        } => {
            v.expr(callee);
            generics.iter_mut().for_each(|g| v.type_expr(g));
            args.iter_mut().for_each(|a| v.expr(&mut a.value));
        }
        ExprKind::Field { base, .. } | ExprKind::TupleField { base, .. } => v.expr(base),
        ExprKind::Index { base, index } => {
            v.expr(base);
            v.expr(index);
        }
        ExprKind::With { base, fields } => {
            v.expr(base);
            fields.iter_mut().for_each(|(_, x)| v.expr(x));
        }
        ExprKind::If {
            cond,
            then,
            otherwise,
        } => {
            v.expr(cond);
            walk_block(v, then);
            if let Some(o) = otherwise {
                v.expr(o);
            }
        }
        ExprKind::IfLet {
            value,
            then,
            otherwise,
            ..
        } => {
            v.expr(value);
            walk_block(v, then);
            if let Some(o) = otherwise {
                v.expr(o);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            v.expr(scrutinee);
            for arm in arms {
                v.pattern(&mut arm.pattern);
                if let Some(g) = &mut arm.guard {
                    v.expr(g);
                }
                v.expr(&mut arm.body);
            }
        }
        ExprKind::Block(b) | ExprKind::Arena(b) | ExprKind::Unsafe(b) => walk_block(v, b),
        ExprKind::Lambda { params, body } => {
            for p in params {
                if let Some(t) = &mut p.ty {
                    v.type_expr(t);
                }
            }
            v.expr(body);
        }
        ExprKind::Try { expr, context } => {
            v.expr(expr);
            if let Some(c) = context {
                v.expr(c);
            }
        }
        ExprKind::Catch { expr, handler, .. } => {
            v.expr(expr);
            v.expr(handler);
        }
        ExprKind::Return(x) => {
            if let Some(x) = x {
                v.expr(x);
            }
        }
        ExprKind::Int(_)
        | ExprKind::BigInt(_)
        | ExprKind::Float(_)
        | ExprKind::Char(_)
        | ExprKind::Bool(_)
        | ExprKind::None
        | ExprKind::Name(_)
        | ExprKind::SelfValue
        | ExprKind::It
        | ExprKind::Break(_)
        | ExprKind::Continue(_)
        | ExprKind::Error => {}
    }
}
