//! Method calls: dispatch to a type's own methods, built-in methods,
//! `.timeout(d)`, and fields holding functions (spec 3.4).

use crate::checker::{Checker, FnSig, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;
use std::collections::HashMap;

impl Checker<'_> {
    pub(crate) fn method_call(
        &mut self,
        e: &Expr,
        base: &Expr,
        name: &Ident,
        generics: &[TypeExpr],
        args: &[Arg],
    ) -> Ty {
        if let Some(t) = self.static_call(base, name, generics, args, e.span) {
            return t;
        }
        if name.name == "timeout"
            && let Some(t) = self.timeout(base, args, e.span)
        {
            return t;
        }
        let mut recv = self.value(base, None);
        // A literal receiver such as `5.wrapping_add(1)` is an `Int` by default.
        if let Ty::Var(v) = self.inf.shallow(&recv) {
            let default = match self.inf.kind(v) {
                crate::infer::VarKind::IntLiteral => Some(Prim::Int),
                crate::infer::VarKind::FloatLiteral => Some(Prim::Float),
                crate::infer::VarKind::Any => None,
            };
            if let Some(p) = default {
                self.inf.unify(&recv, &Ty::Prim(p));
                recv = Ty::Prim(p);
            }
        }
        self.check_arena_call(base, name, args);
        let recv = self.inf.resolve(&recv);
        // A range has the list methods that read it (spec 2.2): it is the
        // list of its numbers, which lowering builds (`lower_range.rs`).
        let recv = match recv {
            Ty::Range(item) if !matches!(name.name.as_str(), "push" | "pop") => {
                let list = Ty::List(item);
                self.expr_types.push((base.span, list.clone()));
                if name.name == "to_list" && args.is_empty() {
                    return list;
                }
                list
            }
            r => r,
        };
        if matches!(recv, Ty::Prim(Prim::Bytes)) && name.name == "as_list" {
            return self.bytes_as_list(generics, args, name.span);
        }
        if matches!(recv, Ty::Prim(Prim::Bytes)) && name.name == "read" {
            return self.bytes_read(generics, args, name.span);
        }
        if let Ty::List(item) = &recv {
            let kind = match name.name.as_str() {
                "map" => Some(crate::gpu::GpuCall::Map),
                "sum" => Some(crate::gpu::GpuCall::Sum),
                "filter" => Some(crate::gpu::GpuCall::Filter),
                "min" | "max" => Some(crate::gpu::GpuCall::MinMax),
                _ => None,
            };
            if let Some(kind) = kind {
                self.note_gpu_map(e.span, item, args, kind);
            }
        }
        let found = match &recv {
            Ty::Adt(def, targs) => self
                .methods
                .get(def)
                .and_then(|m| m.get(&name.name))
                .cloned()
                .map(|s| (s, self.subst_of(*def, targs))),
            Ty::Param(p) => {
                let bounds = self.bounds.get(p).cloned().unwrap_or_default();
                bounds
                    .iter()
                    .find_map(|b| self.methods.get(b)?.get(&name.name).cloned())
                    .map(|s| (s, HashMap::new()))
            }
            Ty::Prim(p) => self
                .ext_methods
                .get(p.name())
                .and_then(|m| m.get(&name.name))
                .cloned()
                .map(|s| (s, HashMap::new())),
            r => self.collection_ext(r, &name.name),
        };
        if let Some((sig, subst)) = found {
            if let Ty::Adt(owner, _) = &recv {
                self.check_method_access(*owner, &sig, name);
                self.check_wheres(*owner, &name.name, &subst, e.span);
            }
            let t = self.apply(&sig, subst, generics, args, e.span, &name.name);
            // `frame.draw(p, …)` gives what `p`'s vertex fn takes (15.3).
            if name.name == "draw"
                && matches!(&recv, Ty::Adt(d, _) if self.res.defs[*d].name == "Frame")
            {
                self.check_draw_args(args, e.span);
            }
            return t;
        }
        // `s.contains(c)`: a `Char` is looked for as its text (spec 14.2).
        if let (Ty::Prim(Prim::String), "contains", [arg]) = (&recv, name.name.as_str(), args)
            && arg.name.is_none()
        {
            let found = self.value(&arg.value, None);
            let string = Ty::Prim(Prim::String);
            if !matches!(self.inf.resolve(&found), Ty::Prim(Prim::Char))
                && !self.coerce(&found, &string)
            {
                self.mismatch(&arg.value, &string, &found);
            }
            return Ty::Prim(Prim::Bool);
        }
        if let Some(t) = self.reflect_has_member(&recv, name, args, e) {
            return t;
        }
        if let Some(t) = self.view_call(&recv, &name.name, args) {
            if matches!(recv, Ty::Builtin("TextBuilder", _)) && name.name != "view" {
                self.require_var(base, &name.name);
            }
            return t;
        }
        if let Some(m) = self.builtin_method(&recv, &name.name) {
            // Waiting for a task, a channel or the network (spec 6.2).
            let waits = matches!(
                (&recv, name.name.as_str()),
                (Ty::Builtin("Task", _), "wait")
                    | (Ty::Builtin("Sender", _), "put")
                    | (Ty::Builtin("Receiver", _), "take")
                    | (Ty::Builtin("Listener" | "Connection" | "UdpSocket", _), _)
            );
            if waits {
                self.no_wait_in_update(e.span, &format!("`.{}`", name.name));
            }
            if matches!(recv, Ty::Builtin("Ptr", _)) {
                self.need_unsafe(&format!("`.{}` of a `Ptr`", name.name), e.span);
            }
            // A listener or connection reaches the network (spec 14.10).
            if matches!(
                recv,
                Ty::Builtin("Listener" | "Connection" | "UdpSocket", _)
            ) {
                self.note_cap("Net", e.span);
            }
            let changes = matches!(
                (&recv, name.name.as_str()),
                (Ty::List(_), "push" | "pop")
                    | (Ty::Builtin("Ring", _), "push")
                    | (
                        Ty::Builtin("TextBuilder", _),
                        "push" | "push_char" | "insert" | "replace" | "remove" | "clear"
                    )
                    | (Ty::Map(..) | Ty::Set(_), "remove")
                    | (Ty::Set(_), "add")
            );
            if changes {
                self.require_var(base, &name.name);
                self.check_captured_change(base);
            }
            let key = match (&recv, name.name.as_str(), m.params.first()) {
                (Ty::List(_), "sort_by", Some(Ty::Fn(_, key, _))) => Some((**key).clone()),
                _ => None,
            };
            // `prices.sum()`, `s.slice(-4)`: say what the call takes.
            if let Some(hint) = takes(&recv, &name.name, args.len(), m.params.len()) {
                self.error(e.span, hint);
                for a in args {
                    self.value(&a.value, None);
                }
                return m.ret;
            }
            let named = m.named.into_iter().map(|(n, t)| (n.to_string(), t, true));
            let params: Vec<_> = m
                .params
                .into_iter()
                .enumerate()
                .map(|(i, t)| (format!("argument {}", i + 1), t, false))
                .chain(named)
                .collect();
            // The function given to `update` runs with the value locked.
            let update = matches!(&recv, Ty::Builtin("Shared", _))
                && matches!(name.name.as_str(), "update" | "update_with");
            self.updating += u32::from(update);
            if update && name.name == "update" {
                self.updates.insert((e.span.start, e.span.end));
            }
            let label = format!("`{}`", name.name);
            // A TLS private `key` takes a secret, as `connect_tls`'s does
            // (spec 8.7, 14.10).
            let tls = matches!(recv, Ty::Builtin("Connection", _));
            let secret: Vec<bool> = (params.iter().enumerate())
                .map(|(i, p)| tls && (p.0 == "key" || (name.name == "accept_tls" && i == 1)))
                .collect();
            let ret = self.passing_errors_on(m.ret, e.span, |c| {
                c.args_with(args, &params, &secret, e.span, &label, "argument");
            });
            self.updating -= u32::from(update);
            // `people.sort_by(p => (p.age, p.name))`: the key must have an
            // order (spec 14.2).
            if let (Some(key), Some(arg)) = (key, args.first()) {
                self.require_order(&key, arg.value.span, "`sort_by`'s key");
            }
            return ret;
        }
        // A field holding a function: `h.on(x)` calls it (spec 3.4).
        if let Ty::Adt(def, adt_args) = &recv
            && let Some(Shape::Record(fields)) = self.types.get(def).map(|t| t.shape.clone())
            && let Some(f) = fields.iter().find(|f| f.name == name.name)
        {
            let subst = self.subst_of(*def, adt_args);
            let fty = self.inf.subst(&f.ty, &subst);
            if let Ty::Fn(ps, ret, _) = self.inf.resolve(&fty) {
                self.check_field_access(*def, f, name);
                let params: Vec<_> = ps
                    .iter()
                    .enumerate()
                    .map(|(i, t)| (format!("argument {}", i + 1), t.clone(), false))
                    .collect();
                self.args(
                    args,
                    &params,
                    e.span,
                    &format!("`{}`", name.name),
                    "argument",
                );
                return (*ret).clone();
            }
        }
        if let Ty::Anon(fs) = &recv
            && let Some((_, fty)) = fs.iter().find(|(n, _)| *n == name.name)
            && let Ty::Fn(ps, ret, _) = self.inf.resolve(fty)
        {
            let params: Vec<_> = ps
                .iter()
                .enumerate()
                .map(|(i, t)| (format!("argument {}", i + 1), t.clone(), false))
                .collect();
            self.args(
                args,
                &params,
                e.span,
                &format!("`{}`", name.name),
                "argument",
            );
            return (*ret).clone();
        }
        for a in args {
            self.value(&a.value, None);
        }
        if !matches!(recv, Ty::Error | Ty::Var(_)) {
            let shown = self.show(&recv);
            self.error(
                name.span,
                match kavach_name(&recv, &name.name) {
                    Some(k) => {
                        format!("`{shown}` has no method `{}`: Kavach's is `{k}`", name.name)
                    }
                    None => format!("`{shown}` has no method `{}`", name.name),
                },
            );
        }
        Ty::Error
    }
}

