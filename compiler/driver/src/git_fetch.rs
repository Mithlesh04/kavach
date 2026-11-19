// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Fetching a git dependency (spec 10.12): the commit a tag, branch or
//! `rev` names in its repository, and that commit checked out into
//! `kavach_modules/<name>/`. Only `kavach update` and a first build fetch.

use crate::fetch_checks::is_commit;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs git with `args`, giving what it prints, or why it failed.
fn git(args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| format!("can't run git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The commit that `want` (a tag, branch or commit; the default branch
/// without one) names in repository `url`.
pub(crate) fn commit_of(url: &str, want: &Option<(String, String)>) -> Result<String, String> {
    let wanted = match want {
        Some((kind, rev)) if kind == "rev" => return Ok(rev.clone()),
        Some((kind, name)) if kind == "tag" => format!("refs/tags/{name}"),
        Some((_, name)) => format!("refs/heads/{name}"),
        None => "HEAD".to_string(),
    };
    // A tag that points at a tag object is listed again with `^{}`: that
    // line names the commit.
    let listed = git(&["ls-remote", "--", url, &wanted, &format!("{wanted}^{{}}")])?;
    let lines: Vec<&str> = listed.lines().collect();
    let line = lines
        .iter()
        .find(|l| l.ends_with("^{}"))
        .or(lines.first())
        .ok_or_else(|| format!("{url} has no `{wanted}`"))?;
    Ok(line.split_whitespace().next().unwrap_or("").to_string())
}

/// The folder holding `commit` of `url`: `kavach_modules/<name>/` of the
/// project at `top`, fetched (or moved to `commit`) when `fetch` allows it.
pub(crate) fn checkout(
    top: &Path,
    name: &str,
    url: &str,
    commit: &str,
    fetch: bool,
) -> Result<PathBuf, String> {
    if !is_commit(commit) {
        return Err(format!(
            "dependency `{name}`: `{commit}` in kavach.lock is not a commit: run `kavach update`"
        ));
    }
    let dir = top.join("kavach_modules").join(name);
    // A checked-out commit leaves HEAD detached, holding the commit itself
    // (in full, where `rev` may give its start).
    let head = std::fs::read_to_string(dir.join(".git").join("HEAD")).unwrap_or_default();
    // A checkout is renamed into place only once it is whole, and the
    // package may be in a folder of it (`dir`), so the commit says enough.
    if head.trim().starts_with(commit) {
        return Ok(dir);
    }
    if !fetch {
        let why = if dir.exists() {
            "is at another commit"
        } else {
            "is not fetched yet"
        };
        return Err(format!("dependency `{name}` {why}: run `kavach update`"));
    }
    let part = dir.with_extension("part");
    let _ = std::fs::remove_dir_all(&part);
    std::fs::create_dir_all(dir.parent().unwrap_or(top)).map_err(|e| e.to_string())?;
    let part_s = part.to_string_lossy().to_string();
    git(&["clone", "--quiet", "--", url, &part_s])?;
    git(&["-C", &part_s, "checkout", "--quiet", commit, "--"])?;
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&part, &dir).map_err(|e| e.to_string())?;
    Ok(dir)
}
