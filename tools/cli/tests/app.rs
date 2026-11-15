//! `std.app` (spec 14.30): a program's folders follow the system's rule
//! and are made, and a panic writes a crash report. The program runs with
//! every folder variable pointing into a folder of its own in the
//! system's temporary folder, so nothing is made in the real ones.

use std::path::Path;
use std::process::Command;

const PROGRAM: &str = "use std.app
use std.fs

fn main() ! Error uses Fs {
    const data = try app.data_dir(\"Skyfall\")
    const cache = try app.cache_dir(\"Skyfall\")
    print(\"{fs.exists(data)} {fs.exists(cache)}\")
    print(data.ends_with(\"Skyfall\"))
    const bad = app.config_dir(\"a/b\") catch _ => \"refused\"
    print(bad)
    try app.crash_reports(\"crashes\")
    panic(\"boom\")
}
";

fn run_in(dir: &Path, native: bool) -> (String, String) {
    let base = dir.join("home");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kavach"));
    cmd.arg("run");
    if native {
        cmd.arg("--native");
    }
    for v in [
        "APPDATA",
        "LOCALAPPDATA",
        "HOME",
        "XDG_DATA_HOME",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
    ] {
        cmd.env(v, base.join(v.to_lowercase()));
    }
    let out = cmd.arg("main.kav").current_dir(dir).output().unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).to_string();
    (text(&out.stdout), text(&out.stderr))
}

#[test]
fn folders_are_made_and_a_panic_writes_a_report() {
    for native in [false, true] {
        let dir = std::env::temp_dir().join(format!("kavach-app-{native}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.kav"), PROGRAM).unwrap();
        let (out, err) = run_in(&dir, native);
        let reports: Vec<String> = std::fs::read_dir(dir.join("crashes"))
            .map(|r| {
                r.flatten()
                    .map(|f| std::fs::read_to_string(f.path()).unwrap_or_default())
                    .collect()
            })
            .unwrap_or_default();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(out, "true true\ntrue\nrefused\n", "native: {native}; {err}");
        assert!(err.contains("panic: boom"), "{err}");
        assert_eq!(reports.len(), 1, "one report: {reports:?}");
        assert!(
            reports[0].starts_with("panic: boom") && reports[0].contains("kavach: "),
            "{}",
            reports[0]
        );
    }
}
