// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Dependencies (spec 10.12): other Kavach packages a project uses, from a
//! folder (`path`) or a git repository (`git`, with a `tag`, `branch` or
//! `rev`). `kavach.lock` records the exact commit of each git dependency,
//! so every build uses the same code; fetched repositories are kept in
//! `kavach_modules/<name>/` of the project, or of its workspace (spec
//! 10.14). Only fetching (`resolve` with `fetch`) uses the network;
//! compiling reads the lock and the folders.

use crate::fetch_checks::{is_commit, is_device_name, unsafe_url};
use crate::git_fetch::{checkout, commit_of};
use crate::lock_file::{lock_text, plain, read_lock};
use crate::project::Source;
use std::path::{Path, PathBuf};

/// Where a dependency comes from.
#[derive(Clone, Debug, PartialEq)]
enum From {
    Path(PathBuf),
    /// The repository, the tag, branch or commit asked for, and the
    /// package's folder in it (`dir`).
    Git(String, Option<(String, String)>, Option<String>),
}

/// A dependency as `kavach.toml` declares it.
struct Dep {
    name: String,
    from: From,
    allow_unsafe: bool,
    /// The capabilities its entry grants (`can`).
    can: Vec<String>,
}

/// A dependency found: its name, its folder, and whether it may use
/// `unsafe` (its own `unsafe = true`, allowed by the package using it).
#[derive(Clone, Debug)]
pub struct Package {
    pub name: String,
    pub root: PathBuf,
    pub allow_unsafe: bool,
    /// The capabilities it may use: what its entry grants, and for a
    /// dependency of a dependency no more than that one was granted.
    pub granted: Vec<String>,
    /// The repository and commit, for a git dependency.
    pub(crate) git: Option<(String, String)>,
}

/// Whether `name` can be used as a module name: `use name`.
fn is_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn read_toml(root: &Path) -> Result<toml::Table, String> {
    let file = root.join("kavach.toml");
    let text = std::fs::read_to_string(&file)
        .map_err(|e| format!("can't read {}: {e}", file.display()))?;
    text.parse::<toml::Table>()
        .map_err(|e| format!("{}: {e}", file.display()))
}

/// The `[package] name` of the project at `root`, when it can name a
/// module: inside its own project too, a library's root module has that
/// name (spec 10.12).
pub(crate) fn own_name(root: &Path) -> Option<String> {
    let table = read_toml(root).ok()?;
    let name = table.get("package")?.get("name")?.as_str()?;
    is_name(name).then(|| name.to_string())
}

