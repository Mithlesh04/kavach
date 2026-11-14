//! Patterns, branching (`if`, `if let`, `match`) and match coverage (spec 4).

use crate::checker::{Checker, FieldInfo, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};

impl Checker<'_> {
    /// Gives every name bound by `p` its type.
    pub(crate) fn bind(&mut self, p: &Pattern, ty: &Ty) {
        match &p.kind {
            PatternKind::Wildcard => {}
            PatternKind::Binding(id) => {
                if let Some(&def) = self.decl.get(&id.span.start) {
                    self.locals.insert(def, ty.clone());
                }
            }
            PatternKind::Literal(e) => {
                self.check_expect(e, ty);
            }
            PatternKind::Range { lo, hi, .. } => {
                self.check_expect(lo, ty);
                self.check_expect(hi, ty);
            }
            PatternKind::Tuple(items) => {
                let parts: Vec<Ty> = items.iter().map(|_| self.fresh()).collect();
                if !self.inf.unify(ty, &Ty::Tuple(parts.clone())) {
                    let shown = self.show(ty);
                    self.error(p.span, format!("this tuple pattern can't match `{shown}`"));
                }
                for (item, t) in items.iter().zip(&parts) {
                    self.bind(item, t);
                }
            }
            PatternKind::None => {
                let inner = self.fresh();
                if !self.inf.unify(ty, &Ty::Optional(Box::new(inner))) {
                    let shown = self.show(ty);
                    self.error(
                        p.span,
                        format!(
                            "`none` only matches an optional value, and `{shown}` is never `none`"
                        ),
                    );
                }
            }
            // Only `const` and `let` take lists and records apart
            // (`destructure.rs`); `match` doesn't yet (spec 1.12).
            PatternKind::List { .. } | PatternKind::Record(_) => {
                self.error(
                    p.span,
                    "list and record patterns go in `const` and `let` (spec 1.12)",
                );
            }
            PatternKind::Ok(inner) | PatternKind::Err(inner) => {
                let is_ok = matches!(p.kind, PatternKind::Ok(_));
                match self.inf.shallow(ty) {
                    Ty::Result(ok, err) => self.bind(inner, if is_ok { &ok } else { &err }),
                    Ty::Error => self.bind(inner, &Ty::Error),
                    _ => {
                        self.error(
                            p.span,
                            "`ok(…)` and `err(…)` only match a call that can fail",
                        );
                        self.bind(inner, &Ty::Error);
                    }
                }
            }
            PatternKind::Variant { path, fields } => {
                self.bind_variant(p, path, fields.as_deref(), ty)
            }
        }
    }

    /// The variant named `name` of the union `ty` is, when the bare name
    /// is ambiguous (several unions have it); reports it otherwise.
    fn variant_by_type(&mut self, name: &Ident, ty: &Ty) -> Option<DefId> {
        let candidates = self.res.ambiguous.get(&name.span.start).cloned()?;
        let candidates = self.imported_variants(&candidates);
        if let [only] = candidates[..] {
            return Some(only);
        }
        let want = self.inf.shallow(ty);
        let found = candidates.iter().copied().find(|c| {
            matches!(self.res.defs[*c].kind, DefKind::Variant { of } if matches!(&want, Ty::Adt(d, _) if *d == of))
        });
        if found.is_none() {
            let unions: Vec<String> = candidates
                .iter()
                .filter_map(|c| match self.res.defs[*c].kind {
                    DefKind::Variant { of } => {
                        Some(format!("`{}.{}`", self.def_name(of), name.name))
                    }
                    _ => None,
                })
                .collect();
            self.error(
                name.span,
                format!(
                    "`{}` is a variant of several types: write {}",
                    name.name,
                    unions.join(" or ")
                ),
            );
        }
        found
    }

    /// Of the variants a bare name may be, those of the program's own types
    /// and of the standard modules it imports: a module brought in only
    /// because the program named one of its variants (`std.devices` for
    /// `NotFound`) doesn't make the name ambiguous. All of them when none
    /// is left.
    pub(crate) fn imported_variants(&self, candidates: &[DefId]) -> Vec<DefId> {
        // A module's `use`s count only in the program's own modules: a
        // standard module uses itself (`use std.devices`).
        let in_std = |def: DefId| {
            (self.res.defs[def].span).is_some_and(|s| {
                self.res.modules[self.res.module_at(s.start)]
                    .name
                    .starts_with("std.")
            })
        };
        let imported: Vec<String> = (self.std_modules.iter())
            .filter(|(alias, _)| !in_std(**alias))
            .map(|(_, m)| format!("std.{m}"))
            .collect();
        let kept: Vec<DefId> = (candidates.iter().copied())
            .filter(|c| {
                let DefKind::Variant { of } = self.res.defs[*c].kind else {
                    return true;
                };
                let Some(span) = self.res.defs[of].span else {
                    return true;
                };
                let module = &self.res.modules[self.res.module_at(span.start)].name;
                !module.starts_with("std.") || imported.contains(module)
            })
            .collect();
        if kept.is_empty() {
            candidates.to_vec()
        } else {
            kept
        }
    }

    /// The variant named `name` of the union `ty`, if it has one.
    fn variant_named(&self, name: &str, ty: &Ty) -> Option<DefId> {
        let Ty::Adt(union, _) = self.inf.shallow(ty) else {
            return None;
        };
        match &self.types.get(&union)?.shape {
            Shape::Union(vs) => vs.iter().find(|v| v.name == name).map(|v| v.def),
            _ => None,
        }
    }

    fn bind_variant(&mut self, p: &Pattern, path: &Path, fields: Option<&[FieldPattern]>, ty: &Ty) {
        let last = path.segments.last().unwrap();
        // A bare name is first the matched value's own variant: `Denied`
        // in a match on a `FsError` is `FsError.Denied`, even when another
        // union in scope has a `Denied` too (spec 4.3).
        let own = (path.segments.len() == 1)
            .then(|| self.variant_named(&last.name, ty))
            .flatten();
        let def = match own.or_else(|| self.res.uses.get(&last.span.start).copied()) {
            Some(def) => def,
            // A bare name several unions have: the one of the value matched.
            None => match self.variant_by_type(last, ty) {
                Some(def) => def,
                None => return,
            },
        };
        self.pattern_variants.insert(last.span.start, def);
        let DefKind::Variant { of } = self.res.defs[def].kind else {
            return;
        };
        // An `Error` holds an error of any type: a pattern names the type
        // and matches only an error of it (spec 5.5, decision 0050).
        let any = matches!(self.inf.resolve(ty), Ty::Builtin("Error", _));
        if any && path.segments.len() == 1 {
            let union = self.def_name(of).to_string();
            self.error(
                p.span,
                format!(
                    "an `Error` can hold any error type: write `{union}.{}`",
                    last.name
                ),
            );
        } else if path.segments.len() == 1 {
            self.bare_variant(last, of);
        }
        let (instance, _) = self.instance(of);
        if !any && !self.inf.unify(ty, &instance) {
            let shown = self.show(ty);
            let union = self.def_name(of).to_string();
            self.error(
                p.span,
                format!(
                    "`{}` is a variant of `{union}`, but this value is `{shown}`",
                    last.name
                ),
            );
            return;
        }
        let args = match self.inf.resolve(&instance) {
            Ty::Adt(_, args) => args,
            _ => Vec::new(),
        };
        let subst = self.subst_of(of, &args);
        let infos = self.variant_fields(of, def).unwrap_or_default();
        for (i, fp) in fields.unwrap_or_default().iter().enumerate() {
            let by_name = |name: &str| infos.iter().find(|f| f.name == name);
            let info = match (&fp.name, &fp.pattern.kind) {
                (Some(n), _) => by_name(&n.name),
                (None, PatternKind::Binding(b)) => by_name(&b.name).or(infos.get(i)),
                (None, _) => infos.get(i),
            };
            match info {
                Some(f) => {
                    let t = self.inf.subst(&f.ty, &subst);
                    self.bind(&fp.pattern, &t);
                }
                None => {
                    self.error(
                        fp.pattern.span,
                        format!("`{}` has no such field", last.name),
                    );
                    self.bind(&fp.pattern, &Ty::Error);
                }
            }
        }
    }

    pub(crate) fn variant_fields(&self, union: DefId, variant: DefId) -> Option<Vec<FieldInfo>> {
        match &self.types.get(&union)?.shape {
            Shape::Union(vs) => vs
                .iter()
                .find(|v| v.def == variant)
                .map(|v| v.fields.clone()),
            _ => None,
        }
    }

    /// `if`, `if let` and `match` whose value is used.
    pub(crate) fn branching(&mut self, e: &Expr, expected: Option<&Ty>) -> Ty {
        self.branches(e, expected, true)
    }

    /// `if`, `match` and blocks used as statements.
    pub(crate) fn statement_like(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Block(b) => self.block_stmt(b, None),
            ExprKind::Arena(b) => {
                self.arenas.push(b.span);
                self.block_stmt(b, None);
                self.arenas.pop();
            }
            ExprKind::Unsafe(b) => {
                self.check_unsafe(e.span);
                self.in_unsafe += 1;
                self.block_stmt(b, None);
                self.in_unsafe -= 1;
            }
            ExprKind::Scoped { call, body } => {
                self.check_scoped_call(call);
                self.block_stmt(body, None);
            }
            _ => {
                self.branches(e, None, false);
            }
        }
    }

    /// Checks one branch: a block, or (in `match`) any expression.
    fn branch(&mut self, e: &Expr, expected: Option<&Ty>, want_value: bool) -> Ty {
        match (&e.kind, want_value) {
            (ExprKind::Block(b), false) => {
                self.block_stmt(b, None);
                Ty::Unit
            }
            (_, false) => {
                self.expr_stmt(e);
                Ty::Unit
            }
            (_, true) => match expected {
                Some(t) => self.check_expect(e, t),
                None => self.value(e, None),
            },
        }
    }

    fn block_branch(&mut self, b: &Block, expected: Option<&Ty>, want_value: bool) -> Ty {
        if want_value {
            self.block_value(b, expected)
        } else {
            self.block_stmt(b, None);
            Ty::Unit
        }
    }

    fn branches(&mut self, e: &Expr, expected: Option<&Ty>, want_value: bool) -> Ty {
        let mut types = Vec::new();
        match &e.kind {
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.condition(cond);
                types.push(self.block_branch(then, expected, want_value));
                match otherwise {
                    Some(o) => types.push(self.branch(o, expected, want_value)),
                    None => types.push(Ty::Unit),
                }
            }
            ExprKind::IfLet {
                name,
                value,
                then,
                otherwise,
            } => {
                let t = self.value(value, None);
                let inner = match self.inf.shallow(&t) {
                    Ty::Optional(inner) => *inner,
                    Ty::Error | Ty::Var(_) => Ty::Error,
                    other => {
                        let shown = self.show(&other);
                        self.error(
                            value.span,
                            format!("`if let` needs an optional value, but this is `{shown}`"),
                        );
                        Ty::Error
                    }
                };
                if let Some(&def) = self.decl.get(&name.span.start) {
                    self.locals.insert(def, inner);
                }
                types.push(self.block_branch(then, expected, want_value));
                match otherwise {
                    Some(o) => types.push(self.branch(o, expected, want_value)),
                    None => types.push(Ty::Unit),
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                let t = self.infer(scrutinee, None);
                for arm in arms {
                    self.bind(&arm.pattern, &t);
                    if let Some(g) = &arm.guard {
                        self.check_expect(g, &Ty::Prim(Prim::Bool));
                    }
                    types.push(self.branch(&arm.body, expected, want_value));
                }
                self.check_coverage(arms, &t, e.span);
            }
            _ => return Ty::Error,
        }
        if !want_value {
            return Ty::Unit;
        }
        if let Some(t) = expected {
            return t.clone();
        }
        // Without an expected type, the branches must agree.
        let mut result = Ty::Never;
        for t in types {
            if matches!(self.inf.shallow(&result), Ty::Never) {
                result = t;
            } else if !self.coerce(&t, &result) {
                let (a, b) = (self.show(&result), self.show(&t));
                self.error(
                    e.span,
                    format!("the branches give different types: `{a}` and `{b}`"),
                );
                return Ty::Error;
            }
        }
        result
    }

    /// A `match` must cover every possible case (spec 4.3).
    fn check_coverage(&mut self, arms: &[MatchArm], ty: &Ty, span: Span) {
        let open: Vec<&Pattern> = arms
            .iter()
            .filter(|a| a.guard.is_none())
            .map(|a| &a.pattern)
            .collect();
        if open.iter().any(|p| irrefutable(p)) {
            return;
        }
        if matches!(self.inf.resolve(ty), Ty::Builtin("Error", _)) {
            self.error(
                span,
                "a `match` on an `Error` needs a `_ => …` arm: an `Error` can hold any error type",
            );
            return;
        }
        let named = &self.pattern_variants;
        let covers = |p: &Pattern, variant: DefId| match &p.kind {
            PatternKind::Variant { path, fields } => {
                named.get(&path.segments.last().unwrap().span.start) == Some(&variant)
                    && fields.iter().flatten().all(|f| irrefutable(&f.pattern))
            }
            _ => false,
        };
        let union_missing = |def: DefId, pats: &[&Pattern]| -> Vec<String> {
            match self.types.get(&def).map(|t| &t.shape) {
                // Named as a case is written: with the union (decision 0070).
                Some(Shape::Union(vs)) => vs
                    .iter()
                    .filter(|v| !pats.iter().any(|p| covers(p, v.def)))
                    .map(|v| format!("{}.{}", self.def_name(def), v.name))
                    .collect(),
                _ => vec!["every other value (add `_ => …`)".to_string()],
            }
        };
        let missing: Vec<String> = match self.inf.resolve(ty) {
            Ty::Adt(def, _) => union_missing(def, &open),
            Ty::Prim(Prim::Bool) => ["true", "false"]
                .into_iter()
                .filter(|b| !open.iter().any(|p| matches!(&p.kind, PatternKind::Literal(Expr { kind: ExprKind::Bool(v), .. }) if v.to_string() == *b)))
                .map(String::from)
                .collect(),
            Ty::Optional(_) => {
                let has_none = open.iter().any(|p| matches!(p.kind, PatternKind::None));
                if has_none { vec!["a present value (`value => …`)".to_string()] } else { vec!["none".to_string()] }
            }
            Ty::Result(_, err) => {
                let mut missing = Vec::new();
                if !open.iter().any(|p| matches!(&p.kind, PatternKind::Ok(i) if irrefutable(i))) {
                    missing.push("ok(…)".to_string());
                }
                let errs: Vec<&Pattern> = open.iter().filter_map(|p| match &p.kind { PatternKind::Err(i) => Some(&**i), _ => None }).collect();
                if !errs.iter().any(|p| irrefutable(p)) {
                    match self.inf.resolve(&err) {
                        Ty::Adt(def, _) => missing.extend(union_missing(def, &errs).into_iter().map(|v| format!("err({v})"))),
                        _ => missing.push("err(…)".to_string()),
                    }
                }
                missing
            }
            Ty::Error | Ty::Var(_) => Vec::new(),
            _ => vec!["every other value (add `_ => …`)".to_string()],
        };
        if !missing.is_empty() {
            let list = missing
                .iter()
                .map(|m| format!("`{m}`"))
                .collect::<Vec<_>>()
                .join(", ");
            self.error(span, format!("this `match` does not cover {list}"));
        }
    }
}

/// A pattern that matches every value.
fn irrefutable(p: &Pattern) -> bool {
    match &p.kind {
        PatternKind::Wildcard | PatternKind::Binding(_) => true,
        PatternKind::Tuple(items) => items.iter().all(irrefutable),
        _ => false,
    }
}
