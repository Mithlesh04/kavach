//! Statements, expressions, patterns and types inside bodies.

use crate::DefKind;
use crate::resolver::Resolver;
use kavach_ast::*;

impl Resolver {
    pub(crate) fn block(&mut self, block: &Block) {
        self.stmts(&block.stmts);
    }

    /// A list of statements in its own scope. Each `let` is visible only to
    /// the statements after it (spec 1.4).
    pub(crate) fn stmts(&mut self, stmts: &[Stmt]) {
        self.push();
        let mut later = Vec::new();
        for s in stmts {
            if let StmtKind::Let { pattern, .. } = &s.kind {
                binding_names(pattern, &mut later);
            }
        }
        self.push_later(later);
        for s in stmts {
            self.stmt(s);
        }
        self.pop_later();
        self.pop();
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let {
                mutable,
                pattern,
                ty,
                value,
            } => {
                self.expr(value);
                if let Some(ty) = ty {
                    self.type_expr(ty);
                }
                self.pattern(pattern, *mutable);
            }
            StmtKind::Assign { target, value, .. } => {
                self.expr(target);
                self.expr(value);
                self.check_assignable(target);
            }
            StmtKind::For {
                pattern,
                iter,
                body,
                ..
            } => {
                self.expr(iter);
                self.push();
                self.pattern(pattern, false);
                self.block(body);
                self.pop();
            }
            StmtKind::While { cond, body, .. } => {
                self.expr(cond);
                self.block(body);
            }
            StmtKind::Assert(e) | StmtKind::Expr(e) => self.expr(e),
        }
    }

    /// Only `let` bindings can change (spec 1.4, 2.4, 3.1).
    fn check_assignable(&mut self, target: &Expr) {
        // `p[i] = v` writes through a pointer, not to `p`: inside `unsafe`
        // the checker, which knows the types, decides (spec 6.4).
        if self.in_unsafe > 0 && matches!(target.kind, ExprKind::Index { .. }) {
            return;
        }
        let mut root = target;
        while let ExprKind::Field { base, .. }
        | ExprKind::TupleField { base, .. }
        | ExprKind::Index { base, .. } = &root.kind
        {
            root = base;
        }
        let message = match &root.kind {
            ExprKind::SelfValue => {
                "`self` can't change: return an updated copy with `with` instead".to_string()
            }
            ExprKind::Name(ident) => match self.res.def_at(ident.span.start).map(|d| d.kind) {
                Some(DefKind::Local { mutable: true }) | None => return,
                Some(DefKind::Local { mutable: false }) => {
                    format!(
                        "`{}` is a `const`, so it can't change: declare it with `let`",
                        ident.name
                    )
                }
                Some(DefKind::Param) => format!(
                    "`{}` is a parameter, so it can't change: copy it into a `let` first",
                    ident.name
                ),
                Some(_) => format!("`{}` can't be assigned to", ident.name),
            },
            _ => return,
        };
        self.error(root.span, message);
    }

    pub(crate) fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Name(ident) => self.use_name(ident),
            ExprKind::Str(parts) => {
                for part in parts {
                    if let StrPart::Expr(x) = part {
                        self.expr(x);
                    }
                }
            }
            ExprKind::AnonRecord(fields) => {
                for (_, x) in fields {
                    self.expr(x);
                }
            }
            ExprKind::List(items) | ExprKind::Tuple(items) => {
                items.iter().for_each(|x| self.expr(x))
            }
            ExprKind::Map(entries) => {
                for (k, v) in entries {
                    self.expr(k);
                    self.expr(v);
                }
            }
            // `sql"…"` or `pg.sql"…"` (spec 10.15).
            ExprKind::Tagged { tag, .. } => match tag.segments.as_slice() {
                [name] => self.use_name(name),
                [module, name] => {
                    let base = Expr {
                        kind: ExprKind::Name(module.clone()),
                        span: module.span,
                    };
                    match self.module_of(&base) {
                        Some(m) => {
                            self.member(m, name);
                        }
                        None => self.use_name(module),
                    }
                }
                _ => {}
            },
            // `orders.total`: a name in another module (spec 10.2).
            ExprKind::Field { base, name, .. } if self.module_of(base).is_some() => {
                let m = self.module_of(base).unwrap_or(0);
                self.member(m, name);
            }
            ExprKind::Unary(_, x)
            | ExprKind::Field { base: x, .. }
            | ExprKind::TupleField { base: x, .. }
            | ExprKind::DotLambda(x)
            | ExprKind::Fail(x)
            | ExprKind::Spawn(_, x, None)
            | ExprKind::Return(Some(x)) => self.expr(x),
            ExprKind::Spawn(_, x, Some(d)) => {
                self.expr(x);
                self.expr(d);
            }
            ExprKind::Binary(_, l, r) | ExprKind::Index { base: l, index: r } => {
                self.expr(l);
                self.expr(r);
            }
            ExprKind::Compare(first, links) => {
                self.expr(first);
                links.iter().for_each(|(_, x)| self.expr(x));
            }
            ExprKind::Call {
                callee,
                generics,
                args,
            } => {
                self.expr(callee);
                generics.iter().for_each(|t| self.type_expr(t));
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::With { base, fields } => {
                self.expr(base);
                fields.iter().for_each(|(_, v)| self.expr(v));
            }
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond);
                self.block(then);
                if let Some(o) = otherwise {
                    self.expr(o);
                }
            }
            ExprKind::IfLet {
                name,
                value,
                then,
                otherwise,
            } => {
                self.expr(value);
                self.push();
                self.check_snake(name, "variables");
                self.define(name, DefKind::Local { mutable: false });
                self.block(then);
                self.pop();
                if let Some(o) = otherwise {
                    self.expr(o);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.expr(scrutinee);
                for arm in arms {
                    self.push();
                    self.pattern(&arm.pattern, false);
                    if let Some(g) = &arm.guard {
                        self.expr(g);
                    }
                    self.expr(&arm.body);
                    self.pop();
                }
            }
            ExprKind::Unsafe(b) => {
                self.in_unsafe += 1;
                self.block(b);
                self.in_unsafe -= 1;
            }
            ExprKind::Block(b) | ExprKind::Arena(b) => self.block(b),
            ExprKind::Lambda { params, body } => {
                self.push();
                for p in params {
                    if let Some(ty) = &p.ty {
                        self.type_expr(ty);
                    }
                    self.check_snake(&p.name, "parameters");
                    self.define(&p.name, DefKind::Param);
                }
                self.expr(body);
                self.pop();
            }
            ExprKind::Try { expr, context } => {
                self.expr(expr);
                if let Some(c) = context {
                    self.expr(c);
                }
            }
            ExprKind::Catch {
                expr,
                binding,
                handler,
            } => {
                self.expr(expr);
                self.push();
                if let Some(b) = binding {
                    self.define(b, DefKind::Local { mutable: false });
                }
                self.expr(handler);
                self.pop();
            }
            ExprKind::Int(_)
            | ExprKind::BigInt(_)
            | ExprKind::Float(_)
            | ExprKind::Char(_)
            | ExprKind::Bool(_)
            | ExprKind::None
            | ExprKind::SelfValue
            | ExprKind::It
            | ExprKind::Return(None)
            | ExprKind::Break(_)
            | ExprKind::Continue(_)
            | ExprKind::Error => {}
        }
    }

    pub(crate) fn pattern(&mut self, p: &Pattern, mutable: bool) {
        match &p.kind {
            PatternKind::Binding(ident) => {
                self.check_snake(ident, "variables");
                self.define(ident, DefKind::Local { mutable });
            }
            PatternKind::Literal(e) => self.expr(e),
            PatternKind::Range { lo, hi, .. } => {
                self.expr(lo);
                self.expr(hi);
            }
            PatternKind::Tuple(items) => items.iter().for_each(|x| self.pattern(x, mutable)),
            PatternKind::Variant { path, fields } => {
                self.variant(path);
                for f in fields.iter().flatten() {
                    self.pattern(&f.pattern, mutable);
                }
            }
            PatternKind::Ok(inner) | PatternKind::Err(inner) => self.pattern(inner, mutable),
            PatternKind::List { items, rest } => {
                // Defaults are read before the names exist.
                for item in items {
                    if let Some(kavach_ast::MapKey::Literal(k)) = &item.key {
                        self.expr(k);
                    }
                    if let Some(d) = &item.default {
                        self.expr(d);
                    }
                    self.pattern(&item.pattern, mutable);
                }
                if let Some(r) = rest {
                    self.check_snake(r, "variables");
                    self.define(r, DefKind::Local { mutable });
                }
            }
            PatternKind::Record(fields) => {
                for f in fields {
                    if let Some(d) = &f.default {
                        self.expr(d);
                    }
                    self.pattern(&f.pattern, mutable);
                }
            }
            PatternKind::Wildcard | PatternKind::None => {}
        }
    }

    /// `Paid`, or `Shape.Circle` when the bare name is ambiguous.
    fn variant(&mut self, path: &Path) {
        let first = &path.segments[0];
        if let [_, name, ..] = path.segments.as_slice() {
            // `fs.NotFound`: a variant of a union in another module.
            if let Some(m) = self.lookup(&first.name).and_then(|d| {
                self.res.uses.insert(first.span.start, d);
                self.res.module_defs.get(&d).copied()
            }) {
                let found = self.modules.own[m]
                    .values()
                    .find_map(|u| self.union_variants.get(u)?.get(&name.name).copied());
                match found {
                    Some(id) => {
                        self.res.uses.insert(name.span.start, id);
                    }
                    None => self.error(
                        name.span,
                        format!("module `{}` has no variant `{}`", first.name, name.name),
                    ),
                }
                return;
            }
            self.use_type(first);
            let union = self.res.uses.get(&first.span.start).copied();
            match union.and_then(|u| self.union_variants.get(&u)?.get(&name.name).copied()) {
                Some(id) => {
                    self.res.uses.insert(name.span.start, id);
                }
                None if union.is_some() => self.error(
                    name.span,
                    format!("`{}` has no variant `{}`", first.name, name.name),
                ),
                None => {}
            }
        } else if self.variants.contains_key(&first.name) {
            self.use_name(first);
        } else if matches!(
            self.lookup(&first.name).map(|id| self.res.defs[id].kind),
            Some(DefKind::Type | DefKind::Builtin | DefKind::Import)
        ) {
            // A union of other types: `type ProfileError = FileError | ParseError` (spec 5.2).
            self.use_name(first);
        } else {
            self.unknown(first, "is not a variant of any type");
        }
    }

    pub(crate) fn type_expr(&mut self, t: &TypeExpr) {
        match &t.kind {
            TypeKind::Named { path, args } => {
                // `orders.Order`: a type in another module (spec 10.2).
                let module = match path.segments.as_slice() {
                    [first, _, ..] => self
                        .lookup(&first.name)
                        .and_then(|d| self.res.module_defs.get(&d).copied()),
                    _ => None,
                };
                match module {
                    Some(m) => {
                        let first = &path.segments[0];
                        if let Some(d) = self.lookup(&first.name) {
                            self.res.uses.insert(first.span.start, d);
                        }
                        let last = &path.segments[path.segments.len() - 1];
                        // A built-in type named through its standard
                        // module, `regex.Regex`: the module's file
                        // doesn't declare it, the language does.
                        if self.member(m, last).is_none()
                            && let Some(d) = self.lookup(&last.name)
                            && self.res.defs[d].kind == DefKind::Builtin
                        {
                            self.res.uses.insert(last.span.start, d);
                        }
                    }
                    None => {
                        self.use_type(&path.segments[0]);
                        // `regex.Regex`: a module the runtime provides
                        // whole, with no file of its own.
                        if let [_, .., last] = path.segments.as_slice()
                            && let Some(d) = self.lookup(&last.name)
                            && self.res.defs[d].kind == DefKind::Builtin
                        {
                            self.res.uses.insert(last.span.start, d);
                        }
                    }
                }
                args.iter().for_each(|a| self.type_expr(a));
            }
            TypeKind::List(inner) | TypeKind::Optional(inner) => self.type_expr(inner),
            TypeKind::Map(k, v) => {
                self.type_expr(k);
                self.type_expr(v);
            }
            TypeKind::Tuple(items) => items.iter().for_each(|x| self.type_expr(x)),
            TypeKind::AnonRecord(fields) => fields.iter().for_each(|(_, x)| self.type_expr(x)),
            TypeKind::Fn {
                params,
                ret,
                errors,
                uses,
            } => {
                params.iter().for_each(|x| self.type_expr(x));
                for r in ret.iter().chain(errors) {
                    self.type_expr(r);
                }
                for path in uses.iter().flatten() {
                    self.use_type(&path.segments[0]);
                }
            }
            TypeKind::Error => {}
        }
    }
}

fn binding_names(p: &Pattern, out: &mut Vec<String>) {
    match &p.kind {
        PatternKind::Binding(ident) => out.push(ident.name.clone()),
        PatternKind::Tuple(items) => items.iter().for_each(|x| binding_names(x, out)),
        PatternKind::List { items, rest } => {
            items.iter().for_each(|x| binding_names(&x.pattern, out));
            out.extend(rest.iter().map(|r| r.name.clone()));
        }
        PatternKind::Record(fields) => fields.iter().for_each(|f| binding_names(&f.pattern, out)),
        _ => {}
    }
}

impl Resolver {
    /// The module `e` names, if it is a name brought in by `use orders`.
    fn module_of(&mut self, e: &Expr) -> Option<usize> {
        let ExprKind::Name(id) = &e.kind else {
            return None;
        };
        let def = self.lookup(&id.name)?;
        let m = self.res.module_defs.get(&def).copied()?;
        self.res.uses.insert(id.span.start, def);
        Some(m)
    }
}
