// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.table` (spec 14.34, decision 0135): the rules of `Table<R>` that
//! need types. The table is written in Kavach (`stdlib/table/`); here:
//!
//! - a row type's fields must be cells, checked where `Table<R>` gets a
//!   real `R`;
//! - `t["name"]` is the column `name`, a list of its field's type, noted
//!   in `table_columns` for the driver to make `t.column(.name)`
//!   (`compiler/driver/src/tables.rs`);
//! - `t.map(f)` gives a table when `f` makes rows, through the second
//!   `map`;
//! - a join's `other` may be a table or a list: the first argument's
//!   type chooses the method.

use crate::checker::{Checker, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;
use std::collections::HashSet;

/// What a cell may hold, for messages.
const CELLS: &str = "a cell holds a number, `Bool`, `Char`, `String`, `Decimal`, a `Date` or `DateTime`, a category (a union whose variants have no fields), or an optional of one (spec 14.34)";

impl Checker<'_> {
    pub(crate) fn table_def(&self) -> Option<DefId> {
        self.res.builtin_type("std.table", "Table")
    }

    /// `R` of a `Table<R>`.
    fn table_row(&self, t: &Ty) -> Option<Ty> {
        match self.inf.resolve(t) {
            Ty::Adt(d, args) if Some(d) == self.table_def() => args.first().cloned(),
            _ => None,
        }
    }

    /// The fields of the record type `row`, in the order written, or
    /// `None` when it is not a record.
    fn row_fields(&self, row: &Ty) -> Option<Vec<(String, Ty)>> {
        match self.inf.resolve(row) {
            Ty::Adt(def, args) => match self.types.get(&def).map(|t| &t.shape) {
                Some(Shape::Record(fields)) => {
                    let subst = self.subst_of(def, &args);
                    let fields = fields.iter();
                    Some(
                        fields
                            .map(|f| (f.name.clone(), self.inf.subst(&f.ty, &subst)))
                            .collect(),
                    )
                }
                _ => None,
            },
            Ty::Anon(fs) => Some(fs),
            _ => None,
        }
    }

    /// Whether a cell can hold a value of type `t` (spec 14.34).
    fn is_cell(&self, t: &Ty, optional_ok: bool) -> bool {
        match self.inf.resolve(t) {
            Ty::Prim(p) => p.is_numeric() || matches!(p, Prim::Bool | Prim::Char | Prim::String),
            Ty::Optional(inner) => optional_ok && self.is_cell(&inner, false),
            Ty::Adt(def, _) => {
                let date = ["Date", "DateTime"]
                    .iter()
                    .any(|n| self.res.builtin_type("std.time", n) == Some(def));
                date || matches!(
                    self.types.get(&def).map(|t| &t.shape),
                    Some(Shape::Union(vs)) if vs.iter().all(|v| v.fields.is_empty())
                )
            }
            _ => false,
        }
    }

    /// Why `row` can't be a table's row, or `None` when it can.
    fn not_a_row(&self, row: &Ty) -> Option<String> {
        let shown = self.show(row);
        let Some(fields) = self.row_fields(row) else {
            return Some(format!(
                "a table's rows are records, each field a column, and `{shown}` is not one: `Table<{shown}>` (spec 14.34)"
            ));
        };
        let (name, ty) = fields.iter().find(|(_, t)| !self.is_cell(t, true))?;
        let field = self.show(ty);
        Some(format!(
            "`Table<{shown}>` can't hold `{shown}`'s field `{name}: {field}`: {CELLS}"
        ))
    }

    /// Each `Table<R>` with a real `R` has rows whose fields are cells:
    /// one error for each row type, where it first appears.
    pub(crate) fn check_table_rows(&mut self) {
        let Some(table) = self.table_def() else {
            return;
        };
        let mut found: Vec<(Span, Ty)> = Vec::new();
        for (span, ty) in &self.expr_types {
            let module = &self.res.modules[self.res.module_at(span.start)];
            if module.name.starts_with("std.") {
                continue;
            }
            let mut rows = Vec::new();
            tables_in(&self.inf.resolve(ty), table, &mut rows);
            found.extend(rows.into_iter().map(|r| (*span, r)));
        }
        found.sort_by_key(|(s, _)| s.start);
        let mut seen = HashSet::new();
        for (span, row) in found {
            if !self.inf.params(&row).is_empty() || has_var(&row) {
                continue;
            }
            let shown = self.show(&row);
            if !seen.insert(shown) {
                continue;
            }
            if let Some(why) = self.not_a_row(&row) {
                self.error(span, why);
            }
        }
    }

    /// `t["name"]` on a table: the column, a list of the field's type.
    /// `None` when `t` is no table or `index` no text, as `t[0]`.
    pub(crate) fn table_column(&mut self, base: &Expr, t: &Ty, index: &Expr) -> Option<Ty> {
        let row = self.table_row(t)?;
        let ExprKind::Str(parts) = &index.kind else {
            return None;
        };
        let name = match parts.as_slice() {
            [StrPart::Text(t)] => t.clone(),
            [] => String::new(),
            _ => {
                self.error(index.span, "`t[\"name\"]` takes a column's name written in the code, not text made while the program runs (spec 14.34)");
                return Some(Ty::Error);
            }
        };
        let Some(fields) = self.row_fields(&row) else {
            let shown = self.show(&row);
            self.error(index.span, format!("`t[\"name\"]` needs a table whose row type is known, and this one's is `{shown}` (spec 14.34)"));
            return Some(Ty::Error);
        };
        let Some((_, field)) = fields.iter().find(|(n, _)| *n == name) else {
            let shown = self.show(&row);
            let names: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
            self.error(
                index.span,
                format!(
                    "`{shown}` has no field `{name}`; it has {} (spec 14.34)",
                    names.join(", ")
                ),
            );
            return Some(Ty::Error);
        };
        let at = crate::user_index::at(base, index);
        self.table_columns
            .insert(at, (name, row.clone(), field.clone()));
        Some(Ty::List(Box::new(field.clone())))
    }

    /// `t.map(f)` where `f` makes rows: a `Table<S>`, through the second
    /// `map`. `ret` is what the first gives, `[S]`.
    pub(crate) fn table_map(&mut self, recv: &Ty, name: &Ident, ret: Ty) -> Ty {
        let (Some(table), Some(_)) = (self.table_def(), self.table_row(recv)) else {
            return ret;
        };
        let Ty::List(item) = self.inf.resolve(&ret) else {
            return ret;
        };
        if name.name != "map" || has_var(&item) || self.not_a_row(&item).is_some() {
            return ret;
        }
        self.chosen
            .insert((name.span.start, name.span.end), "map#2".to_string());
        Ty::Adt(table, vec![*item])
    }

    /// A join of a table with a list: the second `inner_join` or
    /// `left_join`, chosen by the first argument's type, worked out here
    /// and again when the call is checked. `None` for any other call.
    pub(crate) fn table_join(&mut self, recv: &Ty, name: &Ident, args: &[Arg]) -> Option<String> {
        if self.table_row(recv).is_none()
            || !matches!(name.name.as_str(), "inner_join" | "left_join")
        {
            return None;
        }
        let first = args.first().filter(|a| a.name.is_none())?;
        let (inf, errors, warnings, types) = (
            self.inf.clone(),
            self.errors.len(),
            self.warnings.len(),
            self.expr_types.len(),
        );
        let t = self.value(&first.value, None);
        let list = matches!(self.inf.resolve(&t), Ty::List(_));
        self.inf = inf;
        self.errors.truncate(errors);
        self.warnings.truncate(warnings);
        self.expr_types.truncate(types);
        list.then(|| format!("{}#2", name.name))
    }
}