/// What a built-in method takes, when a call gives it too few arguments
/// and a plain "missing argument" wouldn't say (spec 2.2, 14.2).
fn takes(recv: &Ty, method: &str, given: usize, wanted: usize) -> Option<String> {
    if given >= wanted {
        return None;
    }
    let list = matches!(recv, Ty::List(_));
    Some(match method {
        "sum" if list => "`sum` takes a function that gives each item's number: `prices.sum(p => p)`, or `orders.sum(o => o.total)`".to_string(),
        "slice" => "`slice` takes where to start and where to end: `s.slice(0, 4)`. Positions start at 0 and aren't negative: the last 4 are `s.slice(s.len() - 4, s.len())`".to_string(),
        "count" if list => "`count` takes a test and counts the items it holds for: `xs.count(x => x > 0)`. The number of items is `xs.len()`".to_string(),
        _ => return None,
    })
}

/// Kavach's name for a method other languages name differently: `length`
/// is `len`, `append` is `push`. For "has no method" and "has no field".
pub(crate) fn kavach_name(recv: &Ty, name: &str) -> Option<&'static str> {
    let text = matches!(recv, Ty::Prim(Prim::String));
    let list = matches!(recv, Ty::List(_));
    let sized = text || list || matches!(recv, Ty::Map(..) | Ty::Set(_));
    Some(match name {
        "length" | "size" | "count" | "len" if sized => "len()",
        "append" | "push_back" | "add" if list => "push(x)",
        "to_lower" | "to_lowercase" | "lowercase" | "toLowerCase" | "downcase" if text => "lower()",
        "to_upper" | "to_uppercase" | "uppercase" | "toUpperCase" | "upcase" if text => "upper()",
        "strip" if text => "trim()",
        "startswith" | "startsWith" if text => "starts_with(part)",
        "endswith" | "endsWith" if text => "ends_with(part)",
        "includes" if text || list => "contains(x)",
        "find" | "indexOf" if text => "index_of(part)",
        "indexOf" if list => "index_of(x)",
        "reversed" if list => "reverse()",
        "sorted" if list => "sort()",
        "join" | "await" | "get" | "result" if matches!(recv, Ty::Builtin("Task", _)) => "wait()",
        _ => return None,
    })
}

