// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Methods written more than once with one name (spec 2.16, decision
//! 0136). A type may implement an operator's or indexing interface once
//! for each type on the right, each with its own method: two `fn mul`.
//! The standard library's own types may also write a method twice for
//! other arguments, as `std.tensor`'s `sum()` and `sum(axis:)` (14.33).
//! Before names are resolved, the second and later of them are named
//! `mul#2`, `mul#3` …, which no program can write, so every later stage
//! sees one method per name; the checker chooses among them
//! (`compiler/checker/src/op_impls.rs`). Any other name written twice is
//! an error.

use kavach_ast::{File, Item, ItemKind, Span, TypeBody};
use kavach_resolver::Diagnostic;
use std::collections::HashMap;

/// The methods an interface may have more than once (spec 2.16).
const OPERATOR_METHODS: &[&str] = &[
    "add",
    "sub",
    "mul",
    "div",
    "rem",
    "pow",
    "matmul",
    "lt",
    "le",
    "gt",
    "ge",
    "index",
    "index_set",
    "slice",
];

/// Names the later methods written with one name, in each type and
/// `extend`; `in_std` says whether a place is in the standard library.
/// The errors are for names that may not be written twice.
pub(crate) fn name_variants(file: &mut File, in_std: impl Fn(u32) -> bool) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    for item in &mut file.items {
        let (owner, methods) = match &mut item.kind {
            ItemKind::Type(t) => match &mut t.body {
                TypeBody::Record(r) => (t.name.name.clone(), &mut r.methods),
                _ => continue,
            },
            ItemKind::Extend(e) => (String::new(), &mut e.methods),
            _ => continue,
        };
        rename(&owner, methods, &in_std, &mut errors);
    }
    errors
}

fn rename(
    owner: &str,
    methods: &mut [Item],
    in_std: &impl Fn(u32) -> bool,
    errors: &mut Vec<Diagnostic>,
) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for m in methods.iter_mut() {
        let ItemKind::Fn(f) = &mut m.kind else {
            continue;
        };
        let count = seen.entry(f.name.name.clone()).or_insert(0);
        *count += 1;
        if *count == 1 {
            continue;
        }
        let name = f.name.name.clone();
        if !OPERATOR_METHODS.contains(&name.as_str()) && !in_std(f.name.span.start) {
            let of = if owner.is_empty() {
                String::new()
            } else {
                format!(" of `{owner}`")
            };
            errors.push(Diagnostic {
                message: format!("the method `{name}`{of} is written twice: only an operator's or indexing method may be, once for each type it takes (spec 2.16)"),
                span: Span::new(f.name.span.start as usize, f.name.span.end as usize),
            });
        }
        f.name.name = format!("{name}#{count}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The method names of the first type in `src`, after renaming, and
    /// the errors.
    fn renamed(src: &str, in_std: bool) -> (Vec<String>, Vec<String>) {
        let mut file = kavach_parser::parse(src).file;
        let errors = name_variants(&mut file, |_| in_std);
        let ItemKind::Type(t) = &file.items[0].kind else {
            panic!("a type first");
        };
        let TypeBody::Record(r) = &t.body else {
            panic!("a record");
        };
        let names = (r.methods.iter())
            .filter_map(|m| match &m.kind {
                ItemKind::Fn(f) => Some(f.name.name.clone()),
                _ => None,
            })
            .collect();
        (names, errors.into_iter().map(|e| e.message).collect())
    }

    #[test]
    fn later_methods_of_one_name_get_numbers() {
        let src = "type V: Mul<V>, Mul<Float> {\n    x: Float\n    fn mul(self, o: V) -> V { self }\n    fn mul(self, o: Float) -> V { self }\n    fn mul(self, o: Int) -> V { self }\n}\n";
        let (names, errors) = renamed(src, false);
        assert_eq!(names, ["mul", "mul#2", "mul#3"]);
        assert!(errors.is_empty());
    }

    #[test]
    fn other_names_twice_are_errors_outside_the_standard_library() {
        let src = "type B {\n    n: Int\n    fn size(self) -> Int { 1 }\n    fn size(self) -> Int { 2 }\n}\n";
        let (names, errors) = renamed(src, false);
        assert_eq!(names, ["size", "size#2"]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("`size` of `B` is written twice"));
        assert!(renamed(src, true).1.is_empty());
    }
}
