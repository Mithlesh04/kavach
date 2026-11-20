// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The standard library modules written in Kavach (spec 14), and which of
//! them a program gets: only the ones it uses (decision 0024). A module is
//! used when the program, or a module already in, writes `std.<name>`, or
//! names one of the module's public types, variants or interfaces, which every file
//! sees (spec 14), or uses a built-in that gives one (`ask`, `5.seconds`).

use crate::Source;
use kavach_ast::{ItemKind, TypeBody};
use kavach_lexer::TokenKind;
use std::collections::HashSet;
use std::sync::OnceLock;

pub(crate) use crate::std_table::BUILTIN;

/// Whether `path` is one of the standard library's files.
pub(crate) fn is_builtin(path: &str) -> bool {
    BUILTIN.iter().any(|(p, ..)| *p == path)
}

/// Each module's public types and variants: the words that bring it in.
fn names() -> &'static Vec<HashSet<String>> {
    static NAMES: OnceLock<Vec<HashSet<String>>> = OnceLock::new();
    NAMES.get_or_init(|| {
        BUILTIN
            .iter()
            .map(|(_, module, text)| {
                let mut out = HashSet::new();
                for item in kavach_parser::parse(text).file.items {
                    if !item.is_pub {
                        continue;
                    }
                    let t = match &item.kind {
                        ItemKind::Type(t) => t,
                        ItemKind::Interface(i) => {
                            out.insert(i.name.name.clone());
                            continue;
                        }
                        _ => continue,
                    };
                    out.insert(t.name.name.clone());
                    // `std.ai`'s variants (`NotFound`, `Answer`, `Other`)
                    // are common words, and every use of one goes with
                    // `ask` or a type of its own (decision 0140).
                    if *module == "std.ai" {
                        continue;
                    }
                    if let TypeBody::Union(vs) = &t.body {
                        out.extend(vs.iter().map(|v| v.name.name.clone()));
                    }
                }
                out
            })
            .collect()
    })
}

/// The names a program's `text` uses as code: identifiers and dotted
/// paths, not words in comments or in a string's own text, less the
/// types, variants and interfaces it declares itself. So `// Event`, or a
/// program's own `type Event`, brings in no module.
fn code_words(text: &str, into: &mut HashSet<String>) {
    let mut used = HashSet::new();
    let mut path = String::new();
    let mut after_dot = false;
    let flush = |path: &mut String, used: &mut HashSet<String>| {
        if !path.is_empty() {
            used.extend(path.split('.').map(str::to_string));
            used.insert(std::mem::take(path));
        }
    };
    for t in &kavach_lexer::lex(text).tokens {
        match t.kind {
            TokenKind::Ident => {
                if !(after_dot && !path.is_empty()) {
                    flush(&mut path, &mut used);
                } else {
                    path.push('.');
                }
                path.push_str(&text[t.span.start as usize..t.span.end as usize]);
                after_dot = false;
            }
            TokenKind::Dot if !path.is_empty() => after_dot = true,
            _ => {
                flush(&mut path, &mut used);
                after_dot = false;
            }
        }
    }
    flush(&mut path, &mut used);
    for item in kavach_parser::parse(text).file.items {
        match &item.kind {
            ItemKind::Type(t) => {
                used.remove(&t.name.name);
                if let TypeBody::Union(vs) = &t.body {
                    for v in vs {
                        used.remove(&v.name.name);
                    }
                }
            }
            ItemKind::Interface(i) => {
                used.remove(&i.name.name);
            }
            _ => {}
        }
    }
    into.extend(used);
}

/// The list and dict methods of spec 14.2 written in Kavach, by module.
const TABLE_METHODS: [(&str, &[&str]); 2] = [
    (
        "std.collections.grouping",
        &[
            "group_by",
            "count_by",
            "unique",
            "unique_by",
            "inner_join",
            "left_join",
            "pivot",
            "map_values",
            "to_list",
        ],
    ),
    (
        "std.collections.stats",
        &["mean", "median", "var", "std", "describe"],
    ),
];

/// The modules of the methods `text` calls: `.mean(` brings in
/// `std.collections.stats`.
fn table_methods(text: &str, into: &mut HashSet<String>) {
    let tokens = kavach_lexer::lex(text).tokens;
    for w in tokens.windows(3) {
        let called = matches!(w[0].kind, TokenKind::Dot | TokenKind::QuestionDot)
            && matches!(w[1].kind, TokenKind::Ident)
            && matches!(w[2].kind, TokenKind::LParen);
        if !called {
            continue;
        }
        let name = &text[w[1].span.start as usize..w[1].span.end as usize];
        for (module, methods) in TABLE_METHODS {
            if methods.contains(&name) {
                into.insert(module.to_string());
            }
        }
    }
}

