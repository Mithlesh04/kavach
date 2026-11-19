// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Numbers in types (spec 2.9, decision 0137): types are positional,
//! numbers are named. `[Float]<len: 3>`, `List<Int, max: 2>`,
//! `Bytes<len: 32>`, and a program's own `type Grid<T, width: Int>`.
//! Here: the numbers a program declares and gives, and the types they
//! make. Literals, conversions and the rest are in `size_checks.rs`.

use crate::checker::Checker;
use crate::ty::{SizeKind, Ty};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};
use std::collections::HashMap;

/// The largest limit a collection may have (spec 2.2): 2³² − 2.
pub(crate) const MAX_LIMIT: i64 = 4_294_967_294;

/// A call of a function with number parameters: where, the function's
/// name, and each number parameter with what it was given.
pub(crate) type NumCall = (Span, String, Vec<(DefId, Ty)>);

/// What the checker keeps about numbers in types.
#[derive(Default)]
pub(crate) struct Sizes {
    /// Every number parameter (`width: Int`), with its default.
    pub(crate) params: HashMap<DefId, Option<Expr>>,
    /// The generic parameters of every type, types and numbers, in the
    /// order they are declared.
    pub(crate) type_generics: HashMap<DefId, Vec<DefId>>,
    /// Constants known while compiling: `const n = 4`.
    pub(crate) consts: HashMap<DefId, i64>,
    /// Values made with a `max` limit in their type, by span: the limit,
    /// a `Num` or a number parameter's `Param`.
    pub(crate) limits: HashMap<(u32, u32), Ty>,
    /// `try List<Int, max: 2>(xs)`: conversions checked while running.
    /// The type as it is shown, with `{n}` for its number.
    pub(crate) checks: HashMap<(u32, u32), (SizeKind, Ty, String)>,
    /// `List<T, len: n>.filled(v)`: how many items each makes, by span.
    pub(crate) filled: HashMap<(u32, u32), Ty>,
    /// The number arguments of each call of a function with number
    /// parameters, by its span.
    pub(crate) calls: Vec<NumCall>,
}

