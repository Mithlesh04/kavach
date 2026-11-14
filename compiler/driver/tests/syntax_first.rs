//! A file with syntax errors gets only those (spec 13.6): the later
//! checks would report on code the parser couldn't read.

use kavach_driver::{Options, Severity, compile};

#[test]
fn syntax_errors_come_alone() {
    // A string broken by a raw line break swallows what follows, so the
    // checks after parsing would see a long `main` and a missing function.
    let mut src = String::from("use std.fs\n\nfn main() ! Error {\n");
    src.push_str("    const w = try fs.create_writer(\"export.csv\")\n");
    src.push_str("    try w.write_text(\"id,total\n\")\n");
    for i in 0..90 {
        src.push_str(&format!("    print({i})\n"));
    }
    src.push_str("}\n");
    let c = compile(&src, "main.kav", Options::default());
    let shown: Vec<&str> = c.messages.iter().map(|m| m.text.as_str()).collect();
    assert!(
        c.messages.iter().all(|m| m.severity == Severity::Error),
        "{shown:?}"
    );
    assert!(
        shown.iter().any(|t| t.starts_with("text can't continue")),
        "{shown:?}"
    );
    assert!(
        !shown
            .iter()
            .any(|t| t.contains("create_writer") || t.contains("lines")),
        "{shown:?}"
    );
}
