// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What built a program (spec 10.5): `kavach --version`, `kavach version
//! <program>` with how it was built, and `--version` answered by the
//! program only when `[build] version_flag` says, however it runs.

use std::path::Path;
use std::process::Command;

fn kavach(dir: &Path, args: &[&str]) -> String {
    // A fixed build moment, and the options as `kavach build` is given
    // them, so what a built program says is known exactly.
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(args)
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "86400")
        .env_remove("KAVACH_BACKEND")
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// `kavach build` in `dir`, which must make the program: on Windows under
/// a full test run it once said "built" with no program there, so a
/// failure shows what the build itself said.
fn build(dir: &Path) {
    let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .arg("build")
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "86400")
        .env_remove("KAVACH_BACKEND")
        .output()
        .unwrap();
    let exe = dir
        .join(".kavach/bin")
        .join(format!("app{}", std::env::consts::EXE_SUFFIX));
    assert!(
        out.status.success() && exe.is_file(),
        "`kavach build` exited {:?}, program there: {}\n{}{}",
        out.status.code(),
        exe.is_file(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn program(dir: &Path, arg: &str) -> String {
    let exe = dir
        .join(".kavach/bin")
        .join(format!("app{}", std::env::consts::EXE_SUFFIX));
    if !exe.is_file() {
        // Built again, to show why it wasn't.
        let out = Command::new(env!("CARGO_BIN_EXE_kavach"))
            .arg("build")
            .current_dir(dir)
            .output()
            .unwrap();
        panic!(
            "`kavach build` made no {}: {}{}",
            exe.display(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
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
    // A built executable also says when, for what and how (`kavach run`
    // keeps none, so says only the first two lines).
    let system = format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS);
    let built = format!("{says}built 1970-01-02T00:00:00Z for {system}\n");

    // In Cargo's own folder for this test's files, not the system's temp
    // folder: on Windows a new program there was sometimes not yet on
    // disk when the test looked for it, under a full test run.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("kavach-version-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("src")).unwrap();
    let toml = "[package]\nname = \"app\"\nversion = \"2.3.0\"\n";
    std::fs::write(dir.join("kavach.toml"), toml).unwrap();
    let main = "use std.cli\n\nfn main() {\n    print(cli.args())\n}\n";
    std::fs::write(dir.join("src/main.kav"), main).unwrap();

    // Without the setting, `--version` is the program's own argument.
    build(&dir);
    assert_eq!(program(&dir, "--version"), "[\"--version\"]\n");
    assert_eq!(
        kavach(&dir, &["run", "--", "--version"]),
        "[\"--version\"]\n"
    );
    let exe = format!(".kavach/bin/app{}", std::env::consts::EXE_SUFFIX);
    assert_eq!(kavach(&dir, &["version", &exe]), built);

    // With it, every way the program runs answers it before `main`.
    std::fs::write(
        dir.join("kavach.toml"),
        format!("{toml}\n[build]\nversion_flag = true\n"),
    )
    .unwrap();
    build(&dir);
    assert_eq!(program(&dir, "--version"), built);
    assert_eq!(program(&dir, "x"), "[\"x\"]\n");
    assert_eq!(kavach(&dir, &["run", "--", "--version"]), says);
    assert_eq!(kavach(&dir, &["run", "--native", "--", "--version"]), says);
    // The options that change what is built are named.
    kavach(&dir, &["build", "--compress"]);
    let compressed = format!("{says}built 1970-01-02T00:00:00Z for {system} with --compress\n");
    assert_eq!(kavach(&dir, &["version", &exe]), compressed);
    std::fs::remove_dir_all(&dir).unwrap();
}
