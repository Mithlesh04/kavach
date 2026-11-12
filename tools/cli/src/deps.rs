//! `kavach add` and `kavach update` (spec 10.12), and fetching what a
//! project depends on before it is compiled.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The folder of the project in or above the current folder.
fn project_root() -> Result<PathBuf, ExitCode> {
    let here = std::env::current_dir().map_err(|_| ExitCode::from(2))?;
    here.ancestors()
        .find(|d| d.join("kavach.toml").is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            eprintln!("error: no kavach.toml here or above: run this in a project");
            ExitCode::from(2)
        })
}

/// Fetches the dependencies of the project `path` is in, when it has
/// any that aren't here yet, and writes `kavach.lock`.
pub fn fetch_for(path: &str) -> Result<(), ExitCode> {
    let full = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
    let Some(root) = full.ancestors().find(|d| d.join("kavach.toml").is_file()) else {
        return Ok(());
    };
    kavach_driver::deps::resolve(root, true)
        .map(|_| ())
        .map_err(|e| {
            eprintln!("error: {e}");
            ExitCode::from(2)
        })
}

/// `kavach update`: every git dependency fetched again at the tag, branch
/// or commit `kavach.toml` asks for, and `kavach.lock` rewritten.
pub fn update() -> ExitCode {
    let root = match project_root() {
        Ok(r) => r,
        Err(code) => return code,
    };
    let _ = std::fs::remove_file(root.join("kavach.lock"));
    match kavach_driver::deps::resolve(&root, true) {
        Ok(found) => {
            eprintln!("{} dependencies, kavach.lock written", found.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

/// `kavach add <name> --path <folder>` or `kavach add <name> --git <url>
/// [--tag T | --branch B | --rev C] [--unsafe]`: the entry added to
/// `[dependencies]`, then fetched.
pub fn add(args: &[&str]) -> ExitCode {
    let usage = || {
        eprintln!(
            "usage: kavach add <name> --path <folder> | --git <url> [--tag T | --branch B | --rev C] [--unsafe]"
        );
        ExitCode::from(2)
    };
    let [name, rest @ ..] = args else {
        return usage();
    };
    let mut parts = Vec::new();
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        match *flag {
            "--unsafe" => parts.push("unsafe = true".to_string()),
            "--path" | "--git" | "--tag" | "--branch" | "--rev" => match it.next() {
                Some(v) => parts.push(format!("{} = \"{}\"", &flag[2..], v.replace('\\', "/"))),
                None => return usage(),
            },
            _ => return usage(),
        }
    }
    if !parts
        .iter()
        .any(|p| p.starts_with("path") || p.starts_with("git"))
    {
        return usage();
    }
    let root = match project_root() {
        Ok(r) => r,
        Err(code) => return code,
    };
    let file = root.join("kavach.toml");
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let entry = format!("{name} = {{ {} }}", parts.join(", "));
    let new = match text.find("[dependencies]") {
        Some(at) => {
            let end = text[at..].find('\n').map_or(text.len(), |n| at + n + 1);
            format!("{}{entry}\n{}", &text[..end], &text[end..])
        }
        None => format!("{}\n[dependencies]\n{entry}\n", text.trim_end()),
    };
    if std::fs::write(&file, &new).is_err() {
        eprintln!("error: can't write {}", file.display());
        return ExitCode::from(2);
    }
    match kavach_driver::deps::resolve(&root, true) {
        Ok(found) => {
            // What it uses goes into its `can`, for the person adding it
            // to see (spec 10.12).
            let lib = found
                .iter()
                .find(|p| p.name == *name)
                .map(|p| p.root.join("src").join("lib.kav"));
            let caps = lib.map(|l| uses_of(&l)).unwrap_or_default();
            if caps.is_empty() {
                eprintln!("added `{name}`: it uses no capabilities");
                return ExitCode::SUCCESS;
            }
            let quoted: Vec<String> = caps.iter().map(|c| format!("\"{c}\"")).collect();
            let granted = format!(
                "{name} = {{ {}, can = [{}] }}",
                parts.join(", "),
                quoted.join(", ")
            );
            if std::fs::write(&file, new.replacen(&entry, &granted, 1)).is_err() {
                eprintln!("error: can't write {}", file.display());
                return ExitCode::from(2);
            }
            eprintln!(
                "added `{name}`: it uses {}, granted in its `can` in kavach.toml",
                caps.join(", ")
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            // Leave kavach.toml as it was.
            let _ = std::fs::write(&file, &text);
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

/// The capabilities the library `lib` (its `src/lib.kav`) uses anywhere,
/// by name, sorted.
fn uses_of(lib: &Path) -> Vec<String> {
    let Ok(config) = kavach_driver::Config::for_file(lib) else {
        return Vec::new();
    };
    let Some(sources) = kavach_driver::project_sources(lib) else {
        return Vec::new();
    };
    let compiled = kavach_driver::compile_sources(sources, config.options_for(lib));
    let mut caps: Vec<String> = compiled
        .uses
        .values()
        .flatten()
        .map(|c| {
            c.split_once('.')
                .map_or(c.as_str(), |(base, _)| base)
                .to_string()
        })
        .collect();
    caps.sort();
    caps.dedup();
    caps
}
