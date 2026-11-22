// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `ask<T>(prompt)` and `ai.ask_full<T>(prompt)` (spec 8.4): the prompt,
//! the call's own `model:`, and the answer's type, `T` or `Asked<T>`, with
//! `AiError` as the error.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, Span, TypeExpr};

impl Checker<'_> {
    /// `ai.name(…)`: `ai.ask_full<T>(prompt)`, or `ai.spent_text(by)`,
    /// what `std.ai`'s `ai.spent` reads (spec 8.4). Its other functions are
    /// written in Kavach.
    pub(crate) fn ai_call(
        &mut self,
        name: &str,
        generics: &[TypeExpr],
        args: &[Arg],
        span: Span,
    ) -> Ty {
        let string = Ty::Prim(Prim::String);
        match name {
            "ask_full" => self.ask_type(generics, args, span, true),
            "spent_text" => {
                let module = &self.res.modules[self.res.module_at(span.start)].name;
                if !module.starts_with("std.") {
                    self.error(
                        span,
                        "this is for the standard library: use `ai.spent()` (spec 8.4)",
                    );
                }
                let params = [("by".to_string(), string.clone(), false)];
                self.args(args, &params, span, "`ai.spent_text`", "argument");
                string
            }
            _ => {
                self.error(
                    span,
                    format!("`std.ai` has no function `{name}` (spec 8.4)"),
                );
                Ty::Error
            }
        }
    }

    /// The type of an `ask` (`full`: `ai.ask_full`, whose answer comes
    /// with what it took).
    pub(crate) fn ask_type(
        &mut self,
        generics: &[TypeExpr],
        args: &[Arg],
        span: Span,
        full: bool,
    ) -> Ty {
        let shown = if full { "ai.ask_full" } else { "ask" };
        self.no_wait_in_update(span, &format!("`{shown}`"));
        if full {
            // `ask` notes it as an impure prelude function (builtins.rs).
            self.note_cap("Ai", span);
            if self.in_contract {
                self.error(span, "a contract can't call `ai.ask_full`: it uses Ai, and contracts must be pure (spec 7.5)");
            }
            self.full_asks.insert((span.start, span.end));
        }
        let string = Ty::Prim(Prim::String);
        // The prompt, and maybe the call's own `model:` (spec 8.4).
        let (named, plain): (Vec<_>, Vec<_>) = args.iter().partition(|a| a.name.is_some());
        if plain.len() != 1 {
            self.error(
                span,
                format!("`{shown}` takes one prompt: `{shown}<T>(prompt)`, or `{shown}<T>(prompt, model: m)`"),
            );
        }
        if let Some(a) = plain.first() {
            self.check_expect(&a.value, &string);
            self.check_secret(&a.value, "an AI prompt");
        }
        for a in named {
            let label = a.name.as_ref().map_or("", |n| n.name.as_str());
            if label == "model" {
                self.check_model(&a.value);
            } else {
                self.error(a.value.span, format!("`{shown}` has no option `{label}`: its one option is `model:`, a `Model` or a list of them (spec 8.4)"));
            }
        }
        let answer = match generics.first() {
            Some(t) => self.lower(t, None),
            None => self.fresh(),
        };
        let answer = match (full, self.res.std_type("Asked")) {
            (true, Some(def)) => Ty::Adt(def, vec![answer]),
            _ => answer,
        };
        // It fails with an `AiError` (spec 8.4), from `std.ai`.
        let error = match self.builtin_ty("AiError") {
            Ty::Error => self.error_ty(),
            ai => ai,
        };
        Ty::Result(Box::new(answer), Box::new(error))
    }
}
