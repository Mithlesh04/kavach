//! Dependencies from git (spec 10.12): `kavach add` fetches one and writes
//! `kavach.lock` with its exact commit; later builds keep that commit,
//! even after the repository moves on, until `kavach update`; and a
//! package that uses `unsafe` is refused until its entry allows it.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn kavach(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git {args:?}");
}

fn project(dir: &Path, name: &str, extra: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    let toml =
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}");
    fs::write(dir.join("kavach.toml"), toml).unwrap();
}

fn stdout(out: &Output) -> String {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn a_git_dependency_stays_at_its_locked_commit() {
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("kavach-packages-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let (lib, app) = (dir.join("money"), dir.join("app"));

    // A library in a repository of its own: version 1, then version 2.
    project(&lib, "money", "");
    fs::write(
        lib.join("src/lib.kav"),
        "pub fn cents(n: Int) -> Int {\n    n * 100\n}\n",
    )
    .unwrap();
    git(&lib, &["init", "-q"]);
    git(&lib, &["add", "."]);
    git(&lib, &["commit", "-qm", "one"]);

    project(&app, "app", "");
    fs::write(
        app.join("src/main.kav"),
        "use money\n\nfn main() {\n    print(money.cents(3))\n}\n",
    )
    .unwrap();
    let url = lib.to_string_lossy().replace('\\', "/");
    let added = kavach(&app, &["add", "money", "--git", &url]);
    assert!(
        added.status.success(),
        "{}",
        String::from_utf8_lossy(&added.stderr)
    );
    let lock = fs::read_to_string(app.join("kavach.lock")).unwrap();
    assert!(
        lock.contains("name = \"money\"") && lock.contains("commit = \""),
        "{lock}"
    );
    assert_eq!(stdout(&kavach(&app, &["run", "src/main.kav"])), "300\n");
    // Fetched into the project, as npm's node_modules.
    let fetched = app.join("kavach_modules").join("money");
    assert!(fetched.join("src/lib.kav").is_file());

    // The repository moves on; the lock keeps the build where it was.
    fs::write(
        lib.join("src/lib.kav"),
        "pub fn cents(n: Int) -> Int {\n    n * 1000\n}\n",
    )
    .unwrap();
    git(&lib, &["commit", "-qam", "two"]);
    assert_eq!(stdout(&kavach(&app, &["run", "src/main.kav"])), "300\n");
    assert!(kavach(&app, &["update"]).status.success());
    assert_eq!(stdout(&kavach(&app, &["run", "src/main.kav"])), "3000\n");
    // One folder for each package, moved to the new commit.
    let folders = fs::read_dir(app.join("kavach_modules")).unwrap().count();
    assert_eq!(folders, 1);
    assert!(
        fs::read_to_string(fetched.join("src/lib.kav"))
            .unwrap()
            .contains("1000")
    );

    // A source that isn't safe to fetch is refused, and not added.
    let plain = kavach(&app, &["add", "evil", "--git", "http://example.com/evil"]);
    assert!(!plain.status.success());
    assert!(String::from_utf8_lossy(&plain.stderr).contains("https://"));
    let secret = kavach(
        &app,
        &["add", "evil", "--git", "https://me:token@example.com/evil"],
    );
    assert!(String::from_utf8_lossy(&secret.stderr).contains("password or token"));
    assert!(
        !fs::read_to_string(app.join("kavach.toml"))
            .unwrap()
            .contains("evil")
    );

    // A package that uses `unsafe`, refused until its entry allows it.
    let raw = dir.join("raw");
    project(&raw, "raw", "unsafe = true\n");
    fs::write(raw.join("src/lib.kav"), "pub fn one() -> Int {\n    1\n}\n").unwrap();
    let refused = kavach(&app, &["add", "raw", "--path", "../raw"]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("uses `unsafe` code"));
    assert!(
        !fs::read_to_string(app.join("kavach.toml"))
            .unwrap()
            .contains("raw")
    );
    assert!(
        kavach(&app, &["add", "raw", "--path", "../raw", "--unsafe"])
            .status
            .success()
    );

    let _ = fs::remove_dir_all(&dir);
}
