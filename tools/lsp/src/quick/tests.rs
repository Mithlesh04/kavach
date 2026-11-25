// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

use super::*;
use crate::diagnostics::{check, diagnostics};
use std::path::Path;

/// A body left out keeps its lines, so what follows keeps its place.
#[test]
fn a_body_left_out_keeps_its_lines() {
    let text =
        "pub fn a() -> Int {\n    const x = 1\n    x + 1\n}\n\nfn b() {\n    print(\"é\")\n}\n";
    let o = outline(text);
    assert_eq!(o.skeleton, "pub fn a() -> Int {}\n\nfn b() {}\n");
    let out = blank_bodies(text, &o, &[false, false]);
    assert_eq!(
        out,
        "pub fn a() -> Int {panic(\"\")\n\n\n}\n\nfn b() {\n\n}\n"
    );
    assert_eq!(
        blank_bodies(text, &o, &[true, false]).lines().nth(2),
        Some("    x + 1")
    );
}

/// An edit inside a body keeps the skeleton; one to a signature doesn't.
#[test]
fn the_skeleton_changes_only_with_a_declaration() {
    let a = outline("fn f(n: Int) -> Int {\n    n\n}\n");
    let b = outline("fn f(n: Int) -> Int {\n    n + 1 + 2\n}\n");
    let c = outline("fn f(n: Int, m: Int) -> Int {\n    n\n}\n");
    assert_eq!(a.skeleton, b.skeleton);
    assert_ne!(a.skeleton, c.skeleton);
}

/// Each file's messages from a quick check of it against the whole
/// check's, for the project in `dir`: the same, word for word and place
/// for place.
fn same_as_whole(dir: &Path) -> usize {
    let mut files = Vec::new();
    collect(&dir.join("src"), &mut files);
    let mut outlines = Outlines::default();
    for file in &files {
        let path = file.to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(file).unwrap();
        let mine = |c: &kavach_driver::Compiled| {
            diagnostics(c)
                .into_iter()
                .find(|(p, _)| crate::navigate::same_file(p, &path))
                .map(|(_, d)| d)
        };
        let whole = mine(&check(&path, &text, |_| None, None).0);
        let quick = mine(&check(&path, &text, |_| None, Some(&mut outlines)).0);
        assert_eq!(quick, whole, "{path}");
    }
    files.len()
}

fn collect(dir: &Path, into: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, into);
        } else if p.extension().is_some_and(|x| x == "kav") {
            into.push(p);
        }
    }
}

/// A project written to a fresh folder: `kavach.toml` and `src/` files.
fn project(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("kavach-quick-{name}-{}", std::process::id()));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("kavach.toml"), "[package]\nname = \"q\"\n").unwrap();
    for (file, text) in files {
        std::fs::write(root.join("src").join(file), text).unwrap();
    }
    root
}

/// What the other files' bodies tell a file: its `pub` names used only
/// there, a function used as a value there, capabilities a method uses,
/// a test file of the same module, code after a call.
#[test]
fn a_quick_check_says_what_a_whole_one_says() {
    let cases: [&[(&str, &str)]; 3] = [
        &[
            (
                "main.kav",
                "use shop\n\npub fn run(b: shop.Bag) -> Int {\n    b.show()\n    const n = shop.loud()\n    n + b.count()\n}\n\nfn main() {\n    print(run(shop.Bag(items: [1])))\n    print(shop.apply(shop.twice))\n}\n",
            ),
            (
                "shop.kav",
                "pub type Bag {\n    pub items: [Int]\n\n    fn show(self) {\n        print(self.items)\n    }\n\n    pub fn count(self) -> Int {\n        self.items.len()\n    }\n}\n\npub fn loud() -> Int uses Console {\n    print(\"loud\")\n    1\n}\n\npub fn twice(n: Int, unused: Int) -> Int {\n    n * 2\n}\n\npub fn apply(f: fn(Int, Int) -> Int) -> Int {\n    f(1, 2)\n}\n\npub fn spare() -> Int {\n    helper()\n}\n\nfn helper() -> Int {\n    3\n}\n",
            ),
        ],
        &[
            (
                "main.kav",
                "use tools\n\nfn main() {\n    print(tools.half(4))\n    print(\"after\")\n}\n",
            ),
            (
                "tools.kav",
                "pub fn half(n: Int) -> Int\n    require n > 0\n{\n    n / 2\n}\n\nfn inner() -> Int {\n    1\n}\n",
            ),
            (
                "tools.test.kav",
                "test \"inner\" {\n    assert inner() == 1\n}\n",
            ),
        ],
        &[
            (
                "main.kav",
                "use a\nuse b\n\nfn main() {\n    print(a.top())\n}\n",
            ),
            ("a.kav", "use b\n\npub fn top() -> Int {\n    b.mid()\n}\n"),
            (
                "b.kav",
                "pub fn mid() -> Int {\n    low()\n}\n\nfn low() -> Int {\n    print(\"low\")\n    1\n}\n\npub fn unused_here() -> Int {\n    2\n}\n",
            ),
        ],
    ];
    for (i, files) in cases.iter().enumerate() {
        let root = project(&format!("case{i}"), files);
        let checked = same_as_whole(&root);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(checked, files.len());
    }
}

