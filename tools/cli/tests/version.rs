//! What built a program (spec 10.5): `kavach --version`, `kavach version
//! <program>`, and `--version` answered by the program only when `[build]
//! version_flag` says, however it runs.

use std::path::Path;
use std::process::Command;

fn kavach(dir: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn program(dir: &Path, arg: &str) -> String {
    let exe = dir
        .join(".kavach/bin")
        .join(format!("app{}", std::env::consts::EXE_SUFFIX));
    let out = Command::new(exe).arg(arg).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn a_program_says_what_built_it_and_answers_version_only_when_asked_to() {
    let compiler = kavach(Path::new("."), &["--version"]);
    assert!(
        compiler.starts_with("kavach 0.") && compiler.contains(" (commit "),
        "{compiler}"
    );
    let version = compiler.trim_start_matches("kavach ").trim();
    let says = format!("app 2.3.0\nbuilt by kavach {version}\n");

    let dir = std::env::temp_dir().join(format!("kavach-version-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("src")).unwrap();
    let toml = "[package]\nname = \"app\"\nversion = \"2.3.0\"\n";
    std::fs::write(dir.join("kavach.toml"), toml).unwrap();
    let main = "use std.cli\n\nfn main() {\n    print(cli.args())\n}\n";
    std::fs::write(dir.join("src/main.kav"), main).unwrap();

    // Without the setting, `--version` is the program's own argument.
    kavach(&dir, &["build"]);
    assert_eq!(program(&dir, "--version"), "[\"--version\"]\n");
    assert_eq!(
        kavach(&dir, &["run", "--", "--version"]),
        "[\"--version\"]\n"
    );
    let exe = format!(".kavach/bin/app{}", std::env::consts::EXE_SUFFIX);
    assert_eq!(kavach(&dir, &["version", &exe]), says);

    // With it, every way the program runs answers it before `main`.
    std::fs::write(
        dir.join("kavach.toml"),
        format!("{toml}\n[build]\nversion_flag = true\n"),
    )
    .unwrap();
    kavach(&dir, &["build"]);
    assert_eq!(program(&dir, "--version"), says);
    assert_eq!(program(&dir, "x"), "[\"x\"]\n");
    assert_eq!(kavach(&dir, &["run", "--", "--version"]), says);
    assert_eq!(kavach(&dir, &["run", "--native", "--", "--version"]), says);
    std::fs::remove_dir_all(&dir).unwrap();
}
