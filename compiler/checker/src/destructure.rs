//! Destructuring in `const` and `let` (spec 1.12, decision 0037): lists
//! by position, maps by key, records by field name, patterns nested in
//! each other. A part that may be missing (a list item past the end, a
//! map value, a `T?` field) makes every name in it optional, unless the
//! name has a default; a list literal shows how many items it has.

use crate::checker::{Checker, Shape};
use crate::ty::Ty;
use kavach_ast::*;

impl Checker<'_> {
    /// Gives the names of list or record pattern `p` their types, `ty`
    /// being the type of `value`. False when `p` is neither.
    pub(crate) fn destructure(&mut self, p: &Pattern, ty: &Ty, value: &Expr) -> bool {
        if !matches!(p.kind, PatternKind::List { .. } | PatternKind::Record(_)) {
            return false;
        }
        let known = match &value.kind {
            ExprKind::List(xs) => xs.len(),
            _ => 0,
        };
        self.take_apart(p, ty, false, known);
        true
    }

    /// Takes a value of type `ty` apart with list or record pattern `p`;
    /// `missing` says the value itself may not be there.
    fn take_apart(&mut self, p: &Pattern, ty: &Ty, missing: bool, known: usize) {
        match &p.kind {
            PatternKind::List { items, rest } => match self.inf.resolve(ty) {
                Ty::Map(k, v) => self.map_pattern(items, rest.as_ref(), &k, &v, missing),
                _ => self.list_pattern(p, items, rest.as_ref(), ty, missing, known),
            },
            PatternKind::Record(fields) => self.record_pattern(p, fields, ty, missing),
            _ => {}
        }
    }

    /// One part, of type `ty`, for sub-pattern `pat` (a name, `_`, or a
    /// nested pattern), with its default if it has one. `missing`: the
    /// part may not be there.
    fn take_part(
        &mut self,
        pat: &Pattern,
        ty: &Ty,
        default: Option<&Expr>,
        missing: bool,
        secret: bool,
    ) {
        let nested = matches!(pat.kind, PatternKind::List { .. } | PatternKind::Record(_));
        // A `T?` part with a default or a nested pattern: its `T`, which
        // may be missing.
        let (ty, missing) = match self.inf.resolve(ty) {
            Ty::Optional(inner) if default.is_some() || nested => (*inner, true),
            t => (t, missing),
        };
        if let Some(d) = default {
            self.check_expect(d, &ty);
        }
        match &pat.kind {
            PatternKind::Binding(id) => {
                let t = if missing && default.is_none() {
                    optional(ty)
                } else {
                    ty
                };
                if let Some(&def) = self.decl.get(&id.span.start) {
                    self.locals.insert(def, t);
                    // Taken from a secret field, it is a secret (spec 8.7).
                    if secret {
                        self.secret_defs.insert(def);
                    }
                }
            }
            PatternKind::Wildcard => {}
            // A default gives the whole part, so then nothing inside is missing.
            _ if nested => self.take_apart(pat, &ty, missing && default.is_none(), 0),
            _ => self.error(
                pat.span,
                "a part is taken by a name, `_`, or a list, map or record pattern (spec 1.12)",
            ),
        }
    }

    fn list_pattern(
        &mut self,
        p: &Pattern,
        items: &[ListItem],
        rest: Option<&Ident>,
        ty: &Ty,
        missing: bool,
        known: usize,
    ) {
        let item = self.fresh();
        if !self.inf.unify(ty, &Ty::List(Box::new(item.clone()))) {
            let shown = self.show(ty);
            self.error(
                p.span,
                format!("a list pattern takes a list apart, and this is `{shown}`"),
            );
        }
        for (i, it) in items.iter().enumerate() {
            if let Some(k) = &it.key {
                let at = match k {
                    MapKey::Name(n) => n.span,
                    MapKey::Literal(e) => e.span,
                };
                self.error(at, "a key goes in a map pattern, and this is a list");
            }
            self.take_part(
                &it.pattern,
                &item,
                it.default.as_ref(),
                missing || i >= known,
                false,
            );
        }
        if let Some(r) = rest
            && let Some(&def) = self.decl.get(&r.span.start)
        {
            self.locals.insert(def, Ty::List(Box::new(item)));
        }
    }

    /// `const ["asha": a, port = 8080] = m` on a map from `k` to `v`: a
    /// key may be missing, so a name is a `v?`, or a `v` with a default.
    /// A bare name is a text key.
    fn map_pattern(
        &mut self,
        items: &[ListItem],
        rest: Option<&Ident>,
        k: &Ty,
        v: &Ty,
        missing: bool,
    ) {
        if let Some(r) = rest {
            self.error(r.span, "a map pattern has no `...rest` (spec 1.12)");
        }
        let _ = missing; // Every value of a map may be missing anyway.
        for it in items {
            match &it.key {
                Some(MapKey::Literal(e)) => {
                    self.check_expect(e, k);
                }
                // `host:` or a bare `port`: a text key.
                Some(MapKey::Name(id)) => self.text_key(k, id.span),
                None if matches!(it.pattern.kind, PatternKind::Binding(_)) => {
                    self.text_key(k, it.pattern.span)
                }
                None => {
                    self.error(
                        it.pattern.span,
                        "this part of a map pattern needs its key: `\"home\": [city]` (spec 1.12)",
                    );
                    continue;
                }
            }
            self.take_part(&it.pattern, v, it.default.as_ref(), true, false);
        }
    }

    /// A name used as a key is text, so the map's keys must be `String`.
    fn text_key(&mut self, k: &Ty, span: Span) {
        if !self.inf.unify(k, &Ty::Prim(crate::ty::Prim::String)) {
            let shown = self.show(k);
            self.error(span, format!("a name as a key is text, and this map's keys are `{shown}`: write the key, `7: name` (spec 1.12)"));
        }
    }

    fn record_pattern(
        &mut self,
        p: &Pattern,
        fields: &[RecordFieldPattern],
        ty: &Ty,
        missing: bool,
    ) {
        let ty = self.inf.resolve(ty);
        // Each field's type, and whether it is secret, by name.
        let declared: Vec<(String, Ty, bool)> = match &ty {
            Ty::Adt(def, args) => match self.types.get(def).map(|t| t.shape.clone()) {
                Some(Shape::Record(fs)) => {
                    let subst = self.subst_of(*def, args);
                    for f in fields {
                        if let Some(info) = fs.iter().find(|x| x.name == f.field.name) {
                            self.check_field_access(*def, info, &f.field);
                        }
                    }
                    fs.iter()
                        .map(|f| (f.name.clone(), self.inf.subst(&f.ty, &subst), f.secret))
                        .collect()
                }
                _ => return self.not_a_record(p, &ty),
            },
            Ty::Anon(fs) => fs
                .iter()
                .map(|(n, t)| (n.clone(), t.clone(), false))
                .collect(),
            Ty::Error => Vec::new(),
            _ => return self.not_a_record(p, &ty),
        };
        for f in fields {
            let Some((_, field_ty, secret)) = declared.iter().find(|(n, ..)| *n == f.field.name)
            else {
                if ty != Ty::Error {
                    let shown = self.show(&ty);
                    self.error(
                        f.field.span,
                        format!("`{shown}` has no field `{}`", f.field.name),
                    );
                }
                continue;
            };
            let optional_field = matches!(self.inf.resolve(field_ty), Ty::Optional(_));
            if f.default.is_some() && !optional_field && !missing {
                self.error(
                    f.field.span,
                    format!("`{}` is never `none`, so it needs no default", f.field.name),
                );
            }
            self.take_part(&f.pattern, field_ty, f.default.as_ref(), missing, *secret);
        }
    }

    fn not_a_record(&mut self, p: &Pattern, ty: &Ty) {
        let shown = self.show(ty);
        self.error(
            p.span,
            format!("a record pattern takes a record apart, and this is `{shown}`"),
        );
    }
}

/// `t?`, or `t` when it already is optional.
fn optional(t: Ty) -> Ty {
    match t {
        Ty::Optional(_) => t,
        t => Ty::Optional(Box::new(t)),
    }
}
