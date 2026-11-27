// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `json.decode_texts<T>(names, texts)` and `json.encode_texts(value)` (spec
//! 2.12): a record read from, and written as, one text for each field,
//! as a CSV row has them (14.34).

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::{Span, TypeExpr};

type Signature = (Vec<(String, Ty, bool)>, Ty);

impl Checker<'_> {
    /// The parameters and result of `json.decode_texts` or `json.encode_texts`.
    pub(crate) fn json_texts_signature(
        &mut self,
        name: &str,
        generics: &[TypeExpr],
        span: Span,
    ) -> Option<Signature> {
        let string = Ty::Prim(Prim::String);
        let texts = Ty::List(Box::new(Ty::Optional(Box::new(string.clone()))));
        let p = |n: &str, t: Ty| (n.to_string(), t, false);
        if name == "encode_texts" {
            return Some((vec![p("value", self.fresh())], texts));
        }
        let Some(t) = generics.first().map(|g| self.lower(g, None)) else {
            self.error(
                span,
                "`json.decode_texts` needs the type to read: `json.decode_texts<Order>(names, texts)`",
            );
            return Some((Vec::new(), Ty::Error));
        };
        if let Some(why) = self.not_decodable(&t) {
            self.error(span, why);
        }
        self.check_decode_access(&t, span, "`json.decode_texts`");
        self.json_reads.push((self.current_fn, t.clone(), span));
        let error = self.builtin_ty("JsonError");
        Some((
            vec![p("names", Ty::List(Box::new(string))), p("texts", texts)],
            Ty::Result(Box::new(t), Box::new(error)),
        ))
    }
}
