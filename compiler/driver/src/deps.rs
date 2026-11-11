//! Dependencies (spec 10.12): other Kavach packages a project uses, from a
//! folder (`path`) or a git repository (`git`, with a `tag`, `branch` or
//! `rev`). `kavach.lock` records the exact commit of each git dependency,
//! so every build uses the same code; fetched repositories are kept in
//! `kavach_modules/<name>/` of the project. Only fetching (`resolve` with
//! `fetch`) uses the network; compiling reads the lock and the folders.

use crate::fetch_checks::{is_commit, is_device_name, unsafe_url};
use crate::project::Source;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where a dependency comes from.
#[derive(Clone, Debug, PartialEq)]
enum From {
    Path(PathBuf),
    /// The repository, and the tag, branch or commit asked for.
    Git(String, Option<(String, String)>),
}

/// A dependency as `kavach.toml` declares it.
struct Dep {
    name: String,
    from: From,
    allow_unsafe: bool,
}

/// A dependency found: its name, its folder, and whether it may use
/// `unsafe` (its own `unsafe = true`, allowed by the package using it).
#[derive(Clone, Debug)]
pub struct Package {
    pub name: String,
    pub root: PathBuf,
    pub allow_unsafe: bool,
    /// The repository and commit, for a git dependency.
    git: Option<(String, String)>,
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
                From::Git(url, wants.into_iter().next())
            }
            _ => return Err(wrong("give either `path` or `git`")),
        };
        let allow_unsafe = t.get("unsafe").and_then(|v| v.as_bool()).unwrap_or(false);
        out.push(Dep {
            name: name.clone(),
            from,
            allow_unsafe,
        });
    }
    Ok(out)
}

/// The git commits `kavach.lock` records: by name, the repository and
/// commit.
fn read_lock(root: &Path) -> BTreeMap<String, (String, String)> {
    let Ok(text) = std::fs::read_to_string(root.join("kavach.lock")) else {
        return BTreeMap::new();
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return BTreeMap::new();
    };
    let packages = table.get("package").and_then(|p| p.as_array());
    packages
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let s = |k: &str| p.get(k)?.as_str().map(str::to_string);
            Some((s("name")?, (s("git")?, s("commit")?)))
        })
        .collect()
}

/// `kavach.lock` for `packages`: one entry per dependency, in name order.
fn lock_text(root: &Path, packages: &[Package]) -> String {
    let mut out = String::from(
        "# The exact source of each dependency (spec 10.12). Written by kavach: commit it, don't edit it.\n",
    );
    let mut sorted: Vec<&Package> = packages.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    for p in sorted {
        out.push_str(&format!("\n[[package]]\nname = \"{}\"\n", p.name));
        match &p.git {
            Some((url, commit)) => {
                out.push_str(&format!("git = \"{url}\"\ncommit = \"{commit}\"\n"))
            }
            None => out.push_str(&format!("path = \"{}\"\n", relative(root, &p.root))),
        }
    }
    out
}

/// `path` without Windows' `\\?\` prefix, which `canonicalize` adds.
fn plain(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text))
}

/// `to` written relative to the folder `from`, with `..` where needed.
fn relative(from: &Path, to: &Path) -> String {
    let (from, to) = (plain(from), plain(to));
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == 0 {
        return to.to_string_lossy().replace('\\', "/");
    }
    let mut parts: Vec<String> = vec!["..".into(); a.len() - common];
    parts.extend(
        b[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().to_string()),
    );
    if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    }
}

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
fn commit_of(url: &str, want: &Option<(String, String)>) -> Result<String, String> {
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
fn checkout(
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
    if head.trim().starts_with(commit) && dir.join("kavach.toml").is_file() {
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

/// Every dependency of the project at `top`, its dependencies' too, each
/// once. With `fetch`, git dependencies missing from the lock or the
/// folder are fetched and `kavach.lock` is written; without, they must
/// be there already.
pub fn resolve(top: &Path, fetch: bool) -> Result<Vec<Package>, String> {
    let top = &plain(&std::fs::canonicalize(top).unwrap_or_else(|_| top.to_path_buf()));
    let locked = read_lock(top);
    let mut found: Vec<Package> = Vec::new();
    let mut queue: Vec<(PathBuf, Dep)> = declared(top)?
        .into_iter()
        .map(|d| (top.to_path_buf(), d))
        .collect();
    while let Some((from_root, dep)) = queue.pop() {
        let (root, git) = match &dep.from {
            From::Path(p) => (
                plain(&std::fs::canonicalize(p).map_err(|_| {
                    format!("dependency `{}`: no folder {}", dep.name, p.display())
                })?),
                None,
            ),
            From::Git(url, want) => {
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
                (
                    checkout(top, &dep.name, url, &commit, fetch)?,
                    Some((url.clone(), commit)),
                )
            }
        };
        // A fetched package reads nothing outside its own folder.
        if from_root.starts_with(top.join("kavach_modules")) && !root.starts_with(&from_root) {
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
        queue.extend(declared(&root)?.into_iter().map(|d| (root.clone(), d)));
        found.push(Package {
            name: dep.name,
            root,
            allow_unsafe: own_unsafe,
            git,
        });
    }
    if fetch && !found.is_empty() {
        let text = lock_text(top, &found);
        let lock = top.join("kavach.lock");
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
