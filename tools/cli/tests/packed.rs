//! Packed executables (spec 10.5, decision 0081): `kavach build
//! --compress` and `--compress=max` write a smaller file that behaves
//! exactly as the program built without them, on its first start (which
//! unpacks it) and later ones (which find it in the cache), and that
//! `kavach version` still reads. Also the project form, `kavach build
//! --compress` inside a project.

use std::path::Path;
use std::process::{Command, Output};

const PROGRAM: &str = "use std.cli\n\nfn main() {\n    print(\"hello\")\n    for a in cli.args() {\n        print(\"arg {a}\")\n    }\n    if cli.args().len() > 1 {\n        panic(\"asked to stop\")\n    }\n}\n";

fn kavach(dir: &Path, args: &[&str]) -> Output {
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "kavach {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// What a run shows: its exit code, output and error output.
fn run(exe: &Path, cache: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(exe)
        .args(args)
        .env("KAVACH_UNPACKED", cache)
        .output()
        .unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    (out.status.code(), text(&out.stdout), text(&out.stderr))
}

#[test]
fn a_packed_program_is_smaller_and_runs_the_same() {
    let dir = std::env::temp_dir().join(format!("kavach-packed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("app.kav"), PROGRAM).unwrap();
    let exe = |name: &str| dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    let cache = dir.join("cache");

    kavach(
        &dir,
        &["build", "app.kav", "-o", &exe("plain").to_string_lossy()],
    );
    let plain_size = std::fs::metadata(exe("plain")).unwrap().len();
    for (flag, name) in [("--compress", "fast"), ("--compress=max", "max")] {
        let path = exe(name).to_string_lossy().into_owned();
        let built = kavach(&dir, &["build", flag, "app.kav", "-o", &path]);
        assert!(String::from_utf8_lossy(&built.stderr).contains("packed: "));
        let size = std::fs::metadata(exe(name)).unwrap().len();
        assert!(size * 10 < plain_size * 6, "{flag}: {size} of {plain_size}");
        for args in [&[][..], &["one", "two words"][..]] {
            let want = run(&exe("plain"), &cache, args);
            // The first start unpacks into the cache, the second finds it.
            assert_eq!(run(&exe(name), &cache, args), want, "{flag} {args:?}");
            assert_eq!(run(&exe(name), &cache, args), want, "{flag} {args:?}");
        }
        let says = kavach(&dir, &["version", &path]);
        assert!(String::from_utf8_lossy(&says.stdout).starts_with("app\nbuilt by kavach "));
    }
    assert!(std::fs::read_dir(&cache).unwrap().count() >= 1);

    // In a project, `--compress` comes before the file as `--lib` does.
    let project = dir.join("proj");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(project.join("kavach.toml"), "[package]\nname = \"proj\"\n").unwrap();
    std::fs::write(project.join("src/main.kav"), PROGRAM).unwrap();
    kavach(&project, &["build", "--compress"]);
    let built = project
        .join(".kavach/bin")
        .join(format!("proj{}", std::env::consts::EXE_SUFFIX));
    assert_eq!(run(&built, &cache, &["x"]).1, "hello\narg x\n");
    let _ = std::fs::remove_dir_all(&dir);
}