/// `List`, `Map`, `Set` or `Ring`: the kind of collection `t` is, which
/// methods added with `extend<T> [T]` are kept by (spec 2.5).
pub(crate) fn collection_kind(t: &Ty) -> Option<&'static str> {
    match t {
        Ty::List(_) => Some("List"),
        Ty::Map(..) => Some("Map"),
        Ty::Set(_) => Some("Set"),
        Ty::Builtin("Ring", _) => Some("Ring"),
        _ => None,
    }
}

/// Records what each parameter of `written` (`[T]`) stands for in
/// `actual` (`[Int]`).
fn bind(written: &Ty, actual: &Ty, out: &mut HashMap<DefId, Ty>) {
    match (written, actual) {
        (Ty::Param(p), t) => {
            out.entry(*p).or_insert_with(|| t.clone());
        }
        (Ty::List(w), Ty::List(a)) | (Ty::Set(w), Ty::Set(a)) => bind(w, a, out),
        (Ty::Map(wk, wv), Ty::Map(ak, av)) => {
            bind(wk, ak, out);
            bind(wv, av, out);
        }
        (Ty::Builtin(_, ws), Ty::Builtin(_, xs)) => {
            ws.iter().zip(xs).for_each(|(w, a)| bind(w, a, out));
        }
        _ => {}
    }
}

