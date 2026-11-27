// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Values read from JSON through generic code (spec 2.12):
//! `json.decode<T>` of a type parameter is checked where `T` becomes a
//! real type, at the generic call, as it is when the type is written out.
//! A generic function that reads a type parameter from JSON passes that on
//! to whatever calls it with another type parameter, until a real type
//! arrives: `http.get_json<T>` learns at build time that its `T` can't
//! hold a function.

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::Span;
use kavach_resolver::DefId;
use std::collections::{HashMap, HashSet};

/// A call of a generic function: the function it is in, the one it calls,
/// each type parameter with its type there, and where.
pub(crate) type GenericCall = (Option<DefId>, DefId, Vec<(DefId, Ty)>, Span);

impl Checker<'_> {
    pub(crate) fn check_generic_json(&mut self) {
        // The type parameters each function reads from JSON.
        let mut reads: HashMap<Option<DefId>, HashSet<DefId>> = HashMap::new();
        for (f, ty, span) in std::mem::take(&mut self.json_reads) {
            let params = self.inf.params(&ty);
            if params.is_empty() {
                // Known only now, after the call was checked.
                let t = self.inf.resolve(&ty);
                if let Some(why) = self.not_decodable(&t)
                    && !self.errors.iter().any(|e| e.span == span)
                {
                    self.error(span, why);
                }
            }
            reads.entry(f).or_default().extend(params);
        }
        let calls = std::mem::take(&mut self.generic_calls);
        let mut reported = HashSet::new();
        let mut made = HashSet::new();
        loop {
            let mut grew = false;
            for (caller, callee, args, span) in &calls {
                let wanted = reads.get(&Some(*callee)).cloned().unwrap_or_default();
                for (param, ty) in args.iter().filter(|(p, _)| wanted.contains(p)) {
                    let inner = self.inf.params(ty);
                    if !inner.is_empty() {
                        // Another type parameter: its own callers decide.
                        let mine = reads.entry(*caller).or_default();
                        let before = mine.len();
                        mine.extend(inner);
                        grew |= mine.len() > before;
                        continue;
                    }
                    let t = self.inf.resolve(ty);
                    if made.insert((span.start, *param)) {
                        let name = self.res.defs[*callee].name.clone();
                        let call = format!("`{name}`, which reads its type from JSON,");
                        self.check_decode_access(&t, *span, &call);
                    }
                    let Some(why) = self.not_decodable(&t) else {
                        continue;
                    };
                    if reported.insert((span.start, *param)) {
                        let (name, param_name) = (
                            self.res.defs[*callee].name.clone(),
                            self.res.defs[*param].name.clone(),
                        );
                        self.error(
                            *span,
                            format!("`{name}` reads its `{param_name}` from JSON, and here that is `{}`: {why}", self.show(&t)),
                        );
                    }
                }
            }
            // Until no function learns of another parameter it reads.
            if !grew {
                break;
            }
        }
    }
}