impl Checker<'_> {
    /// Notes every number parameter and module constant, before any
    /// type is lowered.
    pub(crate) fn collect_numbers(&mut self, file: &File) {
        for item in &file.items {
            match &item.kind {
                ItemKind::Stmt(s) => {
                    if let StmtKind::Let { value, .. } = &s.kind
                        && let Some(id) = kavach_resolver::module_const(s)
                        && let Some(&def) = self.decl.get(&id.span.start)
                        && let Some(v) = self.const_num(value)
                    {
                        self.sizes.consts.insert(def, v);
                    }
                }
                ItemKind::Type(t) => {
                    let defs = self.note_generics(&t.generics);
                    if let Some(&def) = self.decl.get(&t.name.span.start) {
                        self.sizes.type_generics.insert(def, defs);
                    }
                    if let TypeBody::Record(r) = &t.body {
                        self.note_method_generics(&r.methods);
                    }
                }
                ItemKind::Fn(f) => {
                    self.note_generics(&f.generics);
                }
                ItemKind::Extend(e) => self.note_method_generics(&e.methods),
                ItemKind::Interface(i) => self.note_method_generics(&i.methods),
                _ => {}
            }
        }
    }

    fn note_method_generics(&mut self, methods: &[Item]) {
        for m in methods {
            if let ItemKind::Fn(f) = &m.kind {
                self.note_generics(&f.generics);
            }
        }
    }

    fn note_generics(&mut self, generics: &[GenericParam]) -> Vec<DefId> {
        let mut defs = Vec::new();
        for g in generics {
            let Some(&def) = self.decl.get(&g.name.span.start) else {
                continue;
            };
            if g.is_number() {
                self.sizes.params.insert(def, g.default.clone());
            } else if g.default.is_some() {
                self.error(
                    g.name.span,
                    "only a number parameter has a default (spec 2.9)",
                );
            }
            defs.push(def);
        }
        defs
    }

    /// Whether `def` is a number parameter.
    pub(crate) fn is_num_param(&self, def: DefId) -> bool {
        self.sizes.params.contains_key(&def)
    }

    /// Records a constant known while compiling (`const n = 4`).
    pub(crate) fn note_const(&mut self, pattern: &Pattern, value: &Expr, mutable: bool) {
        if let (false, PatternKind::Binding(id)) = (mutable, &pattern.kind)
            && let Some(&def) = self.decl.get(&id.span.start)
            && let Some(v) = self.const_num(value)
        {
            self.sizes.consts.insert(def, v);
        }
    }

    /// The value of `e` when it is known while compiling: whole-number
    /// literals and constants, joined by `+ - * / %` and a minus.
    pub(crate) fn const_num(&self, e: &Expr) -> Option<i64> {
        match &e.kind {
            ExprKind::Int(n) => i64::try_from(*n).ok(),
            ExprKind::Unary(UnaryOp::Neg, x) => self.const_num(x)?.checked_neg(),
            ExprKind::Name(id) => {
                let def = self.res.uses.get(&id.span.start)?;
                self.sizes.consts.get(def).copied()
            }
            ExprKind::Binary(op, l, r) => {
                let (a, b) = (self.const_num(l)?, self.const_num(r)?);
                match op {
                    BinaryOp::Add => a.checked_add(b),
                    BinaryOp::Sub => a.checked_sub(b),
                    BinaryOp::Mul => a.checked_mul(b),
                    BinaryOp::Div => a.checked_div(b),
                    BinaryOp::Rem => a.checked_rem(b),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// A number argument (`3`, `N`, `n`): a `Num`, or the `Param` of a
    /// number parameter in scope.
    pub(crate) fn num_arg(&mut self, value: &Expr) -> Ty {
        if let Some(v) = self.const_num(value) {
            return Ty::Num(v);
        }
        if let ExprKind::Name(id) = &value.kind
            && let Some(&def) = self.res.uses.get(&id.span.start)
        {
            if self.is_num_param(def) {
                return Ty::Param(def);
            }
            if matches!(
                self.res.defs[def].kind,
                DefKind::GenericParam | DefKind::Type
            ) || (self.res.defs[def].kind == DefKind::Builtin
                && id.name.starts_with(|c: char| c.is_uppercase()))
            {
                self.error(
                    value.span,
                    format!("`{}` is a type, and a type argument has no name: write it before the numbers, as `Grid<{}, width: 3>` (spec 2.9)", id.name, id.name),
                );
                return Ty::Error;
            }
        }
        self.error(
            value.span,
            "a number in a type is a whole-number literal, a constant, or a number parameter: a number known only while running goes in `List.capacity(n)` (spec 2.9)",
        );
        Ty::Error
    }

    /// `[T]<…>`, `{K: V}<…>` and `List<T, …>`, `Dict`, `Set`, `Ring` and
    /// `Bytes` with their numbers: `base` with its `max` or `len`.
    pub(crate) fn sized_type(&mut self, what: &str, base: Ty, nums: &[&TypeExpr]) -> Ty {
        let allowed: &[SizeKind] = match what {
            "List" => &[SizeKind::Max, SizeKind::Len],
            "Bytes" => &[SizeKind::Len],
            "Dict" | "Set" | "Ring" => &[SizeKind::Max],
            _ => &[],
        };
        let takes = match allowed {
            [SizeKind::Max, SizeKind::Len] => "`max` or `len`",
            [SizeKind::Len] => "`len`",
            _ => "`max`",
        };
        let mut found: Option<(SizeKind, Ty)> = None;
        for n in nums {
            let TypeKind::Num { name, value } = &n.kind else {
                continue;
            };
            if allowed.is_empty() {
                self.not_built_numbers(what, n.span);
                return base;
            }
            let Some(name) = name else {
                let v = self
                    .const_num(value)
                    .map_or("2".to_string(), |v| v.to_string());
                let first = if allowed[0] == SizeKind::Len {
                    "len"
                } else {
                    "max"
                };
                self.error(
                    n.span,
                    format!("a number in a type has a name: write `{first}: {v}` (spec 2.9)"),
                );
                continue;
            };
            let kind = match name.name.as_str() {
                "max" => SizeKind::Max,
                "len" => SizeKind::Len,
                _ => {
                    self.error(
                        name.span,
                        format!(
                            "`{what}` has no number `{}`: it takes {takes} (spec 2.2)",
                            name.name
                        ),
                    );
                    continue;
                }
            };
            if !allowed.contains(&kind) {
                self.error(
                    name.span,
                    format!("`{what}` takes {takes}, not `{}` (spec 2.2)", name.name),
                );
                continue;
            }
            if found.is_some() {
                self.error(
                    n.span,
                    "a collection's type has one number: `max` or `len` (spec 2.2)",
                );
                continue;
            }
            let v = self.num_arg(value);
            if let Ty::Num(x) = v
                && !(0..=MAX_LIMIT).contains(&x)
            {
                self.error(
                    value.span,
                    format!("a limit is from 0 to 4,294,967,294, not {x} (spec 2.2)"),
                );
            }
            found = Some((kind, v));
        }
        match found {
            Some((kind, n)) => Ty::Sized(Box::new(base), kind, Box::new(n)),
            None => base,
        }
    }

    /// A built-in type whose numbers are not built yet (decision 0137).
    fn not_built_numbers(&mut self, what: &str, span: Span) {
        let message = match what {
            "Int" | "UInt" => "`Int<bits: n>`, `UInt<bits: n>` and `Int<range: …>` are not built yet: use `Int8` … `UInt64` (spec 2.1)".to_string(),
            "Decimal" => "`Decimal<digits: …, places: …>` is not built yet: use `Decimal` (spec 2.1)".to_string(),
            "Tensor" => "a tensor's `dims` and `shape` in its type are not built yet (spec 14.33)".to_string(),
            _ => format!("`{what}` takes no numbers (spec 2.9)"),
        };
        self.error(span, message);
    }

    /// The arguments of a program's own type `def` written as `args`, in
    /// the order of its generic parameters: types in order, numbers by
    /// name (spec 2.9). A number left out takes its default; with
    /// `infer`, one with none is worked out, else it is an error.
    pub(crate) fn adt_args(
        &mut self,
        def: DefId,
        args: &[TypeExpr],
        self_ty: Option<&Ty>,
        span: Span,
        infer: bool,
    ) -> Vec<Ty> {
        let generics = self.generic_list(def);
        let has_nums = generics.iter().any(|g| self.is_num_param(*g))
            || args.iter().any(|a| matches!(a.kind, TypeKind::Num { .. }));
        if !has_nums {
            return args.iter().map(|a| self.lower(a, self_ty)).collect();
        }
        let owner = self.res.defs[def].name.clone();
        let mut out: Vec<Option<Ty>> = vec![None; generics.len()];
        let slots: Vec<usize> = (0..generics.len())
            .filter(|i| !self.is_num_param(generics[*i]))
            .collect();
        let (mut next_type, mut seen_num) = (0, false);
        let mut unnamed: Vec<(Span, String)> = Vec::new();
        for a in args {
            let TypeKind::Num { name, value } = &a.kind else {
                if seen_num {
                    self.error(
                        a.span,
                        format!("the types come first, then the numbers with their names: `{owner}<Float, width: 3>` (spec 2.9)"),
                    );
                }
                match slots.get(next_type) {
                    Some(&i) => out[i] = Some(self.lower(a, self_ty)),
                    None => self.error(
                        a.span,
                        format!(
                            "`{owner}` takes {} type argument{}, not more",
                            slots.len(),
                            if slots.len() == 1 { "" } else { "s" }
                        ),
                    ),
                }
                next_type += 1;
                continue;
            };
            seen_num = true;
            let place = match name {
                Some(n) => generics.iter().position(|g| self.def_name(*g) == n.name),
                None => (0..generics.len())
                    .find(|i| self.is_num_param(generics[*i]) && out[*i].is_none()),
            };
            let Some(i) = place.filter(|i| self.is_num_param(generics[*i])) else {
                let message = match (name, place) {
                    (Some(n), Some(_)) => format!(
                        "`{}` is a type parameter, and a type argument has no name: write the type before the numbers, as `{owner}<Float, …>` (spec 2.9)",
                        n.name
                    ),
                    (Some(n), None) => format!(
                        "`{owner}` has no number `{}`{} (spec 2.9)",
                        n.name,
                        self.numbers_named(&generics)
                    ),
                    (None, _) => format!("`{owner}` takes no more numbers (spec 2.9)"),
                };
                self.error(a.span, message);
                continue;
            };
            let pname = self.def_name(generics[i]).to_string();
            if name.is_none() {
                let v = self
                    .const_num(value)
                    .map_or("…".to_string(), |v| v.to_string());
                unnamed.push((a.span, format!("{pname}: {v}")));
            }
            if out[i].is_some() {
                self.error(a.span, format!("`{pname}` is given twice"));
                continue;
            }
            out[i] = Some(self.num_arg(value));
        }
        // `Grid<Float, 2, 3>`: one error, showing every name.
        if let Some((at, _)) = unnamed.first() {
            let named: Vec<&str> = unnamed.iter().map(|(_, s)| s.as_str()).collect();
            self.error(
                *at,
                format!(
                    "a number in a type has a name, as a named argument has: write `{}` (spec 2.9)",
                    named.join(", ")
                ),
            );
        }
        let mut result = Vec::new();
        for (i, g) in generics.iter().enumerate() {
            let t = match out[i].take() {
                Some(t) => t,
                None if !self.is_num_param(*g) => self.fresh(),
                None => match self.sizes.params.get(g).cloned().flatten() {
                    Some(d) => self.num_arg(&d),
                    None if infer => self.fresh(),
                    None => {
                        let pname = self.def_name(*g).to_string();
                        self.error(
                            span,
                            format!("`{owner}` needs its number `{pname}`: write `{pname}: …` in `<…>` (spec 2.9)"),
                        );
                        Ty::Error
                    }
                },
            };
            result.push(t);
        }
        result
    }

    /// The generic parameters of type `def`, in order.
    pub(crate) fn generic_list(&self, def: DefId) -> Vec<DefId> {
        match self.sizes.type_generics.get(&def) {
            Some(g) => g.clone(),
            None => self
                .types
                .get(&def)
                .map(|t| t.generics.clone())
                .unwrap_or_default(),
        }
    }

    /// ": its numbers are `width` and `height`", for messages.
    fn numbers_named(&self, generics: &[DefId]) -> String {
        let names: Vec<String> = (generics.iter())
            .filter(|g| self.is_num_param(**g))
            .map(|g| format!("`{}`", self.def_name(*g)))
            .collect();
        match names.len() {
            0 => ": it takes no numbers".to_string(),
            _ => format!(": its numbers are {}", names.join(", ")),
        }
    }

    /// The explicit generic arguments of a call of a function with
    /// `generics`: types in order, numbers by name.
    pub(crate) fn explicit_generics(
        &mut self,
        generics: &[DefId],
        explicit: &[TypeExpr],
        name: &str,
    ) -> HashMap<DefId, Ty> {
        let mut out = HashMap::new();
        let types: Vec<DefId> = (generics.iter().copied())
            .filter(|g| !self.is_num_param(*g))
            .collect();
        let mut next = 0;
        for w in explicit {
            match &w.kind {
                TypeKind::Num {
                    name: Some(n),
                    value,
                } => match generics.iter().find(|g| self.def_name(**g) == n.name) {
                    Some(&g) if self.is_num_param(g) => {
                        let v = self.num_arg(value);
                        out.insert(g, v);
                    }
                    _ => {
                        let named = self.numbers_named(generics);
                        self.error(
                            n.span,
                            format!("`{name}` has no number `{}`{named} (spec 2.9)", n.name),
                        );
                    }
                },
                TypeKind::Num { name: None, value } => {
                    let first = (generics.iter())
                        .find(|g| self.is_num_param(**g) && !out.contains_key(*g))
                        .map(|g| self.def_name(*g).to_string());
                    let v = self
                        .const_num(value)
                        .map_or("…".to_string(), |v| v.to_string());
                    let message = match first {
                        Some(p) => {
                            format!("a number in a type has a name: write `{p}: {v}` (spec 2.9)")
                        }
                        None => format!("`{name}` takes no numbers (spec 2.9)"),
                    };
                    self.error(w.span, message);
                }
                _ => {
                    if let Some(&g) = types.get(next) {
                        let t = self.lower(w, None);
                        out.insert(g, t);
                    }
                    next += 1;
                }
            }
        }
        out
    }
}

impl Checker<'_> {
    /// The type of a variable with no written type, from its value's: a
    /// `let`, which may change, takes it without the collection's own
    /// number (`let out = Bytes([])` may grow); a `const` keeps it.
    pub(crate) fn let_type(&self, actual: Ty, mutable: bool) -> Ty {
        match (mutable, self.inf.shallow_keep(&actual)) {
            (true, Ty::Sized(..)) => self.inf.shallow(&actual),
            _ => actual,
        }
    }
}
