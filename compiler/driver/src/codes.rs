// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The stable code of each message (spec 13.6): what kind of mistake it
//! is, for tools and AI assistants that act on the kind, not the wording.
//! The parser's messages are `syntax`, the size limits' `too-long` and a
//! literal checker's `literal`, by where they come from; the others are
//! known by their wording, in the order of this table, the first match
//! winning.

/// Each code and the words that mark its messages.
const TABLE: &[(&str, &[&str])] = &[
    // Habits from other languages (Rust's `let mut`, `len(xs)`, …).
    (
        "not-kavach",
        &[
            "is now `",
            "Kavach has no",
            "Kavach writes",
            "in Kavach",
            "Kavach's way",
            "is not Kavach",
            "is gone: write",
            "counts from the end with `#`",
            "a list has one dimension",
        ],
    ),
    // Found at compile time: a call that always breaks its contract or
    // an index always out of range (spec 7.7).
    // An index written as a number past a fixed length (spec 2.2).
    (
        "always-fails",
        &["always panics", "always breaks", "is past `"],
    ),
    ("unsafe", &["points at memory C lays out", "`unsafe"]),
    ("gpu", &["`gpu fn`", "(spec 15."]),
    ("contract", &["a contract can't", "contracts must"]),
    ("agent-setting", &["`thinking", "`effort", "`limit Ai`"]),
    (
        "secret",
        &["is secret", "`secret`", "secret value", "the secret field"],
    ),
    (
        "private",
        &[
            "is private",
            "a private field",
            "offers other packages only",
            "may use only `pub(out)` names",
            "shares it only inside that package",
            "is `pub(read)`",
            "has a `pub(read)` field",
            "goes before a record's field",
        ],
    ),
    (
        "operators",
        &[
            "keeps its own operators",
            "goes on the left of",
            "gives an operator",
            "give it one by implementing",
            "would have two meanings",
        ],
    ),
    // Before `capability`: "nothing in this package uses it" is about a
    // name, not a capability.
    (
        "unused",
        &[
            "never used",
            "is unused",
            "not used",
            "nothing in this package uses it",
        ],
    ),
    (
        "capability",
        &[
            " uses ",
            "`uses",
            "uses `",
            "in `can`",
            "capability",
            "is not listed in",
        ],
    ),
    (
        "unknown-name",
        &[
            "is not defined",
            "not defined yet",
            "throws a value away",
            "that functions can't read",
            "is used before its",
            "can't be named alone",
            "is not a standard library module",
            "is gone: ",
            "unknown name",
            "no such",
        ],
    ),
    (
        "duplicate",
        &[
            "is given twice",
            "have the value",
            "already declared",
            "already has a method",
        ],
    ),
    (
        "variant",
        &[
            "is a variant of",
            "every variant of",
            "is for enums",
            "only an enum's variants have values",
            "mixes numbers and text",
        ],
    ),
    (
        "wrong-place",
        &[
            "in an `on cores` loop",
            "every round shares",
            "runs on at least one core",
            "used by no other package",
            "takes only `out`",
            "`pub(out)` goes before",
            "made inside the `arena`",
            "is only for",
            "is for `std.",
            "only the entry file",
            "go in `const` and `let`",
            "write it after the call",
            "names no loop",
            "re-exports names",
            "names each item type",
            "is outside `main`",
        ],
    ),
    ("no-value", &["returns nothing"]),
    ("no-field", &["has no field"]),
    ("no-method", &["has no method"]),
    (
        "cannot-change",
        &[
            "never change",
            "is a `const`",
            "can't change",
            "is not a variable",
            "is not `inout`",
            "must be able to change",
            "would change a copy",
            "but it is a parameter",
            "is already frozen",
            "is sealed with a key",
            "a sealed value's `match`",
        ],
    ),
    (
        "can-fail",
        &["can fail", "`try`", "`catch`", "can't fail", "fails with"],
    ),
    (
        "not-covered",
        &["is not covered", "aren't covered", "does not cover"],
    ),
    (
        "style",
        &[
            "is written",
            "spell it",
            "kebab",
            "snake_case",
            "PascalCase",
        ],
    ),
    (
        "arguments",
        &[
            "argument",
            " takes ",
            "missing `",
            "missing field",
            "parameter",
            "nothing here says what",
            "one value for each place",
        ],
    ),
    (
        "type-mismatch",
        &[
            "might be `none`",
            "a bound names an interface",
            "a `where` bound is",
            "can't match",
            "options can be",
            "is not ",
            "isn't ",
            " gives ",
            "expected",
            "can't be",
            "doesn't fit",
            " needs ",
            "does not implement",
            "doesn't implement",
            "has no ",
            "can't hold",
            "must be `",
            ", and this is",
            "of a fixed size",
            "only when `",
            // Numbers in types (spec 2.9).
            "a limit is from",
            "a number in a type",
            "the limit is known while compiling",
            // Distinct types (spec 2.21).
            "a distinct type holds",
            // Widths and ranges (spec 2.1).
            "a width is from",
            "a value could never be in it",
            "works on bits",
        ],
    ),
];

/// The code of a message, other than the parser's, from its wording;
/// `check` when nothing in the table matches.
pub fn of(text: &str) -> &'static str {
    TABLE
        .iter()
        .find(|(_, words)| words.iter().any(|w| text.contains(w)))
        .map_or("check", |(code, _)| code)
}

/// Every code a message can have, in the table's order, then those set
/// by where a message comes from.
pub fn all() -> Vec<&'static str> {
    let mut codes: Vec<&str> = TABLE.iter().map(|(c, _)| *c).collect();
    codes.extend(["syntax", "too-long", "literal", "check"]);
    codes
}

#[cfg(test)]
mod tests {
    use super::of;

    #[test]
    fn messages_get_their_kind() {
        for (text, code) in [
            (
                "`totl` is not defined; did you mean `total`?",
                "unknown-name",
            ),
            ("`helper` is private to module `orders`", "private"),
            ("`card` is secret", "secret"),
            (
                "`show` uses Console, which `work` doesn't list",
                "capability",
            ),
            (
                "`tokenize` is `pub`, but nothing in this package uses it: remove it (spec 10.1)",
                "unused",
            ),
            ("User has no field zip", "no-field"),
            ("`Square` has no method `name`", "no-method"),
            (
                "`fixed` is a `const`, so `push` can't change it",
                "cannot-change",
            ),
            ("Refunded is not covered", "not-covered"),
            ("Int is not Float; write Float(n)", "type-mismatch"),
            ("a C function, outside `unsafe`", "unsafe"),
            ("Kavach has no overloading: …", "not-kavach"),
            ("Int keeps its own operators", "operators"),
            ("the moon is made of cheese", "check"),
        ] {
            assert_eq!(of(text), code, "{text}");
        }
    }
}