/// The dependencies `root/kavach.toml` declares.
fn declared(root: &Path) -> Result<Vec<Dep>, String> {
    let table = read_toml(root)?;
    let Some(deps) = table.get("dependencies").and_then(|d| d.as_table()) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (name, entry) in deps {
        let wrong = |why: &str| {
            format!(
                "dependency `{name}` in {}: {why}",
                root.join("kavach.toml").display()
            )
        };
        if !is_name(name) {
            return Err(wrong(
                "its name is used as a module name: lowercase letters, digits and `_`",
            ));
        }
        if is_device_name(name) {
            return Err(wrong(
                "Windows keeps this name for a device, so it can't be a folder",
            ));
        }
        let Some(t) = entry.as_table() else {
            return Err(wrong(
                "there is no package registry yet: give a `path` or `git` (spec 10.12)",
            ));
        };
        let text = |key: &str| t.get(key).and_then(|v| v.as_str()).map(str::to_string);
        let from = match (text("path"), text("git")) {
            (Some(p), None) => From::Path(root.join(p)),
            (None, Some(url)) => {
                let wants: Vec<(String, String)> = ["tag", "branch", "rev"]
                    .iter()
                    .filter_map(|k| text(k).map(|v| (k.to_string(), v)))
                    .collect();
                if wants.len() > 1 {
                    return Err(wrong("give at most one of `tag`, `branch` and `rev`"));
                }
                if let Some(why) = unsafe_url(&url) {
                    return Err(wrong(&format!("can't fetch `{url}`: {why}")));
                }
                if let Some((kind, v)) = wants.first() {
                    let bad = if kind == "rev" {
                        !is_commit(v)
                    } else {
                        v.starts_with('-') || v.chars().any(char::is_control)
                    };
                    if bad {
                        return Err(wrong(&format!("`{kind} = \"{v}\"` is not a {kind} name")));
                    }
                }
                let dir = text("dir");
                if let Some(d) = &dir
                    && (d.starts_with('/')
                        || d.starts_with('-')
                        || d.contains('\\')
                        || d.split('/').any(|p| p == ".." || is_device_name(p)))
                {
                    return Err(wrong(&format!(
                        "`dir = \"{d}\"` must be a folder inside the repository, written with `/`"
                    )));
                }
                From::Git(url, wants.into_iter().next(), dir)
            }
            _ => return Err(wrong("give either `path` or `git`")),
        };
        let allow_unsafe = t.get("unsafe").and_then(|v| v.as_bool()).unwrap_or(false);
        let can: Vec<String> = match t.get("can") {
            None => Vec::new(),
            Some(toml::Value::Array(items)) => items
                .iter()
                .map(|v| v.as_str().map(str::to_string))
                .collect::<Option<_>>()
                .ok_or_else(|| {
                    wrong("`can` lists capability names as text, such as `can = [\"Net\"]`")
                })?,
            Some(_) => return Err(wrong("`can` is a list, such as `can = [\"Net\"]`")),
        };
        if let Some(bad) = can.iter().find(|c| {
            !kavach_resolver::CAPABILITIES.contains(&c.as_str())
                && !crate::grants::NARROW.contains(&c.as_str())
        }) {
            let known = kavach_resolver::CAPABILITIES.join(", ");
            return Err(wrong(&format!(
                "`{bad}` in `can` is not a capability: they are {known}, and `Fs.read` or `Fs.write` for part of `Fs` (spec 8.1)"
            )));
        }
        out.push(Dep {
            name: name.clone(),
            from,
            allow_unsafe,
            can,
        });
    }
    Ok(out)
}

