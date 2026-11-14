//! Method calls: dispatch to a type's own methods, built-in methods,
//! `.timeout(d)`, and fields holding functions (spec 3.4).

use crate::checker::{Checker, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::*;
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
            _ => None,
        };
        if let Some((sig, subst)) = found {
            if let Ty::Adt(owner, _) = &recv {
                self.check_method_access(*owner, &sig, name);
            }
            return self.apply(&sig, subst, generics, args, e.span, &name.name);
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
                (Ty::List(_), "push") | (Ty::Map(..) | Ty::Set(_), "remove") | (Ty::Set(_), "add")
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
                format!("`{shown}` has no method `{}`", name.name),
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
        _ => return None,
    })
}