/// Every project among the conformance tests and the examples: each
/// file says the same in a quick check as in a whole one.
#[test]
fn the_repository_s_projects_say_the_same() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut tomls = Vec::new();
    for top in ["tests/conformance", "examples"] {
        find_projects(&repo.join(top), &mut tomls);
    }
    assert!(tomls.len() > 20, "{tomls:?}");
    for dir in tomls {
        same_as_whole(&dir);
    }
}

fn find_projects(dir: &Path, into: &mut Vec<std::path::PathBuf>) {
    if dir.join("kavach.toml").is_file() && dir.join("src").is_dir() {
        into.push(dir.to_path_buf());
    }
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.path().is_dir() {
            find_projects(&e.path(), into);
        }
    }
}

/// The URIs each `publishDiagnostics` names, in order, after a talk with
/// the server.
fn talk(messages: &[serde_json::Value]) -> Vec<String> {
    let mut input = Vec::new();
    for m in messages {
        crate::rpc::write(&mut input, m).unwrap();
    }
    let mut output = Vec::new();
    crate::serve(&input[..], &mut output).unwrap();
    let mut rest = &output[..];
    let mut out = Vec::new();
    while let Some(m) = crate::rpc::read(&mut rest).unwrap() {
        let m = m.unwrap();
        if m["method"] == "textDocument/publishDiagnostics" {
            let uri = m["params"]["uri"].as_str().unwrap();
            out.push(uri.rsplit('/').next().unwrap().to_string());
        }
    }
    out
}

/// Opened: the whole program. A change inside a body: that file alone.
/// A change to a declaration, or a save: the whole program again.
#[test]
fn a_change_in_a_body_checks_its_file_alone() {
    use serde_json::json;
    let main = "use shop\n\nfn main() {\n    print(shop.count())\n}\n";
    let shop = "pub fn count() -> Int {\n    1\n}\n";
    let root = project("server", &[("main.kav", main), ("shop.kav", shop)]);
    let uri = |f: &str| crate::uri::from_path(&root.join("src").join(f).to_string_lossy());
    let notify = |method: &str, params: serde_json::Value| json!({"jsonrpc": "2.0", "method": method, "params": params});
    let edit = |line: u32, text: &str| {
        let at = json!({"line": line, "character": 0});
        notify(
            "textDocument/didChange",
            json!({"textDocument": {"uri": uri("shop.kav")}, "contentChanges": [{"range": {"start": at, "end": at}, "text": text}]}),
        )
    };
    let sent = talk(&[
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
        notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri("shop.kav"), "text": shop}}),
        ),
        edit(1, "    const x = 2\n"),
        edit(0, "pub fn other() -> Int {\n    2\n}\n\n"),
        notify(
            "textDocument/didSave",
            json!({"textDocument": {"uri": uri("shop.kav")}}),
        ),
    ]);
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        sent,
        [
            "main.kav", "shop.kav", // opened: whole
            "shop.kav", // a body: quick
            "main.kav", "shop.kav", // a declaration: whole
            "main.kav", "shop.kav", // saved: whole
        ]
    );
}

/// A body nothing in the file reaches is left out; one it calls stays,
/// and so does one that names what the file declares.
#[test]
fn bodies_nothing_reaches_are_left_out() {
    let main = "use b\n\nfn main() {\n    print(b.used())\n}\n";
    let b = "pub fn used() -> Int {\n    const n = 40\n    n + 2\n}\n\npub fn other() -> Int {\n    const n = 40\n    n + 1\n}\n";
    let c = "use b\n\npub fn calls() -> Int {\n    const n = b.other()\n    n + 1\n}\n";
    let root = project("lighten", &[("main.kav", main), ("b.kav", b), ("c.kav", c)]);
    let main_path = root.join("src").join("main.kav");
    let mut sources = kavach_driver::project_sources(&main_path).unwrap();
    let path = main_path.to_string_lossy().replace('\\', "/");
    crate::diagnostics::lighten(&mut sources, &path, &mut Outlines::default());
    let b_path = root.join("src").join("b.kav");
    let mut sources_b = kavach_driver::project_sources(&main_path).unwrap();
    let path_b = b_path.to_string_lossy().replace('\\', "/");
    crate::diagnostics::lighten(&mut sources_b, &path_b, &mut Outlines::default());
    std::fs::remove_dir_all(&root).unwrap();
    let text = |s: &[kavach_driver::Source], name: &str| {
        s.iter()
            .find(|s| s.path.ends_with(name))
            .unwrap()
            .text
            .clone()
    };
    // From main: `used` is reached, `other` and `calls` aren't.
    let b_seen = text(&sources, "b.kav");
    assert!(b_seen.contains("    n + 2\n"), "{b_seen}");
    assert!(!b_seen.contains("    n + 1\n"), "{b_seen}");
    assert!(text(&sources, "c.kav").contains("{panic(\"\")"));
    // From b: `calls` names `other`, which b declares, so it stays.
    assert!(text(&sources_b, "c.kav").contains("b.other()"));
    assert_eq!(text(&sources_b, "main.kav"), main);
}
