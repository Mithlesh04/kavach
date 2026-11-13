//! `kavach audit` (spec 10.5) lists unsafe code, revealed secrets, calls
//! of `audited fn`s (10.15) and capability use.

use std::fs;
use std::process::Command;

#[test]
fn audit_lists_unsafe_reveal_and_capability_use() {
    let dir = std::env::temp_dir().join(format!("kavach-audit-{}", std::process::id()));
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("kavach.toml"),
        "[package]\nname = \"audit-demo\"\nunsafe = true\n",
    )
    .unwrap();
    fs::write(
        dir.join("src/main.kav"),
        r#"extern c "m" {
    fn sqrt(x: Float) -> Float
}

fn risky(x: Float) -> Float {
    unsafe {
        sqrt(x)
    }
}

fn charge(secret card: String) {
    const shown = reveal(card)
    print(shown.len())
}

type Query {
    text: String
}

literal sql: Query check check_sql

fn check_sql(text: String) -> [LiteralProblem] {
    []
}

/// SQL known only when the program runs (spec 10.15).
audited fn dynamic(text: String) -> Query {
    Query(text: text)
}

fn main() {
    print(risky(2.0))
    charge("4111")
    print(sql"select 1".text)
    print(dynamic("select 2").text)
}
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("audit")
        .arg("src/main.kav")
        .current_dir(&dir)
        .output()
        .unwrap();
    // Without a file, inside the project: the same places, relative to
    // where `kavach` runs, so the list is the same on every machine.
    let whole = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("audit")
        .current_dir(&dir)
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&dir);
    let listed = String::from_utf8_lossy(&whole.stdout);
    assert!(listed.contains("  src/main.kav:"), "{listed}");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    for needle in [
        "unsafe code:",
        "extern c \"m\" fn sqrt",
        "unsafe block",
        "revealed secrets:",
        "reveal(…)",
        "capability use:",
        "fn charge: Console",
        "audited calls:",
        "dynamic(…)",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
    }
}
