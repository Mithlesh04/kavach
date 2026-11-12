//! Workspaces and packages in a folder of a repository (spec 10.12 and
//! 10.14, decision 0091): members share one `kavach.lock` and one
//! `kavach_modules/` at the root, use each other by `path`, and `check`
//! and `build` at the root run on each member; a git dependency's `dir`
//! picks its package, and a `dir` leaving the repository is refused.

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

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn members_share_one_lock_and_run_together() {
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("kavach-workspaces-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);

    // A repository holding a package in a folder of it.
    let repo = dir.join("repo");
    write(
        &repo.join("packages/greet/kavach.toml"),
        "[package]\nname = \"greet\"\n",
    );
    write(
        &repo.join("packages/greet/src/lib.kav"),
        "pub fn hello(n: String) -> String {\n    \"hello, {n}\"\n}\n",
    );
    git(&repo, &["init", "-q"]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "one"]);
    let url = repo.to_string_lossy().replace('\\', "/");

    // The workspace: a library and an app that uses it and the package.
    let ws = dir.join("ws");
    write(
        &ws.join("kavach.toml"),
        "[workspace]\nmembers = [\"packages/*\"]\n",
    );
    write(
        &ws.join("packages/router/kavach.toml"),
        "[package]\nname = \"router\"\n",
    );
    write(
        &ws.join("packages/router/src/lib.kav"),
        "pub fn route(p: String) -> String {\n    \"route {p}\"\n}\n",
    );
    let app = ws.join("packages/app");
    write(
        &app.join("kavach.toml"),
        &format!(
            "[package]\nname = \"app\"\n\n[dependencies]\nrouter = {{ path = \"../router\" }}\ngreet = {{ git = \"{url}\", dir = \"packages/greet\" }}\n"
        ),
    );
    write(
        &app.join("src/main.kav"),
        "use router\nuse greet\n\nfn main() {\n    print(router.route(\"/\"))\n    print(greet.hello(\"ws\"))\n}\n",
    );

    let updated = kavach(&app, &["update"]);
    assert!(updated.status.success(), "{}", text(&updated));
    // One lock and one `kavach_modules/`, at the root.
    assert!(ws.join("kavach.lock").is_file());
    assert!(ws.join("kavach_modules/greet").is_dir());
    assert!(!app.join("kavach.lock").exists() && !app.join("kavach_modules").exists());
    let ran = kavach(&app, &["run"]);
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        "route /\nhello, ws\n",
        "{}",
        text(&ran)
    );

    // At the root: each member in the order listed, folders sorted.
    let checked = kavach(&ws, &["check"]);
    assert!(checked.status.success(), "{}", text(&checked));
    let out = text(&checked);
    let (a, r) = (
        out.find("== packages/app").unwrap(),
        out.find("== packages/router").unwrap(),
    );
    assert!(a < r, "{out}");
    let built = kavach(&ws, &["build"]);
    assert!(built.status.success(), "{}", text(&built));
    assert!(text(&built).contains("a library"), "{}", text(&built));

    // A member that fails makes the whole command fail.
    write(
        &ws.join("packages/router/src/lib.kav"),
        "pub fn route(p: String) -> String {\n    p + 1\n}\n",
    );
    let broken = kavach(&ws, &["check"]);
    assert!(!broken.status.success(), "{}", text(&broken));

    // A `dir` leaving the repository is refused.
    write(
        &app.join("kavach.toml"),
        &format!(
            "[package]\nname = \"app\"\n\n[dependencies]\ngreet = {{ git = \"{url}\", dir = \"../secrets\" }}\n"
        ),
    );
    let refused = kavach(&app, &["update"]);
    assert!(!refused.status.success());
    assert!(
        text(&refused).contains("inside the repository"),
        "{}",
        text(&refused)
    );
    let _ = fs::remove_dir_all(&dir);
}
