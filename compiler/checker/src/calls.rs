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
        // `T(x)`, with `T` a type parameter with a number bound (spec 2.9).
        if let Some(t) = self.number_conversion(callee, args) {
            return t;
        }
        match &callee.kind {
            ExprKind::Name(id) => {
                if let Some(&def) = self.res.uses.get(&id.span.start) {
                    match self.res.defs[def].kind {
                        DefKind::Fn => {
                            let args = match self.c_fns.contains_key(&def) {
                                true => self.c_arguments(def, args),
                                false => args,
                            };
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
                        DefKind::Type => {
                            return self.construct(def, None, args, e.span, &id.name, generics);
                        }
                        DefKind::Variant { of } => {
                            self.bare_variant(id, of);
                            return self.construct(of, Some(def), args, e.span, &id.name, generics);
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
        self.c_needs_unsafe(sig, span, name);
        // A `-> secret T` function gives a secret (spec 8.7).
        if sig.ret_secret {
            self.secret.insert((span.start, span.end));
        }
        for (i, (g, _)) in sig.generics.iter().enumerate() {
            let t = match explicit.get(i) {
                Some(t) => self.lower(t, None),
                None => self.fresh(),
            };
            subst.insert(*g, t);
        }
        if let Some(def) = sig.def.filter(|_| !sig.generics.is_empty()) {
            let args = sig
                .generics
                .iter()
                .filter_map(|(g, _)| Some((*g, subst.get(g)?.clone())));
            self.generic_calls
                .push((self.current_fn, def, args.collect(), span));
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
            self.check_number_bound(*g, &subst[g], span);
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
        if !sig.uses.is_empty() {
            self.check_round_effect(
                &format!("`{name}`, which uses {},", sig.uses.join(", ")),
                span,
            );
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
        if let Some(t) = self.reflect_static(def, name, args, span) {
            return Some(t);
        }
        let kind = self.res.defs[def].kind;
        let type_name = self.res.defs[def].name.clone();
        match kind {
            DefKind::Type | DefKind::Capability => {
                if let Some(Shape::Union(vs)) = self.types.get(&def).map(|t| &t.shape)
                    && let Some(v) = vs.iter().find(|v| v.name == name.name).map(|v| v.def)
                {
                    return Some(self.construct(def, Some(v), args, span, &name.name, &[]));
                }
                let sig = self
                    .methods
                    .get(&def)
                    .and_then(|m| m.get(&name.name))
                    .cloned();
                match sig {
                    Some(sig) if !sig.has_self || kind == DefKind::Capability => {
                        self.check_method_access(def, &sig, name);
                        // A capability call needs that method (spec 8.2).
                        if kind == DefKind::Capability {
                            self.note_cap(&format!("{type_name}.{}", name.name), name.span);
                        }
                        let (_, subst) = self.instance(def);
                        let own: Vec<(DefId, Ty)> =
                            subst.iter().map(|(g, t)| (*g, t.clone())).collect();
                        let ty = self.apply(&sig, subst, generics, args, span, &name.name);
                        for (g, t) in own {
                            self.check_number_bound(g, &t, span);
                        }
                        Some(ty)
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
            // `List.capacity(n)`, `Map.capacity(n)`, `Set.capacity(n)`, at
            // most `n` entries, maybe with the first ones (spec 2.2). With
            // none, the types come from where the value goes, as `{:}`.
            DefKind::Builtin
                if matches!(type_name.as_str(), "Dict" | "Set" | "List" | "Ring")
                    && name.name == "capacity" =>
            {
                let (made, first) = match type_name.as_str() {
                    "Dict" => {
                        let (k, v) = (Box::new(self.fresh()), Box::new(self.fresh()));
                        (Ty::Map(k.clone(), v.clone()), Ty::Map(k, v))
                    }
                    "Set" => {
                        let t = Box::new(self.fresh());
                        (Ty::Set(t.clone()), Ty::List(t))
                    }
                    "Ring" => {
                        let t = self.fresh();
                        (Ty::Builtin("Ring", vec![t.clone()]), Ty::List(Box::new(t)))
                    }
                    _ => {
                        let t = Box::new(self.fresh());
                        (Ty::List(t.clone()), Ty::List(t))
                    }
                };
                let params = [
                    ("n".to_string(), Ty::Prim(Prim::Int), false),
                    ("first".to_string(), first, true),
                ];
                self.args(args, &params, span, "`capacity`", "argument");
                Some(made)
            }
            // `TextBuilder.capacity(n)`: room for `n` bytes (spec 2.18).
            DefKind::Builtin if type_name == "TextBuilder" && name.name == "capacity" => {
                let params = [("n".to_string(), Ty::Prim(Prim::Int), false)];
                self.args(args, &params, span, "`capacity`", "argument");
                Some(Ty::Builtin("TextBuilder", Vec::new()))
            }
            DefKind::Builtin if type_name == "Bytes" && name.name == "of" => {
                Some(self.bytes_of(generics, args, span))
            }
            DefKind::Builtin if type_name == "Float" && name.name == "from_bits" => {
                Some(self.float_from_bits(args, span))
            }
            // A raw address, such as C's `(void *) -1` (spec 6.4).
            DefKind::Builtin if type_name == "Ptr" && name.name == "from_int" => {
                if self.in_unsafe == 0 {
                    self.error(span, "`Ptr.from_int` makes a raw address: use it inside `unsafe { … }` (spec 6.4)");
                }
                let params = [("n".to_string(), Ty::Prim(Prim::Int), false)];
                self.args(args, &params, span, "`from_int`", "argument");
                Some(Ty::Builtin("Ptr", vec![self.fresh()]))
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
