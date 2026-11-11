//! Field access, indexing and `with`, plus two safety rules about values:
//! secrets (spec 2.11) and arenas (spec 6.3).

use crate::checker::{Checker, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::*;

/// Methods that store their argument inside the receiver (spec 6.3).
const STORING: &[&str] = &["push", "insert", "put", "append", "set", "update"];

impl Checker<'_> {
    pub(crate) fn field(&mut self, e: &Expr, base: &Expr, name: &Ident, optional: bool) -> Ty {
        // `Door.Open`: a variant named through its union (spec 2.6).
        if let ExprKind::Name(id) = &base.kind
            && let Some(&def) = self.res.uses.get(&id.span.start)
        {
            // `math.pi`: a constant of a standard module (spec 14.5).
            if self.std_modules.contains_key(&def) {
                return match self.std_constant(def, &name.name) {
                    Some(t) => t,
                    None => {
                        self.error(
                            name.span,
                            format!("`{}` has no constant `{}`", id.name, name.name),
                        );
                        Ty::Error
                    }
                };
            }
            if let Some(Shape::Union(vs)) = self.types.get(&def).map(|t| &t.shape)
                && vs.iter().any(|v| v.name == name.name)
            {
                return self.instance(def).0;
            }
        }
        let t = self.value(base, None);
        let resolved = self.inf.resolve(&t);
        let literal = matches!(self.inf.shallow(&resolved), Ty::Var(v)
            if matches!(self.inf.kind(v), crate::infer::VarKind::IntLiteral));
        let whole = literal || resolved == Ty::Prim(crate::Prim::Int);
        if whole && !optional {
            // `5.seconds`: a whole number with a unit is a `Duration`; it
            // has no other fields, so `2.months` is an error (spec 14.4).
            if !crate::DURATION_UNITS.iter().any(|(u, _)| *u == name.name) {
                self.not_a_unit(base, name);
                return Ty::Error;
            }
            self.inf.unify(&t, &Ty::Prim(crate::Prim::Int));
            let duration = self.builtin_ty("Duration");
            if duration == Ty::Error {
                self.error(
                    name.span,
                    format!(
                        "a number of `{}` is a `Duration`, from `std.time`: add `use std.time`",
                        name.name
                    ),
                );
            }
            return duration;
        }
        let (inner, wrap) = match self.inf.resolve(&t) {
            Ty::Optional(inner) if optional => (*inner, true),
            Ty::Optional(_) => {
                self.error(
                    e.span,
                    format!(
                        "`{}` might be `none`: use `?.{}` or `if let`",
                        self.show(&t),
                        name.name
                    ),
                );
                return Ty::Error;
            }
            other => (other, false),
        };
        let found = match &inner {
            Ty::Adt(def, args) => match self.types.get(def).map(|t| t.shape.clone()) {
                Some(Shape::Record(fields)) => {
                    let found = fields.iter().find(|f| f.name == name.name).cloned();
                    if let Some(f) = &found {
                        self.check_field_access(*def, f, name);
                    }
                    found.map(|f| {
                        let subst = self.subst_of(*def, args);
                        (self.inf.subst(&f.ty, &subst), f.secret)
                    })
                }
                _ => None,
            },
            Ty::Anon(fs) => fs
                .iter()
                .find(|(n, _)| *n == name.name)
                .map(|(_, ty)| (ty.clone(), false)),
            Ty::Error | Ty::Var(_) => return Ty::Error,
            _ => None,
        };
        match found {
            Some((ty, secret)) => {
                if secret {
                    self.secret.insert((e.span.start, e.span.end));
                }
                if wrap { Ty::Optional(Box::new(ty)) } else { ty }
            }
            None => {
                let shown = self.show(&inner);
                self.error(name.span, format!("`{shown}` has no field `{}`", name.name));
                Ty::Error
            }
        }
    }

    pub(crate) fn index(&mut self, base: &Expr, index: &Expr) -> Ty {
        let t = self.value(base, None);
        match self.inf.resolve(&t) {
            Ty::List(item) => {
                self.check_expect(index, &Ty::Prim(Prim::Int));
                *item
            }
            Ty::Prim(Prim::Bytes) => {
                self.check_expect(index, &Ty::Prim(Prim::Int));
                Ty::Prim(Prim::UInt8)
            }
            Ty::Builtin("Ptr", args) => self.ptr_item(&args, index, base.span),
            Ty::Map(k, v) => {
                self.check_expect(index, &k);
                Ty::Optional(v)
            }
            // `j["key"]` and `j[i]`: `Null` when there is nothing there
            // (spec 2.12).
            json if json == self.builtin_ty("Json") => {
                let it = self.value(index, None);
                match self.inf.resolve(&it) {
                    Ty::Prim(Prim::String) | Ty::Error => {}
                    _ if self.inf.unify(&it, &Ty::Prim(Prim::Int)) => {}
                    other => {
                        let shown = self.show(&other);
                        self.error(
                            index.span,
                            format!("a `Json` is indexed with text or an `Int`, not `{shown}`"),
                        );
                    }
                }
                json
            }
            Ty::Error | Ty::Var(_) => Ty::Error,
            other => {
                let shown = self.show(&other);
                self.error(base.span, format!("`{shown}` can't be indexed with `[…]`"));
                Ty::Error
            }
        }
    }

    pub(crate) fn with(&mut self, base: &Expr, fields: &[(Ident, Expr)]) -> Ty {
        let t = self.value(base, None);
        let Ty::Adt(def, args) = self.inf.resolve(&t) else {
            if !matches!(self.inf.resolve(&t), Ty::Error | Ty::Var(_)) {
                self.error(base.span, "`with` needs a record value");
            }
            return Ty::Error;
        };
        let Some(Shape::Record(infos)) = self.types.get(&def).map(|t| t.shape.clone()) else {
            return Ty::Error;
        };
        let subst = self.subst_of(def, &args);
        for (name, value) in fields {
            match infos.iter().find(|f| f.name == name.name) {
                Some(f) => {
                    self.check_field_access(def, f, name);
                    let ft = self.inf.subst(&f.ty, &subst);
                    self.check_expect(value, &ft);
                }
                None => {
                    let owner = self.def_name(def);
                    let message = gone_field(owner, &name.name).map_or_else(
                        || format!("`{owner}` has no field `{}`", name.name),
                        String::from,
                    );
                    self.error(name.span, message);
                    self.value(value, None);
                }
            }
        }
        t
    }

    /// Whether `e` reads a secret field, directly or inside a string (spec 2.11).
    pub(crate) fn check_secret(&mut self, e: &Expr, into: &str) {
        let mut hits = Vec::new();
        self.find_secrets(e, &mut hits);
        for (span, name) in hits {
            let into = if into == "text" {
                "text".to_string()
            } else {
                format!("`{into}`")
            };
            self.error(span, format!("`{name}` is secret, so it can't go into {into}; use `reveal(…)` if you really mean to"));
        }
    }

    /// Whether `e`'s value is a secret: a secret field, a secret parameter,
    /// `env(…)`, or a variable set from one (spec 8.7, 10.13).
    pub(crate) fn is_secret(&self, e: &Expr) -> bool {
        let mut hits = Vec::new();
        self.find_secrets(e, &mut hits);
        !hits.is_empty()
    }

    /// Passing a secret where nothing may receive it: an argument of a
    /// parameter, or a field, that isn't marked `secret` (spec 8.7).
    pub(crate) fn check_secret_arg(&mut self, e: &Expr, what: &str, name: &str, owner: &str) {
        let mut hits = Vec::new();
        self.find_secrets(e, &mut hits);
        for (span, secret) in hits {
            self.error(
                span,
                format!("`{secret}` is secret, so it can't go into the {what} `{name}` of {owner}, which isn't `secret`; mark it `secret`, or use `reveal(…)` if you really mean to"),
            );
        }
    }

    fn find_secrets(&self, e: &Expr, out: &mut Vec<(Span, String)>) {
        match &e.kind {
            ExprKind::Field { name, .. } if self.secret.contains(&(e.span.start, e.span.end)) => {
                out.push((e.span, name.name.clone()))
            }
            ExprKind::Name(id)
                if self
                    .res
                    .uses
                    .get(&id.span.start)
                    .is_some_and(|d| self.secret_defs.contains(d)) =>
            {
                out.push((e.span, id.name.clone()))
            }
            // `env(…)` (spec 10.13).
            ExprKind::Call { .. } if self.secret.contains(&(e.span.start, e.span.end)) => {
                out.push((e.span, "env(…)".to_string()))
            }
            ExprKind::Binary(_, l, r) => {
                self.find_secrets(l, out);
                self.find_secrets(r, out);
            }
            ExprKind::Try { expr, .. } => self.find_secrets(expr, out),
            ExprKind::Catch { expr, handler, .. } => {
                self.find_secrets(expr, out);
                self.find_secrets(handler, out);
            }
            _ => {}
        }
    }

    /// Inside an `arena`, storing a value made there into something from
    /// outside (`outside.push(value)`) is an error (spec 6.3).
    pub(crate) fn check_arena_call(&mut self, base: &Expr, method: &Ident, args: &[Arg]) {
        if STORING.contains(&method.name.as_str()) {
            for a in args {
                self.check_arena_store(base, &a.value);
            }
        }
    }

    pub(crate) fn check_arena_store(&mut self, target: &Expr, value: &Expr) {
        let Some(&arena) = self.arenas.last() else {
            return;
        };
        let inside = |c: &Self, e: &Expr| -> Option<bool> {
            let mut root = e;
            while let ExprKind::Field { base, .. }
            | ExprKind::Index { base, .. }
            | ExprKind::TupleField { base, .. } = &root.kind
            {
                root = base;
            }
            let ExprKind::Name(id) = &root.kind else {
                return None;
            };
            let def = c.res.uses.get(&id.span.start)?;
            let span = c.res.defs[*def].span?;
            Some(span.start >= arena.start && span.start < arena.end)
        };
        if inside(self, target) == Some(false) && inside(self, value) == Some(true) {
            self.error(value.span, "this value was made inside the `arena` and can't be stored outside it: return it as the arena's value instead");
        }
    }
}

