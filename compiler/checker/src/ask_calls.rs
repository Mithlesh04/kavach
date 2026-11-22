// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `ask<T>(prompt)`, `ai.ask_full<T>(prompt)` and `ai.count(prompt)`
//! (spec 8.4): the prompt, the call's own `model:`, and the answer's
//! type, `T`, `Asked<T>` or `Count`, with `AiError` as the error.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, Span, TypeExpr};

impl Checker<'_> {
    /// `ai.name(…)`: `ai.ask_full<T>(prompt)`, `ai.count(prompt)`, or
    /// `ai.spent_text(by)`,
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
            "count" => self.count_type(args, span),
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

    /// `ai.count(prompt)` or `ai.count(prompt, model: m)`: a prompt's
    /// tokens, counted before it is sent (spec 8.4), as a `Count`.
    fn count_type(&mut self, args: &[Arg], span: Span) -> Ty {
        self.no_wait_in_update(span, "`ai.count`");
        self.note_cap("Ai", span);
        if self.in_contract {
            self.error(span, "a contract can't call `ai.count`: it uses Ai, and contracts must be pure (spec 7.5)");
        }
        self.prompt_and_model(
            args,
            span,
            "ai.count",
            "`ai.count(prompt)`, or `ai.count(prompt, model: m)`",
        );
        let count = self.builtin_ty("Count");
        let error = match self.builtin_ty("AiError") {
            Ty::Error => self.error_ty(),
            ai => ai,
        };
        Ty::Result(Box::new(count), Box::new(error))
    }

    /// The prompt of an `ask` or a count, and maybe the call's own
    /// `model:` (spec 8.4); `forms` is how the call is written.
    fn prompt_and_model(&mut self, args: &[Arg], span: Span, shown: &str, forms: &str) {
        let string = Ty::Prim(Prim::String);
        let (named, plain): (Vec<_>, Vec<_>) = args.iter().partition(|a| a.name.is_some());
        if plain.len() != 1 {
            self.error(span, format!("`{shown}` takes one prompt: {forms}"));
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
        // The prompt, and maybe the call's own `model:` (spec 8.4).
        let forms = format!("`{shown}<T>(prompt)`, or `{shown}<T>(prompt, model: m)`");
        self.prompt_and_model(args, span, shown, &forms);
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
