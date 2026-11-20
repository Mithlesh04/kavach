// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Indexing and slicing (spec 2.19): `x[#k]`, `x[start:end:step]` and
//! `x[i, j]`, a text's characters, and the written forms that are
//! compile errors (`xs[-1]`, a step of 0, `rows[1, 2]` on a list).

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;

const INT: Ty = Ty::Prim(Prim::Int);

/// What a value gives for `x[i]` and for `x[a:b]`, when it has positions.
pub(crate) fn positional(t: &Ty) -> Option<(Ty, Ty)> {
    Some(match t {
        Ty::List(item) => ((**item).clone(), t.clone()),
        Ty::Prim(Prim::String) => (Ty::Prim(Prim::Char), t.clone()),
        Ty::Builtin("StrView", _) => (Ty::Prim(Prim::Char), t.clone()),
        Ty::Prim(Prim::Bytes) => (Ty::Prim(Prim::UInt8), t.clone()),
        _ => return None,
    })
}

/// The written number of `-k`, a negative number literal.
fn written_negative(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Unary(UnaryOp::Neg, x) => match &x.kind {
            ExprKind::Int(n) => Some(n.to_string()),
            ExprKind::Float(t) => Some(t.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// How `base` is shown in a message: its name, or `xs`.
pub(crate) fn shown(base: &Expr) -> String {
    match &base.kind {
        ExprKind::Name(id) => id.name.clone(),
        ExprKind::SelfValue => "self".to_string(),
        ExprKind::Field { base, name, .. } => format!("{}.{}", shown(base), name.name),
        _ => "xs".to_string(),
    }
}

impl Checker<'_> {
    /// The type of `base[index]` when `index` is `#k`, a slice or parts
    /// separated by commas; `None` for a plain index, which the caller
    /// types. `t` is the type of `base`, already worked out.
    pub(crate) fn sliced(&mut self, base: &Expr, t: &Ty, index: &Expr) -> Option<Ty> {
        let t = self.inf.resolve(t);
        // A program's own type, through `Index` and `Slice` (spec 2.16).
        if let Some(ty) = self.own_index(base, &t, index) {
            return Some(ty);
        }
        match &index.kind {
            ExprKind::Dims(parts) => {
                for p in parts {
                    self.part_types(p);
                }
                let message = match &t {
                    Ty::List(_) => {
                        let at: Vec<String> = parts.iter().map(|_| "[…]".to_string()).collect();
                        format!(
                            "a list has one dimension: write one index at a time, as `{}{}` (spec 2.19)",
                            shown(base),
                            at.concat()
                        )
                    }
                    Ty::Error => return Some(Ty::Error),
                    other => format!("`{}` can't be indexed with commas", self.show(other)),
                };
                self.error(index.span, message);
                Some(Ty::Error)
            }
            ExprKind::FromEnd(k) => {
                self.check_expect(k, &INT);
                self.no_written_negative(base, k, "#");
                Some(match positional(&t) {
                    Some((item, _)) => item,
                    None => self.not_positional(&t, index, "counted from the end with `#`"),
                })
            }
            ExprKind::Part { .. } => {
                self.part_types(index);
                if let ExprKind::Part { start, end, .. } = &index.kind {
                    for x in [start, end].into_iter().flatten() {
                        if let ExprKind::FromEnd(k) = &x.kind {
                            self.no_written_negative(base, k, "#");
                        } else {
                            self.no_written_negative(base, x, "slice");
                        }
                    }
                }
                Some(match positional(&t) {
                    Some((_, slice)) => slice,
                    None => self.not_positional(&t, index, "sliced"),
                })
            }
            _ => {
                if positional(&t).is_some() {
                    self.no_written_negative(base, index, "index");
                }
                None
            }
        }
    }

    /// Types each part of a slice, or an index among parts.
    pub(crate) fn part_types(&mut self, p: &Expr) {
        match &p.kind {
            ExprKind::Part { start, end, step } => {
                for x in [start, end].into_iter().flatten() {
                    match &x.kind {
                        ExprKind::FromEnd(k) => {
                            self.check_expect(k, &INT);
                        }
                        _ => {
                            self.check_expect(x, &INT);
                        }
                    }
                }
                if let Some(s) = step {
                    self.check_expect(s, &INT);
                    if matches!(s.kind, ExprKind::Int(0)) {
                        self.error(s.span, "the step of a slice can't be 0 (spec 2.19)");
                    }
                }
            }
            ExprKind::FromEnd(k) => {
                self.check_expect(k, &INT);
            }
            _ => {
                self.value(p, None);
            }
        }
    }

    /// `xs[-1]`: a written negative number is a compile error that names
    /// the `#` form (spec 2.19).
    fn no_written_negative(&mut self, base: &Expr, x: &Expr, place: &str) {
        let Some(n) = written_negative(x) else {
            return;
        };
        let name = shown(base);
        let message = match place {
            "index" => format!(
                "Kavach counts from the end with `#`: write `{name}[#{n}]`; a plain number always counts from the start (spec 2.19)"
            ),
            "#" => format!(
                "`#-{n}` is past the end: `#k` already counts from the end, so write `#{n}` or a plain number (spec 2.19)"
            ),
            _ => format!(
                "Kavach counts from the end with `#`: write `#{n}` for `-{n}` in `{name}[…]`, as `{name}[#{n}:]`; a plain number always counts from the start (spec 2.19)"
            ),
        };
        self.error(x.span, message);
    }

    fn not_positional(&mut self, t: &Ty, index: &Expr, what: &str) -> Ty {
        let message = match t {
            Ty::Error | Ty::Var(_) => return Ty::Error,
            Ty::Map(..) => format!(
                "a dict has keys, not positions: it can't be {what}; read a key with `d[key]` (spec 2.19)"
            ),
            other => format!("`{}` can't be {what} (spec 2.19)", self.show(other)),
        };
        self.error(index.span, message);
        Ty::Error
    }

    /// Checks a target of `=` that is a slice or a text's character:
    /// a slice of a list is set only with `=`, and text never changes.
    pub(crate) fn check_slice_target(&mut self, target: &Expr, op: AssignOp) {
        let ExprKind::Index { base, index } = &target.kind else {
            return;
        };
        let found = self.expr_types.iter().rev().find(|(s, _)| *s == base.span);
        let Some(t) = found.map(|(_, t)| t.clone()) else {
            return;
        };
        let t = self.inf.resolve(&t);
        if matches!(t, Ty::Prim(Prim::String) | Ty::Builtin("StrView", _)) {
            self.error(
                target.span,
                "text never changes in place (spec 1.7): make new text, as `s[:i] + c + s[i + 1:]`",
            );
            return;
        }
        let slice = matches!(index.kind, ExprKind::Part { .. });
        if slice && !matches!(t, Ty::List(_)) {
            if !matches!(t, Ty::Error | Ty::Var(_)) {
                let shown = self.show(&t);
                self.error(
                    target.span,
                    format!(
                        "only a list's slice can be set, not {}'s (spec 2.19)",
                        crate::op_impls::a_shown(&shown)
                    ),
                );
            }
            return;
        }
        if slice && op != AssignOp::Set {
            self.error(
                target.span,
                "a slice is set with `=` and as many items (spec 2.19): to add items, use `+=` on the list",
            );
        }
    }

    /// A method that did a slice's job, gone for the slice (spec 2.19):
    /// a compile error that names the slice, with the type it gave.
    pub(crate) fn removed_for_slice(
        &mut self,
        recv: &Ty,
        base: &Expr,
        name: &Ident,
        args: &[Arg],
    ) -> Option<Ty> {
        let text = matches!(recv, Ty::Prim(Prim::String));
        let view = matches!(recv, Ty::Builtin("StrView", _));
        let list = matches!(recv, Ty::List(_));
        let bytes = matches!(recv, Ty::Prim(Prim::Bytes));
        let b = shown(base);
        let arg = |i: usize| args.get(i).map_or("…".to_string(), |a| render(&a.value));
        let two = args.len() == 2;
        let (form, gives) = match name.name.as_str() {
            "slice" if text || view || list || bytes => {
                (format!("{b}[{}:{}]", arg(0), arg(1)), recv.clone())
            }
            "take" if list => (format!("{b}[:{}]", arg(0)), recv.clone()),
            "skip" if list => (format!("{b}[{}:]", arg(0)), recv.clone()),
            "reverse" if list => (format!("{b}[::-1]"), recv.clone()),
            "truncate" if text => (format!("{b}[:{}]", arg(0)), recv.clone()),
            "view" if view && two => (format!("{b}[{}:{}]", arg(0), arg(1)), recv.clone()),
            "view" if text && two => (
                format!("{b}.view()[{}:{}]", arg(0), arg(1)),
                crate::text_views::view_ty(),
            ),
            _ => return None,
        };
        for a in args {
            self.value(&a.value, None);
        }
        self.error(
            name.span,
            format!(
                "`{}` is gone: write the slice `{form}` (spec 2.19)",
                name.name
            ),
        );
        Some(gives)
    }
}

/// A short argument as written: a number or a name, else `…`.
fn render(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Int(n) => n.to_string(),
        ExprKind::Name(_) | ExprKind::Field { .. } | ExprKind::SelfValue => shown(e),
        _ => "…".to_string(),
    }
}
