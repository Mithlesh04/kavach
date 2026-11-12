//! Calls: functions, records and variants, methods, and their arguments.

use crate::checker::{Checker, FnSig, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};
use std::collections::HashMap;

impl Checker<'_> {
    pub(crate) fn call(
        &mut self,
        e: &Expr,
        callee: &Expr,
        generics: &[TypeExpr],
        args: &[Arg],
        expected: Option<&Ty>,
    ) -> Ty {
        match &callee.kind {
            ExprKind::Name(id) => {
                if let Some(&def) = self.res.uses.get(&id.span.start) {
                    match self.res.defs[def].kind {
                        DefKind::Fn => {
                            if self.c_fns.contains(&def) && self.in_unsafe == 0 {
                                self.error(
                                    e.span,
                                    format!("`{}` is a C function: call it inside `unsafe {{ … }}`, in a safe function around it (spec 10.7)", id.name),
                                );
                            }
                            if self.c_fns.contains(&def) {
                                self.check_callbacks(def, args);
                            }
                            if let Some(sig) = self.fns.get(&def).cloned() {
                                return self.apply(
                                    &sig,
                                    HashMap::new(),
                                    generics,
                                    args,
                                    e.span,
                                    &id.name,
                                );
                            }
                        }
                        DefKind::Type => return self.construct(def, None, args, e.span, &id.name),
                        DefKind::Variant { of } => {
                            self.bare_variant(id, of);
                            return self.construct(of, Some(def), args, e.span, &id.name);
                        }
                        DefKind::Builtin => {
                            let name = self.res.defs[def].name.clone();
                            return self.builtin_call(&name, generics, args, e.span, expected);
                        }
                        _ => {}
                    }
                }
            }
            ExprKind::Field {
                base,
                name,
                optional: false,
            } => {
                return self.method_call(e, base, name, generics, args);
            }
            _ => {}
        }
        let f = self.value(callee, None);
        match self.inf.resolve(&f) {
            Ty::Fn(params, ret, _) => {
                let params: Vec<_> = params
                    .into_iter()
                    .enumerate()
                    .map(|(i, t)| (format!("argument {}", i + 1), t, false))
                    .collect();
                self.args(args, &params, e.span, "this function", "argument");
                if let ExprKind::Name(id) = &callee.kind {
                    let def = self.res.uses.get(&id.span.start).copied();
                    // A lambda checked only now may turn out to fail.
                    if let Some(d) = def
                        && self.deferred.contains_key(&d)
                    {
                        self.check_deferred(def);
                        if let Some(Ty::Fn(_, r, _)) =
                            self.locals.get(&d).map(|t| self.inf.resolve(t))
                        {
                            return *r;
                        }
                    }
                }
                *ret
            }
            other => {
                for a in args {
                    self.value(&a.value, None);
                }
                if !matches!(other, Ty::Error | Ty::Var(_)) {
                    let shown = self.show(&other);
                    self.error(
                        callee.span,
                        format!("`{shown}` is not a function, so it can't be called"),
                    );
                }
                Ty::Error
            }
        }
    }

    /// Calls a function or method with signature `sig`.
    pub(crate) fn apply(
        &mut self,
        sig: &FnSig,
        mut subst: HashMap<DefId, Ty>,
        explicit: &[TypeExpr],
        args: &[Arg],
        span: Span,
        name: &str,
    ) -> Ty {
        for (i, (g, _)) in sig.generics.iter().enumerate() {
            let t = match explicit.get(i) {
                Some(t) => self.lower(t, None),
                None => self.fresh(),
            };
            subst.insert(*g, t);
        }
        let params: Vec<_> = sig
            .params
            .iter()
            .map(|p| (p.name.clone(), self.inf.subst(&p.ty, &subst), p.has_default))
            .collect();
        let secret: Vec<bool> = sig.params.iter().map(|p| p.secret).collect();
        self.args_with(
            args,
            &params,
            &secret,
            span,
            &format!("`{name}`"),
            "argument",
        );
        self.check_inout_args(sig, args);
        for (g, bounds) in &sig.generics {
            let ty = self.inf.resolve(&subst[g]);
            let has = match &ty {
                Ty::Adt(def, _) => self.implements.get(def),
                Ty::Prim(p) => self.ext_implements.get(p.name()),
                // Not known yet, or bounded where it was declared.
                Ty::Var(_) | Ty::Param(_) | Ty::Error | Ty::Never => continue,
                _ => None,
            };
            let missing: Vec<_> = bounds
                .iter()
                .filter(|b| !has.is_some_and(|s| s.contains(b)))
                .copied()
                .collect();
            for b in missing {
                let i = self.def_name(b).to_string();
                let (t, fix) = match &ty {
                    Ty::Adt(def, _) => {
                        let t = self.def_name(*def).to_string();
                        let fix = format!(": write `type {t}: {i}` or `extend {t}: {i}`");
                        (t, fix)
                    }
                    // `log.info("order placed")`: events are records (14.12).
                    Ty::Prim(_) if i == "LogEvent" => (
                        self.show(&ty),
                        ": log events are records, so a log line can be searched by its fields. Declare one, as `type OrderPlaced: LogEvent { order_id: Int }`, and log that: `log.info(OrderPlaced(order_id: 7))` (spec 14.12)".to_string(),
                    ),
                    Ty::Prim(_) => (
                        self.show(&ty),
                        format!(": write `extend {}: {i}`", self.show(&ty)),
                    ),
                    _ => (self.show(&ty), String::new()),
                };
                self.error(span, format!("`{t}` does not implement `{i}`{fix}"));
            }
        }
        if let Some(def) = sig.def {
            self.note_fn(def, span);
        }
        if self.in_contract && !sig.uses.is_empty() {
            self.error(span, format!("a contract can't call `{name}`: it uses {}, and contracts must be pure (spec 7.5)", sig.uses.join(", ")));
        }
        let ret = self.inf.subst(&sig.ret, &subst);
        let Some(err) = &sig.errors else {
            return ret;
        };
        let err = self.inf.subst(err, &subst);
        // `! E` with `E` a type parameter that no function given here
        // fails with: the call can't fail (spec 3.4).
        match self.inf.shallow(&err) {
            Ty::Var(_) => {
                self.inf.unify(&err, &Ty::Never);
                ret
            }
            Ty::Never => ret,
            _ => Ty::Result(Box::new(ret), Box::new(err)),
        }
    }

    /// Matches arguments to parameters `(name, type, has_default)`.
    /// Positional ones fill parameters in order, named ones by name.
    pub(crate) fn args(
        &mut self,
        args: &[Arg],
        params: &[(String, Ty, bool)],
        span: Span,
        owner: &str,
        what: &str,
    ) {
        self.args_with(args, params, &[], span, owner, what);
    }

    /// Like `args`; `secret` says which parameters may receive a secret
    /// (spec 8.7). Any other parameter refuses one.
    pub(crate) fn args_with(
        &mut self,
        args: &[Arg],
        params: &[(String, Ty, bool)],
        secret: &[bool],
        span: Span,
        owner: &str,
        what: &str,
    ) {
        // `total(4, 8, 15)` for `fn total(prices: [Int])`: Kavach has no
        // rest parameters, so the values go in a list (spec 3.1).
        if let [(name, Ty::List(_), _)] = params
            && args.len() > 1
            && args.iter().all(|a| a.name.is_none())
        {
            let called = owner.trim_matches('`');
            self.error(
                    span,
                    format!(
                        "{owner} takes one list, `{name}`, not separate values: put them in a list, as in `{called}([…])` (spec 3.1)"
                    ),
                );
            for a in args {
                self.value(&a.value, None);
            }
            return;
        }
        let mut filled = vec![false; params.len()];
        let mut next = 0;
        // A built-in method passes on the errors of the functions it is
        // given, not of those used inside its arguments (spec 5.2).
        let mut passing = self.rethrow.take();
        for a in args {
            let index = match &a.name {
                Some(n) => match params.iter().position(|p| p.0 == n.name) {
                    Some(i) => i,
                    None => {
                        let message = crate::fields::gone_field(owner, &n.name).map_or_else(
                            || format!("{owner} has no {what} `{}`", n.name),
                            String::from,
                        );
                        self.error(n.span, message);
                        self.value(&a.value, None);
                        continue;
                    }
                },
                None => {
                    next += 1;
                    if next > params.len() {
                        self.error(
                            a.value.span,
                            format!("too many arguments: {owner} takes {}", params.len()),
                        );
                        self.value(&a.value, None);
                        continue;
                    }
                    next - 1
                }
            };
            if filled[index] {
                self.error(
                    a.value.span,
                    format!("`{}` is given twice", params[index].0),
                );
            }
            filled[index] = true;
            let want = &params[index].1;
            let actual = self.value(&a.value, Some(want));
            self.rethrow = passing.take();
            self.fitting = Some(self.made(&a.value));
            let fits = self.coerce(&actual, want);
            self.fitting = None;
            if !fits {
                self.mismatch(&a.value, want, &actual);
            }
            passing = self.rethrow.take();
            if !secret.get(index).copied().unwrap_or(false) {
                self.check_secret_arg(&a.value, what, &params[index].0, owner);
            }
        }
        self.rethrow = passing;
        for (p, done) in params.iter().zip(filled) {
            if !done && !p.2 {
                self.error(span, format!("missing {what} `{}` for {owner}", p.0));
            }
        }
    }

    /// `User(name: …)` or `TooLarge(value: …)`: records and variants are
    /// created with named arguments (spec 2.4).
    fn construct(
        &mut self,
        def: DefId,
        variant: Option<DefId>,
        args: &[Arg],
        span: Span,
        name: &str,
    ) -> Ty {
        let (instance, subst) = self.instance(def);
        let fields = match (variant, self.types.get(&def).map(|t| t.shape.clone())) {
            (Some(v), _) => self.variant_fields(def, v).unwrap_or_default(),
            (None, Some(Shape::Record(fields))) => fields,
            (None, Some(Shape::Union(_))) => {
                self.error(
                    span,
                    format!("`{name}` is a union: create one of its variants instead"),
                );
                return Ty::Error;
            }
            _ => return Ty::Error,
        };
        if variant.is_none() {
            self.note_drop(def, span);
            self.check_construct_access(def, &fields, span, name);
        }
        if args.iter().any(|a| a.name.is_none()) && !fields.is_empty() {
            let example = fields
                .iter()
                .map(|f| format!("{}: …", f.name))
                .collect::<Vec<_>>()
                .join(", ");
            self.error(
                span,
                format!("records are created with named arguments: write `{name}({example})`"),
            );
            for a in args {
                self.value(&a.value, None);
            }
            return instance;
        }
        let params: Vec<_> = fields
            .iter()
            .map(|f| (f.name.clone(), self.inf.subst(&f.ty, &subst), f.has_default))
            .collect();
        let secret: Vec<bool> = fields.iter().map(|f| f.secret).collect();
        self.args_with(args, &params, &secret, span, &format!("`{name}`"), "field");
        instance
    }

    /// `items.push(x)` changes `items`, so it must be a `const` (spec 2.2).
    pub(crate) fn require_var(&mut self, base: &Expr, method: &str) {
        let mut root = base;
        while let ExprKind::Field { base, .. }
        | ExprKind::TupleField { base, .. }
        | ExprKind::Index { base, .. } = &root.kind
        {
            root = base;
        }
        let ExprKind::Name(id) = &root.kind else {
            return;
        };
        let kind = self
            .res
            .uses
            .get(&id.span.start)
            .map(|d| self.res.defs[*d].kind);
        let what = match kind {
            Some(DefKind::Local { mutable: false }) => "a `const`",
            Some(DefKind::Param) => "a parameter",
            _ => return,
        };
        self.error(
            root.span,
            format!(
                "`{method}` changes `{}`, but it is {what}: declare it with `let`",
                id.name
            ),
        );
    }

    /// `Rect.square(3)`, `Int(text)`, `Payments.refund(…)`, `Bot.start(…)`.
    pub(crate) fn static_call(
        &mut self,
        base: &Expr,
        name: &Ident,
        generics: &[TypeExpr],
        args: &[Arg],
        span: Span,
    ) -> Option<Ty> {
        let ExprKind::Name(id) = &base.kind else {
            return None;
        };
        let def = *self.res.uses.get(&id.span.start)?;
        let kind = self.res.defs[def].kind;
        let type_name = self.res.defs[def].name.clone();
        match kind {
            DefKind::Type | DefKind::Capability => {
                if let Some(Shape::Union(vs)) = self.types.get(&def).map(|t| &t.shape)
                    && let Some(v) = vs.iter().find(|v| v.name == name.name).map(|v| v.def)
                {
                    return Some(self.construct(def, Some(v), args, span, &name.name));
                }
                let sig = self
                    .methods
                    .get(&def)
                    .and_then(|m| m.get(&name.name))
                    .cloned();
                match sig {
                    Some(sig) if !sig.has_self || kind == DefKind::Capability => {
                        // A capability call needs that method (spec 8.2).
                        if kind == DefKind::Capability {
                            self.note_cap(&format!("{type_name}.{}", name.name), name.span);
                        }
                        let (_, subst) = self.instance(def);
                        Some(self.apply(&sig, subst, generics, args, span, &name.name))
                    }
                    _ => {
                        if let Some(t) = self.enum_call(def, name, args, span) {
                            return Some(t);
                        }
                        self.error(
                            name.span,
                            format!("`{type_name}` has no function `{}`", name.name),
                        );
                        Some(Ty::Error)
                    }
                }
            }
            DefKind::Agent if name.name == "start" => Some(self.agent_start(def, args, span)),
            DefKind::Import => self.std_call(def, &name.name, generics, args, span),
            DefKind::Builtin if type_name == "Set" && name.name == "of" => Some(self.set_of(args)),
            // `Map.with_capacity(n)`, `Set.with_capacity(n)` (spec 2.2):
            // empty, so the types come from where the value goes, as `[:]`.
            DefKind::Builtin
                if matches!(type_name.as_str(), "Map" | "Set") && name.name == "with_capacity" =>
            {
                let params = [("n".to_string(), Ty::Prim(Prim::Int), false)];
                self.args(args, &params, span, "`with_capacity`", "argument");
                Some(if type_name == "Map" {
                    Ty::Map(Box::new(self.fresh()), Box::new(self.fresh()))
                } else {
                    Ty::Set(Box::new(self.fresh()))
                })
            }
            DefKind::Builtin if type_name == "Float" && name.name == "from_bits" => {
                Some(self.float_from_bits(args, span))
            }
            DefKind::Builtin => {
                Prim::from_name(&type_name)?;
                let message = if name.name == "parse" {
                    format!(
                        "`{type_name}.parse` is gone: write `{type_name}(text)`, which can fail the same way (spec 2.1)"
                    )
                } else {
                    format!("`{type_name}` has no function `{}`", name.name)
                };
                self.error(name.span, message);
                for a in args {
                    self.value(&a.value, None);
                }
                Some(Ty::Error)
            }
            _ => None,
        }
    }
}
