//! Every conformance test and example must resolve as the spec says.

use kavach_resolver::resolve_modules;
use std::fs;
use std::path::{Path, PathBuf};

/// The standard library modules every program has, as the driver adds
/// them after the program's own file.
const BUILTIN: [(&str, &str); 24] = [
    ("std.json", include_str!("../../../stdlib/json/json.kav")),
    ("std.text", include_str!("../../../stdlib/text/text.kav")),
    (
        "std.collections",
        include_str!("../../../stdlib/collections/collections.kav"),
    ),
    ("std.math", include_str!("../../../stdlib/math/math.kav")),
    ("std.fs", include_str!("../../../stdlib/fs/fs.kav")),
    (
        "std.time.digits",
        include_str!("../../../stdlib/time/digits.kav"),
    ),
    ("std.time", include_str!("../../../stdlib/time/time.kav")),
    ("std.net", include_str!("../../../stdlib/net/net.kav")),
    ("std.http", include_str!("../../../stdlib/http/http.kav")),
    (
        "std.http.message",
        include_str!("../../../stdlib/http/message.kav"),
    ),
    (
        "std.http.hpack",
        include_str!("../../../stdlib/http/hpack.kav"),
    ),
    ("std.term", include_str!("../../../stdlib/term/term.kav")),
    ("std.log", include_str!("../../../stdlib/log/log.kav")),
    (
        "std.http.h2.wire",
        include_str!("../../../stdlib/http/h2wire.kav"),
    ),
    ("std.http.h2", include_str!("../../../stdlib/http/h2.kav")),
    ("std.ai", include_str!("../../../stdlib/ai/ai.kav")),
    (
        "std.random",
        include_str!("../../../stdlib/random/random.kav"),
    ),
    ("std.test", include_str!("../../../stdlib/test/test.kav")),
    (
        "std.clipboard",
        include_str!("../../../stdlib/clipboard/clipboard.kav"),
    ),
    (
        "std.devices",
        include_str!("../../../stdlib/devices/devices.kav"),
    ),
    (
        "std.agents",
        include_str!("../../../stdlib/agents/agents.kav"),
    ),
    (
        "std.grpc.protobuf",
        include_str!("../../../stdlib/grpc/protobuf.kav"),
    ),
    (
        "std.grpc.frames",
        include_str!("../../../stdlib/grpc/frames.kav"),
    ),
    ("std.grpc", include_str!("../../../stdlib/grpc/grpc.kav")),
];

/// `src` followed by the standard library modules, and every module (the
/// program's own first).
fn with_builtins(src: &str) -> (String, Vec<kavach_resolver::Module>) {
    let mut all = String::new();
    let mut modules = Vec::new();
    for (name, text) in [("main", src)].into_iter().chain(BUILTIN) {
        let start = all.len() as u32;
        all.push_str(text);
        if !all.ends_with('\n') {
            all.push('\n');
        }
        modules.push(kavach_resolver::Module {
            name: name.into(),
            start,
            end: all.len() as u32,
        });
    }
    (all, modules)
}

fn kav_files(dir: &Path, out: &mut Vec<PathBuf>) {
    // Project folders (several modules) are checked by the conformance
    // runner, which compiles them together.
    if dir.join("kavach.toml").is_file() && dir.join("src").is_dir() {
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            kav_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "kav") {
            out.push(path);
        }
    }
}

/// Files that parse cleanly must have no resolver errors, unless they are
/// compile-error tests: then errors may only be on the line marked
/// `// error:`. For the rules the resolver owns, it must find the error.
#[test]
fn conformance_tests_and_examples_resolve() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    kav_files(&root.join("tests/conformance"), &mut files);
    kav_files(&root.join("examples"), &mut files);

    let resolver_rules = [
        "let-cannot-be-reassigned.kav",
        "no-duplicate-name-in-scope.kav",
        "variables-use-snake-case.kav",
        "types-use-pascal-case.kav",
        "unknown-name.kav",
        "locals-only-after-let.kav",
        "parameters-cannot-be-reassigned.kav",
    ];
    let mut problems = Vec::new();
    let mut checked = 0;
    for path in &files {
        let src = fs::read_to_string(path).unwrap();
        let (all, modules) = with_builtins(&src);
        let parsed = kavach_parser::parse(&all);
        if !parsed.errors.is_empty() {
            continue; // syntax-error tests belong to the parser
        }
        checked += 1;
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let marked = src
            .lines()
            .position(|l| l.contains("// error:"))
            .map(|i| i + 1);
        let errors: Vec<(usize, String)> = resolve_modules(&parsed.file, &modules)
            .errors
            .iter()
            .filter(|e| e.span.start < modules[0].end)
            .map(|e| {
                (
                    src[..e.span.start as usize].matches('\n').count() + 1,
                    e.message.clone(),
                )
            })
            .collect();
        let on_marked_line = |line: usize| errors.iter().all(|(l, _)| *l == line);
        let ok = match marked {
            None => errors.is_empty(),
            Some(line) if resolver_rules.contains(&name.as_str()) => {
                !errors.is_empty() && on_marked_line(line)
            }
            Some(line) => on_marked_line(line),
        };
        if !ok {
            problems.push(format!("{name} (marked line {marked:?}): {errors:?}"));
        }
    }
    assert!(checked > 90, "only {checked} files checked");
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}