/// The modules `sources` use, directly or through each other, as sources
/// to add after them. A file given as the program that is itself a
/// standard library module counts as that module.
pub(crate) fn used(sources: &[Source]) -> Vec<Source> {
    let mut seen = HashSet::new();
    for s in sources {
        code_words(&s.text, &mut seen);
    }
    // Built-ins whose values are the modules' types: `ask` fails with
    // `AiError`; `5.seconds` and `.timeout(d)` are `Duration`s.
    if ["ask", "Model", "AgentSettings"]
        .iter()
        .any(|w| seen.contains(*w))
    {
        seen.insert("std.ai".to_string());
    }
    let units = kavach_checker::DURATION_UNITS.iter().map(|(u, _)| *u);
    if units.chain(["timeout"]).any(|u| seen.contains(u)) {
        seen.insert("std.time".to_string());
    }
    // A channel's `take()` gives `Taken<T>` (spec 9.4).
    if seen.contains("Channel") {
        seen.insert("std.tasks".to_string());
    }
    // `T.members()` gives `std.reflect`'s types (14.32).
    if seen.contains("members") {
        seen.insert("std.reflect".to_string());
    }
    // Grouping and statistics are list methods written in Kavach (14.2).
    for s in sources {
        table_methods(&s.text, &mut seen);
    }
    let mut taken = vec![false; BUILTIN.len()];
    for (i, (_, module, _)) in BUILTIN.iter().enumerate() {
        taken[i] = sources.iter().any(|s| s.module == *module);
    }
    let mut out = Vec::new();
    loop {
        let next = BUILTIN.iter().enumerate().position(|(i, (_, module, _))| {
            !taken[i] && (seen.contains(*module) || names()[i].iter().any(|n| seen.contains(n)))
        });
        let Some(i) = next else {
            break;
        };
        taken[i] = true;
        let (path, module, text) = BUILTIN[i];
        // A module brings in only the modules it names (`use std.net`):
        // its own words would match common variant names (`Denied`).
        let mut own = HashSet::new();
        code_words(text, &mut own);
        // Its durations are `std.time`'s, as a program's are.
        let mut units = kavach_checker::DURATION_UNITS.iter().map(|(u, _)| *u);
        if units.any(|u| own.contains(u)) || own.contains("timeout") {
            seen.insert("std.time".to_string());
        }
        if own.contains("Channel") {
            seen.insert("std.tasks".to_string());
        }
        seen.extend(own.into_iter().filter(|w| w.starts_with("std.")));
        out.push(Source {
            path: path.to_string(),
            module: module.to_string(),
            text: text.to_string(),
            package: String::new(),
        });
    }
    // In the order of `BUILTIN`, as before.
    out.sort_by_key(|s| BUILTIN.iter().position(|(p, ..)| *p == s.path));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modules_for(text: &str) -> Vec<String> {
        let program = Source {
            path: "main.kav".into(),
            module: "main".into(),
            text: text.into(),
            package: String::new(),
        };
        used(&[program]).into_iter().map(|s| s.module).collect()
    }

    /// Every program sees the standard library's public types (10.2), so
    /// one named like a prelude type hides it: std.system's `Display`
    /// once hid the `Display` interface from every program.
    #[test]
    fn no_standard_type_takes_a_prelude_name() {
        for (path, _, text) in BUILTIN {
            for item in kavach_parser::parse(text).file.items {
                let name = match &item.kind {
                    ItemKind::Type(t) => &t.name.name,
                    ItemKind::Interface(i) => &i.name.name,
                    _ => continue,
                };
                assert!(
                    !item.is_pub || !kavach_resolver::PRELUDE_TYPES.contains(&name.as_str()),
                    "{path} declares `{name}`, a prelude name"
                );
            }
        }
    }

    #[test]
    fn a_program_gets_only_the_modules_it_uses() {
        for (text, want) in [
            ("print(\"hi\")", vec![]),
            ("use std.json\nprint(json.stringify(1))", vec!["std.json"]),
            // A module's own `use`s come along: http needs json, fs (for
            // `http.file`), net, h2, its forms and its server, which needs
            // cli for its stop hold; h2's client takes turns on a channel,
            // which brings `Taken` from tasks.
            (
                "use std.http",
                vec![
                    "std.json",
                    "std.fs",
                    "std.time.digits",
                    "std.time",
                    "std.net",
                    "std.http",
                    "std.cli",
                    "std.tasks",
                    "std.http.upgrade",
                    "std.http.multipart",
                    "std.http.server",
                    "std.http.hpack",
                    // http joins traces (spec 14.12).
                    "std.log",
                    "std.http.message",
                    "std.http.client",
                    "std.http.h2.wire",
                    "std.http.h2.client",
                    "std.http.h2",
                    "std.http.h2.serving",
                    "std.http.h3",
                ],
            ),
            // Public types and variants are seen everywhere (spec 14).
            ("fn f(e: FsError) {}", vec!["std.fs"]),
            // Only names used as code count: not a comment, a string's
            // text, or a type the program declares itself (S3).
            (
                "// Event
print(1)",
                vec![],
            ),
            ("print(\"Event\")", vec![]),
            (
                "type Event = Start | Stop
print(Event.Start)",
                vec![],
            ),
            (
                "use std.window
fn f(e: Event) {}",
                // Gamepad events come to windows (spec 14.26).
                vec!["std.window", "std.input"],
            ),
            ("const d = 5.seconds", vec!["std.time.digits", "std.time"]),
            // `Model`'s `timeout` is a `Duration`.
            (
                "const a = ask<Bool>(\"?\")",
                vec!["std.time.digits", "std.time", "std.ai"],
            ),
        ] {
            assert_eq!(modules_for(text), want, "{text}");
        }
    }
}
