// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `ai.count(prompt)` (spec 8.4, *Counting tokens before sending*): a
//! prompt's input tokens, counted by the model's provider before it is
//! sent, or estimated offline. The counting itself is
//! `kavach_runtime_net::ai::count`, native code's too. In an agent it has
//! an audit entry, `Ai.count`; it is not an `ask`, so no limit counts it.

use super::Asking;
use crate::calls::Args;
use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Expr;
use kavach_checker::Ty;
use kavach_runtime_core::agents::Asked;
use kavach_runtime_net::ai::{self, Counted};

impl<'a> Interp<'a> {
    pub(crate) fn count_tokens(&mut self, args: Args<'a>, e: &'a Expr) -> Eval<'a> {
        let span = e.span;
        self.refuse_if_held(span)?;
        let prompt = match args.into_iter().find(|(name, _)| name.is_none()) {
            Some((_, Value::Str(s))) => s.to_string(),
            Some((_, v)) => v.display(),
            None => String::new(),
        };
        let agent = self.running.last().cloned();
        // The call's own model, else its agent's, else the default; a
        // list counts with its first model (spec 8.4).
        let named = agent.as_ref().map(|a| a.decl.name.name.as_str());
        let model = self.c.ask_model((span.start, span.end), named);
        let shown = format!(
            "ai.count(\"{}\")",
            prompt.chars().take(60).collect::<String>()
        );
        let check = agent
            .as_ref()
            .map(|a| a.guard.borrow().begin("Ai.count", &shown));
        // Refused only while the audit log can't be written (spec 8.6).
        if let (Some(agent), Some(c)) = (&agent, &check)
            && let Some(why) = c.refused.clone()
        {
            agent.guard.borrow().audit(c, &format!("denied: {why}"));
            let v = vec![("reason", self.str_value(&why))];
            return Err(Flow::Fail(self.builtin_variant("AiError", "Denied", v)));
        }
        let dry = self.dry_run || self.testing;
        let counted = match dry {
            // A dry run or a generated test counts nothing (spec 8.4).
            true => Ok(Counted::dry()),
            false => {
                let settings = kavach_driver::model_settings(&model);
                let wire = self.provider_wire(&model, &settings, span)?;
                let mut side = Asking {
                    i: self,
                    wires: vec![wire],
                    t: Ty::Error,
                    span,
                };
                ai::count(&mut side, &settings, &prompt)?
            }
        };
        // The audit log shows the count as it prints, in both modes.
        let shown_count = counted.as_ref().map(Counted::shown).ok();
        let result = match counted {
            Ok(c) => {
                let ty = self.c.resolution.std_type("Count");
                let Some(def) = ty else {
                    return Err(self.panic("`ai.count` needs `std.ai`'s `Count`", span));
                };
                self.decode(&c.tree(), &Ty::Adt(def, Vec::new()), "")
            }
            Err(f) => Err(self.ai_error(&f.tree(), span)),
        };
        if let (Some(agent), Some(mut c)) = (agent, check) {
            if self.dry_run {
                c.notes.push("dry run".into());
            }
            c.model = Some(Asked {
                model: model.name.clone(),
                counted: !dry,
                ..Asked::default()
            });
            let shown_result = match &result {
                Ok(_) => format!("ok {}", shown_count.unwrap_or_default()),
                Err(Flow::Fail(e)) => format!("error {}", e.display()),
                Err(_) => String::new(),
            };
            agent.guard.borrow().audit(&c, &shown_result);
        }
        result
    }
}