impl Checker<'_> {
    /// A key's type parameter in `std.table`'s own `sort_by`, `min_by`
    /// and `max_by`: its order is checked where the table's method is
    /// called (`table_key`).
    pub(crate) fn table_key_param(&self, t: &Ty) -> bool {
        let Ty::Param(p) = self.inf.resolve(t) else {
            return false;
        };
        (self.res.defs[p].span)
            .is_some_and(|s| self.res.modules[self.res.module_at(s.start)].name == "std.table")
    }

    /// `t.sort_by(key)`, `t.min_by(f)` and `t.max_by(f)`: the key must
    /// have an order, as a list's must (spec 14.2).
    pub(crate) fn table_key(&mut self, recv: &Ty, name: &Ident, args: &[Arg]) {
        if self.table_row(recv).is_none()
            || !matches!(name.name.as_str(), "sort_by" | "min_by" | "max_by")
        {
            return;
        }
        let Some(arg) = args.first() else {
            return;
        };
        if let Some(Ty::Fn(_, key, _)) = self.type_at(arg.value.span).map(|t| self.inf.resolve(&t))
        {
            let what = format!("`{}`'s key", name.name);
            self.require_order(&key, arg.value.span, &what);
        }
    }
}

/// The row types of the tables in `t`, at any depth.
fn tables_in(t: &Ty, table: DefId, out: &mut Vec<Ty>) {
    match t {
        Ty::Adt(d, args) if *d == table => out.extend(args.first().cloned()),
        Ty::Adt(_, args) | Ty::Tuple(args) | Ty::Builtin(_, args) => {
            args.iter().for_each(|a| tables_in(a, table, out));
        }
        Ty::List(x) | Ty::Set(x) | Ty::Optional(x) => tables_in(x, table, out),
        Ty::Map(k, v) | Ty::Result(k, v) => {
            tables_in(k, table, out);
            tables_in(v, table, out);
        }
        Ty::Anon(fs) => fs.iter().for_each(|(_, x)| tables_in(x, table, out)),
        _ => {}
    }
}

/// Whether `t` still holds a type being inferred.
fn has_var(t: &Ty) -> bool {
    match t {
        Ty::Var(_) | Ty::Error => true,
        Ty::Adt(_, a) | Ty::Tuple(a) | Ty::Builtin(_, a) => a.iter().any(has_var),
        Ty::List(x) | Ty::Set(x) | Ty::Optional(x) => has_var(x),
        Ty::Map(k, v) | Ty::Result(k, v) => has_var(k) || has_var(v),
        Ty::Anon(fs) => fs.iter().any(|(_, x)| has_var(x)),
        Ty::Fn(ps, r, _) => ps.iter().any(has_var) || has_var(r),
        _ => false,
    }
}
