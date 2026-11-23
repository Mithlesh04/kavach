// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `json.decode_texts<T>(names, texts)` and `json.encode_texts(value)` (spec
//! 2.12): a record read from one text for each field, each read as its
//! type reads text (`kavach_runtime_core::json::text_tree`), then
//! decoded as `json.decode` decodes; and a record's fields as `print`
//! writes them. Native code does the same (`rt_json_texts.rs`).

use crate::calls::Args;
use crate::generics::subst;
use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_checker::{Prim, Ty};
use kavach_runtime_core::json::{TextKind, Tree, path_field, text_tree};
use std::rc::Rc;

/// How a field of type `ty` reads its text.
fn kind_of(ty: &Ty) -> TextKind {
    match ty {
        Ty::Optional(inner) => kind_of(inner),
        Ty::Prim(Prim::Float) => TextKind::Float,
        Ty::Prim(Prim::Float32) => TextKind::Float32,
        Ty::Prim(Prim::Decimal) => TextKind::Decimal,
        Ty::Prim(Prim::Bool) => TextKind::Bool,
        Ty::Prim(p) if p.is_int() || *p == Prim::BigInt => TextKind::Whole,
        _ => TextKind::Text,
    }
}

impl<'a> Interp<'a> {
    /// `json.decode_texts<T>(names, texts)`; `ty` is the call's type.
    pub(crate) fn json_decode_texts(
        &mut self,
        args: Args<'a>,
        ty: Option<Ty>,
        span: Span,
    ) -> Eval<'a> {
        let Some(Ty::Result(t, _)) = ty else {
            return Ok(Value::Unit);
        };
        let a = crate::json::arrange(args, &["names", "texts"]);
        let list = |v: &Option<Value<'a>>| match v {
            Some(Value::List(xs)) => xs.iter().cloned().collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let (names, texts) = (list(&a[0]), list(&a[1]));
        if names.len() != texts.len() {
            let why = format!(
                "`json.decode_texts` was given {} names and {} texts",
                names.len(),
                texts.len()
            );
            return Err(self.panic(why, span));
        }
        let mut fields = Vec::with_capacity(names.len());
        let mut problems = Vec::new();
        for (name, text) in names.iter().zip(&texts) {
            let name = name.display();
            let value = match text {
                Value::Str(s) => {
                    let kind = self
                        .text_field(&t, &name)
                        .map_or(TextKind::Text, |f| kind_of(&f));
                    text_tree(kind, s).unwrap_or_else(|want| {
                        problems.push((path_field("", &name), want.to_string()));
                        Tree::Null
                    })
                }
                _ => Tree::Null,
            };
            fields.push((name, value));
        }
        if !problems.is_empty() {
            return Err(Flow::Fail(self.invalid(&problems)));
        }
        self.decode_tree(&Tree::Object(fields), &t, false)
    }

    /// The type of field `name` of record type `t`.
    fn text_field(&self, t: &Ty, name: &str) -> Option<Ty> {
        match t {
            Ty::Adt(def, args) => {
                let params = self.type_params(*def, args);
                self.field_ty(*def, name).map(|f| subst(&f, &params))
            }
            Ty::Anon(fields) => fields
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, f)| f.clone()),
            _ => None,
        }
    }

    /// `json.encode_texts(value)`: each field as `print` writes it, `none` for a
    /// `none`, `"<secret>"` for a secret.
    pub(crate) fn json_texts(&mut self, value: Value<'a>, span: Span) -> Eval<'a> {
        let fields: Vec<(bool, Value<'a>)> = match &value {
            Value::Record { fields, .. } => {
                fields.iter().map(|f| (f.secret, f.value.clone())).collect()
            }
            _ => vec![(false, value.clone())],
        };
        let mut out = Vec::with_capacity(fields.len());
        for (secret, v) in fields {
            out.push(match (secret, v) {
                (true, _) => self.str_value("<secret>"),
                (_, Value::None) => Value::None,
                (_, v) => {
                    let text = self.shown(&v, true, span)?;
                    Value::Str(Rc::new(text))
                }
            });
        }
        Ok(Value::List(Rc::new(out.into())))
    }
}
