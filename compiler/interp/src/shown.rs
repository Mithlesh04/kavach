// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `Display` (spec 1.11): a value whose type implements it shows as its
//! `display()` gives, at the top and inside collections and records.
//! `print` and `debug` (spec 1.10) write values shown so.

use crate::calls::Args;
use crate::interp::Interp;
use crate::value::{Eval, Flow, Shown, Value};
use kavach_ast::Span;

impl<'a> Interp<'a> {
    /// `print(a, b, …, sep: s, end: e)`, to standard output, and
    /// `debug(…)`, to standard error after its file and line (spec 1.10).
    pub(crate) fn print_call(&mut self, name: &str, args: Args<'a>, span: Span) -> Eval<'a> {
        // `print(x)`, the usual call, written straight on.
        if let ("print", [(None, v)]) = (name, args.as_slice()) {
            let text = self.shown(v, true, span)?;
            self.out.push_str(&text);
            self.out.push('\n');
            self.show_live();
            return Ok(Value::Unit);
        }
        let text = self.printed(args, span)?;
        if name == "debug" {
            let (path, line) = self.c.shown_location(span.start);
            self.err.push_str(&format!("[{path}:{line}] {text}"));
        } else {
            self.out.push_str(&text);
            self.show_live();
        }
        Ok(Value::Unit)
    }

    /// What `print` writes: each value as it shows at the top, `sep`
    /// between them (a space unless given) and `end` after the last (a
    /// line end unless given).
    fn printed(&mut self, args: Args<'a>, span: Span) -> Result<String, Flow<'a>> {
        let (mut sep, mut end) = (None, None);
        let mut pieces = Vec::with_capacity(args.len());
        for (name, v) in args {
            match name {
                Some("sep") => sep = Some(v.display()),
                Some("end") => end = Some(v.display()),
                _ => pieces.push(self.shown(&v, true, span)?),
            }
        }
        let mut text = pieces.join(sep.as_deref().unwrap_or(" "));
        text.push_str(end.as_deref().unwrap_or("\n"));
        Ok(text)
    }

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
