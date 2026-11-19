// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A string in quotes is exactly what is written (spec 1.7, decision 0131).
//! One that looks like it wanted a template gets a warning that says to
//! write it in backticks: `"Hello, {name}"` where `name` is in scope, or
//! `"a\nb"` whose backslashes all look like escapes.

use crate::Diagnostic;
use crate::resolver::Resolver;
use kavach_ast::{Span, StrPart};

impl Resolver {
    pub(crate) fn plain_string_hint(&mut self, span: Span, parts: &[StrPart]) {
        if self.plain_strings.binary_search(&span.start).is_err() {
            return;
        }
        let [StrPart::Text(text)] = parts else {
            return;
        };
        let message = if let Some(name) = self.filled_name(text) {
            format!(
                "`{{{name}}}` in quotes is these characters: to fill in `{name}`, write the text in backticks, `` `…{{{name}}}…` `` (spec 1.7)"
            )
        } else if looks_escaped(text) {
            "a backslash in quotes is a backslash: for a line break or a tab, write the text in backticks, `` `first\\nsecond` `` (spec 1.7)".to_string()
        } else {
            return;
        };
        self.res.warnings.push(Diagnostic { message, span });
    }

    /// The first `{name}` or `{name.field}`, not after a backslash, whose
    /// name is in scope.
    fn filled_name<'t>(&self, text: &'t str) -> Option<&'t str> {
        let mut rest = text;
        while let Some(i) = rest.find('{') {
            // `C:\temp\{name}`: a path, not a template.
            let after_backslash = rest[..i].ends_with('\\');
            rest = &rest[i + 1..];
            if after_backslash {
                continue;
            }
            let end = rest.find('}')?;
            let inside = &rest[..end];
            let first = inside.split('.').next().unwrap_or("");
            let word = |w: &str| {
                w.chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_')
                    && w.chars().all(|c| c.is_alphanumeric() || c == '_')
            };
            if inside.split('.').all(word) && self.lookup(first).is_some() {
                return Some(inside);
            }
        }
        None
    }
}

/// `"a\nb"`: every backslash is followed by `n`, `t` or `r`, and the text
/// isn't a Windows path (`C:\new`).
fn looks_escaped(text: &str) -> bool {
    let mut found = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n' | 't' | 'r') => found = true,
                _ => return false,
            }
        }
    }
    found && !text.contains(":\\")
}
