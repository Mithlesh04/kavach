//! The standard library modules written in Kavach (spec 14), and which of
//! them a program gets: only the ones it uses (decision 0024). A module is
//! used when the program, or a module already in, writes `std.<name>`, or
//! names one of the module's public types, variants or interfaces, which every file
//! sees (spec 14), or uses a built-in that gives one (`ask`, `5.seconds`).

use crate::Source;
use kavach_ast::{ItemKind, TypeBody};
use std::collections::HashSet;
use std::sync::OnceLock;

/// The modules: path, module name and source.
pub(crate) const BUILTIN: &[(&str, &str, &str)] = &[
    (
        "std/json.kav",
        "std.json",
        include_str!("../../../stdlib/json/json.kav"),
    ),
    (
        "std/text.kav",
        "std.text",
        include_str!("../../../stdlib/text/text.kav"),
    ),
    (
        "std/collections.kav",
        "std.collections",
        include_str!("../../../stdlib/collections/collections.kav"),
    ),
    (
        "std/math.kav",
        "std.math",
        include_str!("../../../stdlib/math/math.kav"),
    ),
    (
        "std/fs.kav",
        "std.fs",
        include_str!("../../../stdlib/fs/fs.kav"),
    ),
    (
        "std/time/digits.kav",
        "std.time.digits",
        include_str!("../../../stdlib/time/digits.kav"),
    ),
    (
        "std/time.kav",
        "std.time",
        include_str!("../../../stdlib/time/time.kav"),
    ),
    (
        "std/net.kav",
        "std.net",
        include_str!("../../../stdlib/net/net.kav"),
    ),
    (
        "std/http.kav",
        "std.http",
        include_str!("../../../stdlib/http/http.kav"),
    ),
    (
        "std/cli.kav",
        "std.cli",
        include_str!("../../../stdlib/cli/cli.kav"),
    ),
    (
        "std/http/multipart.kav",
        "std.http.multipart",
        include_str!("../../../stdlib/http/multipart.kav"),
    ),
    (
        "std/http/server.kav",
        "std.http.server",
        include_str!("../../../stdlib/http/server.kav"),
    ),
    (
        "std/http/hpack.kav",
        "std.http.hpack",
        include_str!("../../../stdlib/http/hpack.kav"),
    ),
    (
        "std/term.kav",
        "std.term",
        include_str!("../../../stdlib/term/term.kav"),
    ),
    (
        "std/log.kav",
        "std.log",
        include_str!("../../../stdlib/log/log.kav"),
    ),
    (
        "std/http/message.kav",
        "std.http.message",
        include_str!("../../../stdlib/http/message.kav"),
    ),
    (
        "std/http/h2/wire.kav",
        "std.http.h2.wire",
        include_str!("../../../stdlib/http/h2wire.kav"),
    ),
    (
        "std/http/h2.kav",
        "std.http.h2",
        include_str!("../../../stdlib/http/h2.kav"),
    ),
    (
        "std/http/h3.kav",
        "std.http.h3",
        include_str!("../../../stdlib/http/h3.kav"),
    ),
    (
        "std/ai.kav",
        "std.ai",
        include_str!("../../../stdlib/ai/ai.kav"),
    ),
    (
        "std/random.kav",
        "std.random",
        include_str!("../../../stdlib/random/random.kav"),
    ),
    (
        "std/clipboard.kav",
        "std.clipboard",
        include_str!("../../../stdlib/clipboard/clipboard.kav"),
    ),
    (
        "std/devices.kav",
        "std.devices",
        include_str!("../../../stdlib/devices/devices.kav"),
    ),
    (
        "std/agents.kav",
        "std.agents",
        include_str!("../../../stdlib/agents/agents.kav"),
    ),
    (
        "std/test.kav",
        "std.test",
        include_str!("../../../stdlib/test/test.kav"),
    ),
    (
        "std/grpc/protobuf.kav",
        "std.grpc.protobuf",
        include_str!("../../../stdlib/grpc/protobuf.kav"),
    ),
    (
        "std/grpc/frames.kav",
        "std.grpc.frames",
        include_str!("../../../stdlib/grpc/frames.kav"),
    ),
    (
        "std/grpc.kav",
        "std.grpc",
        include_str!("../../../stdlib/grpc/grpc.kav"),
    ),
];

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
            .map(|(.., text)| {
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
                    if let TypeBody::Union(vs) = &t.body {
                        out.extend(vs.iter().map(|v| v.name.name.clone()));
                    }
                }
                out
            })
            .collect()
    })
}

/// The words of `text`: its identifiers and dotted paths (`std.json`).
fn words(text: &str, into: &mut HashSet<String>) {
    let mut word = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_alphanumeric() || c == '_' || (c == '.' && !word.is_empty()) {
            word.push(c);
            continue;
        }
        if !word.is_empty() {
            let w = word.trim_end_matches('.');
            into.extend(w.split('.').map(str::to_string));
            into.insert(w.to_string());
            word.clear();
        }
    }
}

/// The modules `sources` use, directly or through each other, as sources
/// to add after them. A file given as the program that is itself a
/// standard library module counts as that module.
pub(crate) fn used(sources: &[Source]) -> Vec<Source> {
    let mut seen = HashSet::new();
    for s in sources {
        words(&s.text, &mut seen);
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
        words(text, &mut own);
        // Its durations are `std.time`'s, as a program's are.
        let mut units = kavach_checker::DURATION_UNITS.iter().map(|(u, _)| *u);
        if units.any(|u| own.contains(u)) || own.contains("timeout") {
            seen.insert("std.time".to_string());
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

    #[test]
    fn a_program_gets_only_the_modules_it_uses() {
        for (text, want) in [
            ("print(\"hi\")", vec![]),
            ("use std.json\nprint(json.stringify(1))", vec!["std.json"]),
            // A module's own `use`s come along: http needs json, fs (for
            // `http.file`), net, h2, its forms and its server, which needs
            // cli for its stop hold.
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
                    "std.http.multipart",
                    "std.http.server",
                    "std.http.hpack",
                    "std.http.message",
                    "std.http.h2.wire",
                    "std.http.h2",
                    "std.http.h3",
                ],
            ),
            // Public types and variants are seen everywhere (spec 14).
            ("fn f(e: FsError) {}", vec!["std.fs"]),
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
