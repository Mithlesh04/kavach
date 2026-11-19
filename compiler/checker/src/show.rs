// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Types for messages, as people write them, and types with their
//! unknown parts settled.

use crate::checker::Checker;
use crate::infer::VarKind;
use crate::ty::Ty;

impl Checker<'_> {
    /// A type as people write it, for messages.
    pub(crate) fn show(&self, ty: &Ty) -> String {
        let list = |ts: &[Ty]| {
            ts.iter()
                .map(|t| self.show(t))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let args = |ts: &[Ty]| {
            if ts.is_empty() {
                String::new()
            } else {
                format!("<{}>", list(ts))
            }
        };
        match self.inf.resolve_keep(ty) {
            Ty::Prim(p) => p.name().to_string(),
            Ty::Unit => "nothing".to_string(),
            Ty::Never => "never".to_string(),
            Ty::Error => "?".to_string(),
            Ty::List(t) => format!("[{}]", self.show(&t)),
            Ty::Set(t) => format!("Set<{}>", self.show(&t)),
            Ty::Map(k, v) => format!("{{{}: {}}}", self.show(&k), self.show(&v)),
            Ty::Optional(t) => format!("{}?", self.show(&t)),
            Ty::Range(t) => format!("Range<{}>", self.show(&t)),
            Ty::Tuple(ts) => format!("({})", list(&ts)),
            Ty::Anon(fs) => format!(
                "({})",
                fs.iter()
                    .map(|(n, t)| format!("{n}: {}", self.show(t)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Ty::Fn(ps, r, limit) => {
                let uses = match limit.as_deref() {
                    None => String::new(),
                    Some([one]) => format!(" uses {one}"),
                    Some(caps) => format!(" uses ({})", caps.join(", ")),
                };
                match *r {
                    Ty::Result(o, e) => format!(
                        "fn({}) -> {} ! {}{uses}",
                        list(&ps),
                        self.show(&o),
                        self.show(&e)
                    ),
                    r => format!("fn({}) -> {}{uses}", list(&ps), self.show(&r)),
                }
            }
            Ty::Adt(d, ts) => {
                // Numbers with their names (spec 2.9).
                let generics = self.generic_list(d);
                let shown: Vec<String> = (ts.iter().enumerate())
                    .map(
                        |(i, t)| match generics.get(i).filter(|g| self.is_num_param(**g)) {
                            Some(g) => format!("{}: {}", self.def_name(*g), self.show(t)),
                            None => self.show(t),
                        },
                    )
                    .collect();
                match shown.is_empty() {
                    true => self.def_name(d).to_string(),
                    false => format!("{}<{}>", self.def_name(d), shown.join(", ")),
                }
            }
            Ty::Num(n) => n.to_string(),
            Ty::Sized(base, kind, n) => {
                let n = format!("{}: {}", kind.name(), self.show(&n));
                match self.inf.resolve_keep(&base) {
                    Ty::List(_) | Ty::Map(..) => format!("{}<{n}>", self.show(&base)),
                    Ty::Set(t) => format!("Set<{}, {n}>", self.show(&t)),
                    Ty::Builtin(name, ts) if !ts.is_empty() => {
                        format!("{name}<{}, {n}>", list(&ts))
                    }
                    other => format!("{}<{n}>", self.show(&other)),
                }
            }
            Ty::Builtin(n, ts) => format!("{n}{}", args(&ts)),
            Ty::Frozen(t, _, None) => format!("Frozen<{}>", self.show(&t)),
            Ty::Frozen(t, _, Some(k)) => format!("Frozen<{}, {}>", self.show(&t), self.show(&k)),
            Ty::Param(d) => self.def_name(d).to_string(),
            Ty::Result(o, _) => self.show(&o),
            Ty::Var(v) => match self.inf.kind(v) {
                VarKind::IntLiteral => "Int".to_string(),
                VarKind::FloatLiteral => "Float".to_string(),
                VarKind::Any => "_".to_string(),
            },
        }
    }

    /// `ty` with every variable replaced, number literals defaulting to
    /// `Int` and `Float`, and unknowns becoming `Error`.
    pub(crate) fn defaulted(&self, ty: &Ty) -> Ty {
        let resolved = self.inf.resolve_plain(ty);
        self.default_vars(&resolved)
    }

    fn default_vars(&self, ty: &Ty) -> Ty {
        let d = |t: &Ty| Box::new(self.default_vars(t));
        let all = |ts: &[Ty]| ts.iter().map(|t| self.default_vars(t)).collect();
        match ty {
            Ty::Var(v) => match self.inf.kind(*v) {
                VarKind::IntLiteral => Ty::Prim(crate::ty::Prim::Int),
                VarKind::FloatLiteral => Ty::Prim(crate::ty::Prim::Float),
                VarKind::Any => Ty::Error,
            },
            Ty::List(t) => Ty::List(d(t)),
            Ty::Set(t) => Ty::Set(d(t)),
            Ty::Optional(t) => Ty::Optional(d(t)),
            Ty::Range(t) => Ty::Range(d(t)),
            Ty::Map(k, v) => Ty::Map(d(k), d(v)),
            Ty::Result(o, e) => Ty::Result(d(o), d(e)),
            Ty::Tuple(ts) => Ty::Tuple(all(ts)),
            Ty::Anon(fs) => Ty::Anon(
                fs.iter()
                    .map(|(n, t)| (n.clone(), self.default_vars(t)))
                    .collect(),
            ),
            Ty::Fn(ps, r, u) => Ty::Fn(all(ps), d(r), u.clone()),
            Ty::Adt(id, ts) => Ty::Adt(*id, all(ts)),
            Ty::Builtin(n, ts) => Ty::Builtin(n, all(ts)),
            other => other.clone(),
        }
    }
}
