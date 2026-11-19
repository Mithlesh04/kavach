// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What the compiler checks about numbers in types (spec 2.9, decision
//! 0137): literals that don't fit, values given where other numbers are
//! expected, indexes past a `len`, changing a list of fixed length, and
//! the conversions `try List<Int, max: 2>(xs)` checks while running.

use crate::checker::Checker;
use crate::ty::{Prim, SizeKind, Ty};
use kavach_ast::*;

/// The methods that change how many items a list holds (spec 2.2).
const GROWING: &[&str] = &[
    "push", "pop", "remove", "insert", "clear", "append", "extend", "truncate", "retain",
];

impl Checker<'_> {
    /// A collection literal `e` of type `t`, given where `expected` has a
    /// number (spec 2.2): it must fit, and it is made with the limit.
    pub(crate) fn size_literal(&mut self, e: &Expr, expected: Option<&Ty>, t: Ty) -> Ty {
        let Some(want) = expected.map(|x| self.inf.shallow_keep(x)) else {
            return t;
        };
        let Ty::Sized(_, kind, n) = &want else {
            return t;
        };
        let Some(count) = self.literal_count(e) else {
            return t;
        };
        let n = self.inf.shallow_keep(n);
        match (&n, kind) {
            (Ty::Var(_), _) => {
                self.inf.unify(&n, &Ty::Num(count));
            }
            (Ty::Num(v), SizeKind::Max) if count > *v => {
                let shown = self.show(&want);
                self.error(
                    e.span,
                    format!(
                        "{} doesn't fit `{shown}`: it holds at most {v} (spec 2.2)",
                        entries(e, count)
                    ),
                );
            }
            (Ty::Num(v), SizeKind::Len) if count != *v => {
                let shown = self.show(&want);
                self.error(
                    e.span,
                    format!(
                        "{} doesn't fit `{shown}`: it holds exactly {v} (spec 2.2)",
                        entries(e, count)
                    ),
                );
            }
            (Ty::Param(p), SizeKind::Len) => {
                let shown = self.show(&want);
                let p = self.def_name(*p).to_string();
                self.error(
                    e.span,
                    format!("`{p}` is known only for each call, so a literal can't be checked against `{shown}`: make it with `try` (spec 2.9)"),
                );
            }
            _ => {}
        }
        if *kind == SizeKind::Max {
            self.sizes
                .limits
                .insert((e.span.start, e.span.end), n.clone());
        }
        Ty::Sized(Box::new(t), *kind, Box::new(n))
    }

    /// How many entries collection literal `e` has: a list's items, a
    /// dict's keys or `Set.of`'s values, those known while compiling and
    /// equal counted once.
    pub(crate) fn literal_count(&self, e: &Expr) -> Option<i64> {
        let distinct = |xs: &mut dyn Iterator<Item = &Expr>| {
            let mut seen = std::collections::HashSet::new();
            let mut count = 0;
            for x in xs {
                match const_key(x) {
                    Some(k) if !seen.insert(k.clone()) => {}
                    _ => count += 1,
                }
            }
            count
        };
        match &e.kind {
            ExprKind::List(items) => Some(items.len() as i64),
            ExprKind::Map(entries) => Some(distinct(&mut entries.iter().map(|(k, _)| k))),
            ExprKind::Call { callee, args, .. } => match &callee.kind {
                ExprKind::Field { base, name, .. }
                    if name.name == "of"
                        && matches!(&base.kind, ExprKind::Name(s) if s.name == "Set") =>
                {
                    Some(distinct(&mut args.iter().map(|a| &a.value)))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// A value of type `actual` where `expected` is needed, when either
    /// has a number (spec 2.9): a number on both must match; a value with
    /// one fits where none is wanted; one without fits only through `try`.
    /// `None` when neither has one.
    pub(crate) fn coerce_sized(&mut self, actual: &Ty, expected: &Ty) -> Option<bool> {
        let (a, e) = (
            self.inf.shallow_keep(actual),
            self.inf.shallow_keep(expected),
        );
        match (&a, &e) {
            (Ty::Sized(ab, ak, an), Ty::Sized(eb, ek, en)) => {
                Some(ak == ek && self.inf.unify(an, en) && self.coerce(ab, eb))
            }
            (Ty::Sized(ab, ..), _) if !matches!(e, Ty::Var(_)) => Some(self.coerce(ab, &e)),
            (Ty::Var(_) | Ty::Error | Ty::Never, Ty::Sized(..)) => None,
            (_, Ty::Sized(..)) => Some(false),
            (_, Ty::Optional(inner))
                if matches!(self.inf.shallow_keep(inner), Ty::Sized(..))
                    && !matches!(a, Ty::Optional(_) | Ty::Var(_)) =>
            {
                Some(self.coerce(&a, inner))
            }
            _ => None,
        }
    }

    /// The message for a value without numbers given where its type with
    /// numbers is expected, if that is the mismatch.
    pub(crate) fn sized_mismatch(&self, expected: &Ty, actual: &Ty) -> Option<String> {
        let Ty::Sized(base, ..) = self.inf.shallow_keep(expected) else {
            return None;
        };
        let a = self.inf.shallow_keep(actual);
        if matches!(a, Ty::Sized(..)) {
            return None;
        }
        let same = match (self.inf.shallow(&base), &a) {
            (Ty::List(_), Ty::List(_)) | (Ty::Map(..), Ty::Map(..)) | (Ty::Set(_), Ty::Set(_)) => {
                true
            }
            (Ty::Builtin(x, _), Ty::Builtin(y, _)) => x == *y,
            (Ty::Prim(Prim::Bytes), Ty::Prim(Prim::Bytes)) => true,
            _ => false,
        };
        if !same {
            return None;
        }
        let (want, got) = (self.show(expected), self.show(actual));
        let call = self.conversion_name(expected);
        Some(format!(
            "a `{got}` may not fit `{want}`, so it becomes one only through `try {call}(…)`, which fails with a `SizeError` when it doesn't fit (spec 2.9)"
        ))
    }

    /// `List<Int, max: 2>`: the conversion's name for a type with numbers.
    pub(crate) fn conversion_name(&self, t: &Ty) -> String {
        let Ty::Sized(base, kind, n) = self.inf.resolve_keep(t) else {
            return self.show(t);
        };
        let n = format!("{}: {}", kind.name(), self.show(&n));
        match self.inf.resolve_keep(&base) {
            Ty::List(x) => format!("List<{}, {n}>", self.show(&x)),
            Ty::Map(k, v) => format!("Dict<{}, {}, {n}>", self.show(&k), self.show(&v)),
            Ty::Set(x) => format!("Set<{}, {n}>", self.show(&x)),
            Ty::Prim(Prim::Bytes) => format!("Bytes<{n}>"),
            _ => self.show(t),
        }
    }

    /// `v[3]` of a `[Float]<len: 3>`: an index written as a number is
    /// checked while compiling (spec 2.2, 2.13).
    pub(crate) fn check_fixed_index(&mut self, base: &Expr, index: &Expr) {
        let Some(k) = self.const_num(index) else {
            return;
        };
        let Some(t) = (self.expr_types.iter().rev())
            .find(|(s, _)| *s == base.span)
            .map(|(_, t)| t.clone())
        else {
            return;
        };
        if let Ty::Sized(_, SizeKind::Len, n) = self.inf.shallow_keep(&t)
            && let Ty::Num(len) = self.inf.shallow_keep(&n)
            && k >= len
        {
            let shown = self.show(&t);
            self.error(
                index.span,
                format!(
                    "index {k} is past `{shown}`: its indexes are 0 to {} (spec 2.2)",
                    len - 1
                ),
            );
        }
    }

    /// `v.push(x)` or `v += xs` on a list of fixed length (spec 2.2).
    pub(crate) fn check_fixed_change(&mut self, recv: &Ty, what: &str, span: Span) {
        if what != "+=" && !GROWING.contains(&what) {
            return;
        }
        if let Ty::Sized(base, SizeKind::Len, n) = self.inf.shallow_keep(recv)
            && matches!(self.inf.shallow(&base), Ty::List(_))
        {
            let shown = self.show(recv);
            let n = self.show(&n);
            self.error(
                span,
                format!("a `{shown}` always holds {n} items, so `{what}` can't change how many: set an item in place, or use `[T]<max: {n}>` (spec 2.2)"),
            );
        }
    }

    /// The type of operand `e`, without its collection's own number:
    /// `Bytes([2]) < Bytes([1, 9])` compares two `Bytes`, and `xs + ys`
    /// is a new list with no limit (spec 2.2).
    pub(crate) fn plain_value(&mut self, e: &Expr) -> Ty {
        let t = self.value(e, None);
        self.inf.shallow(&t)
    }

    /// What an assignment `x op= v` gives `v` to: `xs += ys` needs no
    /// limit of `ys`'s own (spec 2.2), so a compound one takes the plain
    /// type.
    pub(crate) fn assigned_slot(&self, op: AssignOp, slot: Ty) -> Ty {
        match op.binary() {
            Some(_) => self.inf.shallow(&slot),
            None => slot,
        }
    }

    /// `List.capacity(5)`: a limit known while compiling goes in the type
    /// (spec 2.2).
    pub(crate) fn check_known_capacity(&mut self, what: &str, args: &[Arg]) {
        if let Some(a) = args.first().filter(|a| a.name.is_none())
            && let Some(n) = self.const_num(&a.value)
        {
            self.error(
                a.value.span,
                format!("the limit is known while compiling: write it in the type, `{what}<max: {n}>`, and make the value as a literal (spec 2.2)"),
            );
        }
    }
}

/// "a list of 3 items", for messages.
fn entries(e: &Expr, count: i64) -> String {
    let (what, unit) = match &e.kind {
        ExprKind::Map(_) => ("dict", "key"),
        ExprKind::List(_) => ("list", "item"),
        _ => ("set", "value"),
    };
    let s = if count == 1 { "" } else { "s" };
    format!("a {what} of {count} {unit}{s}")
}

/// A value known while compiling, as text to compare, for counting equal
/// keys once.
fn const_key(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Int(n) => Some(format!("i{n}")),
        ExprKind::Bool(b) => Some(format!("b{b}")),
        ExprKind::Char(c) => Some(format!("c{c}")),
        ExprKind::Str(parts) => {
            let mut text = String::from("s");
            for p in parts {
                match p {
                    StrPart::Text(t) => text.push_str(t),
                    StrPart::Expr(_) => return None,
                }
            }
            Some(text)
        }
        _ => None,
    }
}