impl Checker<'_> {
    /// A method added to the lists, maps, sets or rings `recv` is one of,
    /// with what the extend's parameters stand for here.
    fn collection_ext(&self, recv: &Ty, name: &str) -> Option<(FnSig, HashMap<DefId, Ty>)> {
        let kind = collection_kind(recv)?;
        let sig = self.ext_methods.get(kind)?.get(name)?.clone();
        let mut subst = HashMap::new();
        if let Some(written) = self.ext_targets.get(&(kind.to_string(), name.to_string())) {
            bind(written, recv, &mut subst);
        }
        Some((sig, subst))
    }

    /// `extend<T> [T]` (spec 2.5): its methods, kept for every list, map,
    /// set or ring; a name the collection already has is an error.
    pub(crate) fn collect_collection_ext(
        &mut self,
        target: &Ty,
        sigs: Vec<(String, FnSig)>,
        methods: &[Item],
    ) {
        let Some(kind) = collection_kind(target) else {
            return;
        };
        let own = crate::builtin_methods(target, self.res);
        for (name, sig) in sigs {
            if own.contains(&name.as_str()) {
                let span = methods
                    .iter()
                    .find_map(|m| match &m.kind {
                        ItemKind::Fn(f) if f.name.name == name => Some(f.name.span),
                        _ => None,
                    })
                    .unwrap_or(Span { start: 0, end: 0 });
                self.error(
                    span,
                    format!(
                        "`{}` already has a method `{name}`: give yours another name (spec 2.5)",
                        self.show(target)
                    ),
                );
                continue;
            }
            self.ext_targets
                .insert((kind.to_string(), name.clone()), target.clone());
            self.ext_methods
                .entry(kind.to_string())
                .or_default()
                .insert(name, sig);
        }
    }
}