impl Checker<'_> {
    /// `2.months`: a whole number takes only the units of a `Duration`.
    fn not_a_unit(&mut self, base: &Expr, name: &Ident) {
        let units: Vec<String> = crate::DURATION_UNITS
            .iter()
            .map(|(u, _)| format!("`{u}`"))
            .collect();
        let why = match name.name.as_str() {
            "month" | "months" | "year" | "years" => {
                " Months and years have no fixed length: count days instead."
            }
            "week" | "weeks" => " For weeks, write `(7 * n).days`.",
            _ => "",
        };
        let what = match &base.kind {
            ExprKind::Int(n) => format!("`{n}` has"),
            ExprKind::Name(id) => format!("`{}` is an `Int`, which has", id.name),
            _ => "an `Int` has".to_string(),
        };
        self.error(
            name.span,
            format!(
                "{what} no field `{}`. The units that make a `Duration` are {}.{why}",
                name.name,
                units.join(", ")
            ),
        );
    }

    /// A field that isn't `pub` can be used only inside the module that
    /// declares its type (spec 10.1).
    pub(crate) fn check_field_access(
        &mut self,
        def: kavach_resolver::DefId,
        f: &crate::checker::FieldInfo,
        name: &Ident,
    ) {
        let Some(decl) = self.res.defs[def].span else {
            return;
        };
        if f.is_pub || self.res.module_at(decl.start) == self.res.module_at(name.span.start) {
            return;
        }
        let module = &self.res.modules[self.res.module_at(decl.start)].name;
        self.error(
            name.span,
            format!(
                "the field `{}` of `{}` is private to module `{module}`: mark it `pub` there to use it here (spec 10.1)",
                f.name,
                self.def_name(def)
            ),
        );
    }

    /// A record with private fields can be created only in its own module;
    /// others call a `pub fn` there that makes one (spec 10.1).
    pub(crate) fn check_construct_access(
        &mut self,
        def: kavach_resolver::DefId,
        fields: &[crate::checker::FieldInfo],
        span: kavach_ast::Span,
        name: &str,
    ) {
        let Some(decl) = self.res.defs[def].span else {
            return;
        };
        let Some(private) = fields.iter().find(|f| !f.is_pub) else {
            return;
        };
        if self.res.module_at(decl.start) == self.res.module_at(span.start) {
            return;
        }
        let module = &self.res.modules[self.res.module_at(decl.start)].name;
        self.error(
            span,
            format!(
                "`{name}` has a private field (`{}`), so only module `{module}` can create one: add a `pub fn` there that makes it (spec 10.1)",
                private.name
            ),
        );
    }
}

/// What to write instead of a field a standard type no longer has.
pub(crate) fn gone_field(owner: &str, field: &str) -> Option<&'static str> {
    match (owner.trim_matches('`'), field) {
        ("Model", "temperature") => Some(
            "`Model` has no `temperature`: current models take only their own, and newer API versions refuse it. Use `effort` (`Low` to `Max`) to trade quality against tokens (spec 8.4)",
        ),
        _ => None,
    }
}
