//! `std.app` (spec 14.30): a program's folders follow the system's rule
//! and are made, and a panic writes a crash report naming the functions
//! that were running. The program runs with
//! every folder variable pointing into a folder of its own in the
//! system's temporary folder, so nothing is made in the real ones.

use std::path::Path;
use std::process::Command;

const PROGRAM: &str = "use std.app
use std.fs

fn fall(n: Int) -> Int {
    if n == 0 {
        panic(\"boom\")
    }
    const below = fall(n - 1)
    below * below + n
}

fn main() ! Error uses Fs {
    const data = try app.data_dir(\"Skyfall\")
    const cache = try app.cache_dir(\"Skyfall\")
    print(`{fs.exists(data)} {fs.exists(cache)}`)
    print(data.ends_with(\"Skyfall\"))
    const bad = app.config_dir(\"a/b\") catch _ => \"refused\"
    print(bad)
    try app.crash_reports(\"crashes\")
    print(fall(40))
}
";

/// How the program runs: in the interpreter, `run --native`, or built.
#[derive(Clone, Copy, Debug)]
enum Mode {
    Interpreted,
    Native,
    Built,
}

fn run_in(dir: &Path, mode: Mode) -> (String, String) {
    let base = dir.join("home");
    let kavach = env!("CARGO_BIN_EXE_kavach");
    let mut cmd = Command::new(kavach);
    match mode {
        Mode::Interpreted => {
            cmd.args(["run", "main.kav"]);
        }
        Mode::Native => {
            cmd.args(["run", "--native", "main.kav"]);
        }
        Mode::Built => {
            let built = Command::new(kavach)
                .args(["build", "main.kav"])
                .current_dir(dir)
                .output()
                .unwrap();
            assert!(
                built.status.success(),
                "{}",
                String::from_utf8_lossy(&built.stderr)
            );
            cmd = Command::new(dir.join(if cfg!(windows) { "main.exe" } else { "main" }));
        }
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
    let out = cmd.current_dir(dir).output().unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).to_string();
    (text(&out.stdout), text(&out.stderr))
}

#[test]
fn folders_are_made_and_a_panic_writes_a_report() {
    for mode in [Mode::Interpreted, Mode::Native, Mode::Built] {
        let dir = std::env::temp_dir().join(format!("kavach-app-{mode:?}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.kav"), PROGRAM).unwrap();
        let (out, err) = run_in(&dir, mode);
        let reports: Vec<String> = std::fs::read_dir(dir.join("crashes"))
            .map(|r| {
                r.flatten()
                    .map(|f| std::fs::read_to_string(f.path()).unwrap_or_default())
                    .collect()
            })
            .unwrap_or_default();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(out, "true true\ntrue\nrefused\n", "{mode:?}: {err}");
        assert!(err.contains("panic: boom"), "{err}");
        assert_eq!(reports.len(), 1, "one report: {reports:?}");
        assert!(
            reports[0].starts_with("panic: boom") && reports[0].contains("kavach: "),
            "{}",
            reports[0]
        );
        // The functions running, innermost first (spec 14.30): `fall`
        // uses what it calls itself for twice, so no compiler turns it
        // into a loop, and 40 calls deep, more than LLVM inlines (a
        // function inlined into its caller isn't listed).
        let calls: Vec<&str> = reports[0]
            .lines()
            .filter(|l| l.starts_with("  in "))
            .collect();
        assert_eq!(
            calls.first(),
            Some(&"  in fall (main.kav:4)"),
            "{mode:?}: {}",
            reports[0]
        );
        assert_eq!(
            calls.last(),
            Some(&"  in main (main.kav:12)"),
            "{mode:?}: {}",
            reports[0]
        );
    }
}
