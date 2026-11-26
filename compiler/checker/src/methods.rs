// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
        if let Some(t) = self.frozen_method(e, base, name, args) {
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
        self.check_fixed_change(&recv, &name.name, name.span);
        let held = recv.clone();
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
        // `d[k].push(x)`: a changing method on a dict entry changes the
        // value under the key, as `d[k] += [x]` does (spec 2.2); a missing
        // key panics. Its receiver is the value, not the optional read.
        let recv = match (&recv, &base.kind) {
            (Ty::Optional(inner), ExprKind::Index { base: dict, .. })
                if changes_in_place(inner, &name.name)
                    && matches!(
                        self.type_at(dict.span).map(|t| self.inf.resolve(&t)),
                        Some(Ty::Map(..))
                    ) =>
            {
                let inner = (**inner).clone();
                self.expr_types.push((base.span, inner.clone()));
                inner
            }
            _ => recv,
        };
        if matches!(recv, Ty::Prim(Prim::Bytes)) && name.name == "as_list" {
            return self.bytes_as_list(generics, args, name.span);
        }
        if matches!(recv, Ty::Prim(Prim::Bytes)) && name.name == "read" {
            return self.bytes_read(generics, args, name.span);
        }
        // `xs.map(f, on: device)`: the device, then the `map` (spec 15.6).
        let args = match (&recv, name.name.as_str()) {
            (Ty::List(_), "map") => self.map_device(args),
            _ => args,
        };
        if let Ty::List(item) = &recv {
            if let (Some(f), "map" | "filter" | "sum") = (args.first(), name.name.as_str()) {
                self.check_gpu_fn_choice(&f.value);
            }
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
            Ty::Adt(def, targs) => {
                // A method written more than once (`op_impls.rs`).
                self.check_written_axis(*def, args);
                let chosen = self.call_variant(*def, &name.name, args);
                // A table joined with a list (spec 14.34).
                let chosen = self.table_join(&recv, name, args).or(chosen);
                if let Some(v) = &chosen {
                    self.chosen
                        .insert((name.span.start, name.span.end), v.clone());
                }
                let n = chosen.as_deref().unwrap_or(&name.name);
                (self.methods.get(def).and_then(|m| m.get(n)).cloned())
                    .map(|s| (s, self.subst_of(*def, targs)))
            }
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
                let n = (self.chosen.get(&(name.span.start, name.span.end)).cloned())
                    .unwrap_or_else(|| name.name.clone());
                self.check_wheres(*owner, &n, &subst, e.span);
            }
            self.note_frozen_self(base, &held, sig.def);
            let t = self.apply(&sig, subst, generics, args, e.span, &name.name);
            self.settle_stats(&name.name, &sig, args, &t);
            // `t.map(f)` that makes rows gives a table (spec 14.34).
            let t = self.table_map(&recv, name, t);
            self.table_key(&recv, name, args);
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
        // `xs.slice(1, 3)`, `xs.take(n)`…: gone, for the slice (spec 2.19).
        if let Some(t) = self.removed_for_slice(&recv, base, name, args) {
            return t;
        }
        if let Some(t) = self.join_given_lambdas(&recv, name, args) {
            return t;
        }
        if let Some(t) = self.view_call(&recv, &name.name, args) {
            // `b.push(text)` changes the builder, as the methods below
            // that change their receiver do (spec 2.18, 3.3).
            if changes_in_place(&recv, &name.name) {
                self.require_var(base, &name.name);
                self.check_captured_change(base);
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
            if changes_in_place(&recv, &name.name) {
                self.require_var(base, &name.name);
                self.check_captured_change(base);
            }
            let key = match (&recv, name.name.as_str(), m.params.first()) {
                (Ty::List(_), "sort_by" | "min_by" | "max_by", Some(Ty::Fn(_, key, _))) => {
                    Some((**key).clone())
                }
                _ => None,
            };
            // `prices.sum()`: say what the call takes.
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
            let reads = !changes_in_place(&recv, &name.name);
            let ret = self.passing_errors_on(m.ret, e.span, |c| {
                if reads {
                    c.frozen.callee = crate::frozen::Callee::Reads;
                }
                c.args_with(args, &params, &secret, e.span, &label, "argument");
            });
            self.updating -= u32::from(update);
            if name.name == "sum" && matches!(recv, Ty::List(_) | Ty::Set(_)) {
                self.sum_gives_a_number(&ret, e.span);
            }
            // `people.sort_by(p => (p.age, p.name))`: the key must have an
            // order (spec 14.2).
            if let (Some(key), Some(arg)) = (key, args.first()) {
                let what = format!("`{}`'s key", name.name);
                self.require_order(&key, arg.value.span, &what);
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
            let hint = (self.dict_read_hint(base, &recv, &name.name, args.len()))
                .or_else(|| crate::int_numbers::range_hint(&recv, &name.name));
            self.error(
                name.span,
                match (kavach_name(&recv, &name.name).or(self.numpy_name(&recv, &name.name)))
                    .or(crate::table_methods::table_name(&recv, &name.name))
                {
                    Some(k) => {
                        format!("`{shown}` has no method `{}`: Kavach's is `{k}`", name.name)
                    }
                    None => match hint {
                        Some(hint) => format!("`{shown}` has no method `{}`: {hint}", name.name),
                        None => format!("`{shown}` has no method `{}`", name.name),
                    },
                },
            );
        }
        Ty::Error
    }

    /// `b["numbers"].len()`: a dict read is an optional, `none` when the key
    /// is missing (spec 2.2); say how to read through it. A changing
    /// method works on the entry itself (`method_call`).
    fn dict_read_hint(&mut self, base: &Expr, recv: &Ty, method: &str, n: usize) -> Option<String> {
        let ExprKind::Index { base: dict, index } = &base.kind else {
            return None;
        };
        let Ty::Optional(inner) = recv else {
            return None;
        };
        let is_dict = matches!(
            self.type_at(dict.span).map(|t| self.inf.resolve(&t)),
            Some(Ty::Map(..))
        );
        if !is_dict || self.builtin_method(inner, method).is_none() {
            return None;
        }
        let key = match &index.kind {
            ExprKind::Str(parts) => match &parts[..] {
                [StrPart::Text(t)] => format!("\"{t}\""),
                _ => return None,
            },
            ExprKind::Int(i) => i.to_string(),
            ExprKind::Name(id) => id.name.clone(),
            _ => return None,
        };
        let read = match &dict.kind {
            ExprKind::Name(id) => format!("{}[{key}]", id.name),
            _ => return None,
        };
        let args = if n == 0 { "" } else { "…" };
        Some(format!(
            "reading a dict gives an optional, `none` when the key is missing (spec 2.2): write `{read}?.{method}({args})`, or `({read} ?? default).{method}({args})`"
        ))
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
    let walks = list || matches!(recv, Ty::Set(_));
    Some(match name {
        "length" | "size" | "count" | "len" if sized => "len()",
        "append" | "push_back" | "add" if list => "push(x)",
        "to_lower" | "to_lowercase" | "lowercase" | "toLowerCase" | "downcase" if text => "lower()",
        "to_upper" | "to_uppercase" | "uppercase" | "toUpperCase" | "upcase" if text => "upper()",
        "strip" if text => "trim()",
        "startswith" | "startsWith" if text => "starts_with(part)",
        "endswith" | "endsWith" if text => "ends_with(part)",
        "some" if walks => "any(test)",
        "every" if walks => "all(test)",
        "includes" if text || walks => "contains(x)",
        "each" | "forEach" | "foreach" if walks => "for_each(f)",
        "findIndex" | "findindex" | "find_first_index" if list => "find_index(test)",
        "findLastIndex" if list => "find_last_index(test)",
        "indexOf" if list => "index_of(x)",
        "find" | "indexOf" if text => "index_of(part)",
        "indexOf" if list => "index_of(x)",
        "reversed" if list => "xs[::-1]",
        "sorted" if list => "sort()",
        "join" | "await" | "get" | "result" if matches!(recv, Ty::Builtin("Task", _)) => "wait()",
        _ => return None,
    })
}

/// Whether built-in method `name` changes a receiver of type `recv` in
/// place, so the receiver must be a `let` (spec 2.2). The one list: the
/// interpreter and lowering ask it too. `recv` is the receiver's type
/// with its variables resolved; a method a program adds (2.5) is never
/// one, whatever its name.
pub fn changes_in_place(recv: &Ty, name: &str) -> bool {
    matches!(
        (recv, name),
        (Ty::List(_), "push" | "pop" | "swap")
            | (Ty::Builtin("Ring", _), "push")
            | (
                Ty::Builtin("TextBuilder", _),
                "push" | "push_char" | "insert" | "replace" | "remove" | "clear"
            )
            | (Ty::Map(..) | Ty::Set(_), "remove")
            | (Ty::Set(_), "add")
    )
}

/// Whether method `name` changes its receiver in place for some receiver
/// type: for code that sees a call before its types are known, as
/// lowering's scan of a lambda's body. Made from `changes_in_place`.
pub fn changes_some_receiver(name: &str) -> bool {
    let error = || Box::new(Ty::Error);
    [
        Ty::List(error()),
        Ty::Map(error(), error()),
        Ty::Set(error()),
        Ty::Builtin("Ring", Vec::new()),
        Ty::Builtin("TextBuilder", Vec::new()),
    ]
    .iter()
    .any(|t| changes_in_place(t, name))
}