/// Every dependency of the project at `top`, its dependencies' too, each
/// once. With `fetch`, git dependencies missing from the lock or the
/// folder are fetched and `kavach.lock` is written; without, they must
/// be there already.
pub fn resolve(top: &Path, fetch: bool) -> Result<Vec<Package>, String> {
    let top = &plain(&std::fs::canonicalize(top).unwrap_or_else(|_| top.to_path_buf()));
    // A workspace keeps one lock and one `kavach_modules/` (spec 10.14).
    let home = &crate::workspace::home(top);
    let locked = read_lock(home);
    let mut found: Vec<Package> = Vec::new();
    // Each with the folder that names it, and what that one was granted
    // (`None` for the project itself, which may use anything).
    let mut queue: Vec<(PathBuf, Dep, Option<Vec<String>>)> = declared(top)?
        .into_iter()
        .map(|d| (top.to_path_buf(), d, None))
        .collect();
    while let Some((from_root, dep, above)) = queue.pop() {
        let granted: Vec<String> = match &above {
            None => dep.can.clone(),
            Some(a) => dep
                .can
                .iter()
                .flat_map(|c| crate::grants::meet(c, a))
                .collect(),
        };
        let (root, git) = match &dep.from {
            From::Path(p) => (
                plain(&std::fs::canonicalize(p).map_err(|_| {
                    format!("dependency `{}`: no folder {}", dep.name, p.display())
                })?),
                None,
            ),
            From::Git(url, want, dir) => {
                let commit = match locked.get(&dep.name) {
                    Some((u, c)) if u == url => c.clone(),
                    _ if fetch => commit_of(url, want)?,
                    _ => {
                        return Err(format!(
                            "dependency `{}` is not in kavach.lock: run `kavach update`",
                            dep.name
                        ));
                    }
                };
                let fetched = checkout(home, &dep.name, url, &commit, fetch)?;
                let root = match dir {
                    Some(d) => fetched.join(d),
                    None => fetched,
                };
                if !root.join("kavach.toml").is_file() {
                    return Err(format!(
                        "dependency `{}`: {url} has no package at `{}`",
                        dep.name,
                        dir.as_deref().unwrap_or(".")
                    ));
                }
                (root, Some((url.clone(), commit)))
            }
        };
        // A fetched package reads nothing outside its own folder.
        if from_root.starts_with(home.join("kavach_modules")) && !root.starts_with(&from_root) {
            return Err(format!(
                "dependency `{}` of {} is a folder outside it ({}): a fetched package can't name one (spec 10.12)",
                dep.name,
                from_root.display(),
                root.display()
            ));
        }
        let local = top.join("src");
        if from_root == *top
            && (local.join(format!("{}.kav", dep.name)).is_file() || local.join(&dep.name).is_dir())
        {
            return Err(format!(
                "the module `{0}` in src/ has the name of the dependency `{0}`: rename one of them (spec 10.2)",
                dep.name
            ));
        }
        if let Some(seen) = found.iter().find(|p| p.name == dep.name) {
            if seen.root != root {
                return Err(format!(
                    "two packages are named `{}` ({} and {}): rename one in its `[dependencies]`",
                    dep.name,
                    seen.root.display(),
                    root.display()
                ));
            }
            continue;
        }
        if !root.join("src").join("lib.kav").is_file() {
            return Err(format!(
                "dependency `{}` is not a library: {} has no src/lib.kav",
                dep.name,
                root.display()
            ));
        }
        let own_unsafe = read_toml(&root)?
            .get("package")
            .and_then(|p| p.get("unsafe"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if own_unsafe && !dep.allow_unsafe {
            return Err(format!(
                "dependency `{}` uses `unsafe` code: write `unsafe = true` in its entry in {} to allow it (spec 10.12)",
                dep.name,
                from_root.join("kavach.toml").display()
            ));
        }
        queue.extend(
            declared(&root)?
                .into_iter()
                .map(|d| (root.clone(), d, Some(granted.clone()))),
        );
        found.push(Package {
            name: dep.name,
            root,
            allow_unsafe: own_unsafe,
            granted,
            git,
        });
    }
    if fetch && !found.is_empty() {
        let text = lock_text(home, &found, (home != top).then_some(&locked));
        let lock = home.join("kavach.lock");
        if std::fs::read_to_string(&lock).ok().as_deref() != Some(text.as_str()) {
            std::fs::write(&lock, text)
                .map_err(|e| format!("can't write {}: {e}", lock.display()))?;
        }
    }
    Ok(found)
}

/// The files of each dependency of the project at `top`, as modules of
/// their package: `src/lib.kav` is the module named after the dependency,
/// `src/shapes.kav` is `<name>.shapes`. Their test files are left out.
pub fn sources(top: &Path) -> Vec<Source> {
    let mut out = Vec::new();
    for p in resolve(top, false).unwrap_or_default() {
        let src = p.root.join("src");
        let mut files = Vec::new();
        crate::project::kav_files(&src, &mut files);
        for f in files {
            let Ok(rel) = f.strip_prefix(&src) else {
                continue;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            if rel.ends_with(".test.kav") || rel == "main.kav" {
                continue;
            }
            let inner = rel.trim_end_matches(".kav").replace('/', ".");
            let module = if inner == "lib" {
                p.name.clone()
            } else {
                format!("{}.{inner}", p.name)
            };
            let Ok(text) = std::fs::read_to_string(&f) else {
                continue;
            };
            out.push(Source {
                path: f.to_string_lossy().replace('\\', "/"),
                module,
                text,
                package: p.name.clone(),
            });
        }
    }
    out
}
