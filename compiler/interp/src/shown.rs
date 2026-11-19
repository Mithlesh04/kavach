// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `Display` (spec 1.11): a value whose type implements it shows as its
//! `display()` gives, at the top and inside collections and records.

use crate::interp::Interp;
use crate::value::{Flow, Shown, Value};
use kavach_ast::Span;

impl<'a> Interp<'a> {
    /// `v` as `print` shows it (`top`), or as it shows inside a collection.
    pub(crate) fn shown(
        &mut self,
        v: &Value<'a>,
        top: bool,
        span: Span,
    ) -> Result<String, Flow<'a>> {
        if self.c.display_types.is_empty() {
            return Ok(if top { v.display() } else { v.display_code() });
        }
        let mut texts = Shown::new();
        self.display_each(v, &mut texts, span)?;
        Ok(v.write_shown(top, &texts))
    }

    /// Runs `display()` for `v` and each value inside it whose type
    /// implements `Display`, keeping the text by the value's address.
    fn display_each(
        &mut self,
        v: &Value<'a>,
        texts: &mut Shown,
        span: Span,
    ) -> Result<(), Flow<'a>> {
        let def = match v {
            Value::Record { def, .. } => Some(*def),
            Value::Variant { union, .. } => Some(*union),
            _ => None,
        };
        let own = def
            .filter(|d| self.c.display_types.contains(d))
            .and_then(|d| self.methods.get(&d).and_then(|m| m.get("display")).copied());
        if let Some(f) = own {
            let text = self.call_fn(f, Some(v.clone()), Vec::new(), span)?;
            texts.insert(std::ptr::from_ref(v) as usize, text.display());
            return Ok(());
        }
        match v {
            Value::List(items) => {
                for x in items.iter() {
                    self.display_each(x, texts, span)?;
                }
            }
            Value::Tuple(items) => {
                for x in items {
                    self.display_each(x, texts, span)?;
                }
            }
            Value::Map(m) => {
                for (k, x) in m.entries.iter() {
                    self.display_each(k, texts, span)?;
                    self.display_each(x, texts, span)?;
                }
            }
            Value::Set(s) => {
                for (k, _) in s.entries.iter() {
                    self.display_each(k, texts, span)?;
                }
            }
            Value::Record { fields, .. } | Value::Variant { fields, .. } => {
                for f in fields.iter().filter(|f| !f.secret) {
                    self.display_each(&f.value, texts, span)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
