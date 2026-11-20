// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Each `Model` the program writes, checked by its provider while the
//! program is compiled (spec 8.4, decision 0140): what the provider's
//! `check` gives becomes a compile error where the model is given.

use crate::interp::Interp;
use crate::value::{Flow, Value};
use kavach_driver::{Message, Severity};
use kavach_runtime_core::json::{Limits, Tree, parse};

/// About a second of work for one check, as a tagged literal's checker.
const CHECK_STEPS: u64 = 1_000_000;

impl Interp<'_> {
    pub(crate) fn model_problems(&mut self) -> Vec<Message> {
        let c = self.c;
        let mut out = Vec::new();
        let error = |text: String, span| Message {
            severity: Severity::Error,
            code: "model",
            text,
            span,
        };
        for (span, model) in &c.ai_written {
            self.steps = 0;
            self.step_limit = Some(CHECK_STEPS);
            let settings = kavach_driver::model_settings(model);
            let asked = self.str_value(r#"{"op":"check"}"#);
            let found = self
                .provider_wire(model, &settings, *span)
                .and_then(|wire| self.call_value(wire, vec![asked], *span));
            self.step_limit = None;
            match found {
                Ok(Value::Str(text)) => {
                    let Ok(Tree::Array(items)) = parse(&text, &Limits::default()) else {
                        continue;
                    };
                    for item in items {
                        if let Tree::Text(why) = item {
                            out.push(error(why, *span));
                        }
                    }
                }
                Err(Flow::Panic(why, _)) => {
                    let name = model.provider.as_ref().map_or("Anthropic", |(_, n)| n);
                    let text = format!("the check of `{name}` stopped: {why} (spec 8.4)");
                    out.push(error(text, *span));
                }
                _ => {}
            }
        }
        out
    }
}
