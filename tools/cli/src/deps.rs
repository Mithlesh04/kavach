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
        Ok(_) => {
            eprintln!("added `{name}`");
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
