//! Records the commit the compiler is built from (spec 10.5): `kavach
//! --version` prints it, and every executable `kavach build` writes
//! carries it. `KAVACH_COMMIT` set by hand wins; without git it is
//! `unknown`.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    let text = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (out.status.success() && !text.is_empty()).then_some(text)
}

fn main() {
    println!("cargo:rerun-if-env-changed=KAVACH_COMMIT");
    // A new commit, or a switch to another branch, builds this again.
    for path in ["HEAD", "refs/heads", "packed-refs"] {
        if let Some(p) = git(&["rev-parse", "--git-path", path]) {
            println!("cargo:rerun-if-changed={p}");
        }
    }
    let commit = std::env::var("KAVACH_COMMIT")
        .ok()
        .or_else(|| git(&["rev-parse", "--short=8", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=KAVACH_COMMIT={commit}");
}
