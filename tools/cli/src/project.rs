//! Projects (spec 10.3, 10.5): `kavach new`, and running `run`, `build`,
//! `test` and `check` on a project's `src/main.kav` when no file is given.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The folder with `kavach.toml` at or above the current folder.
pub(crate) fn root() -> Option<PathBuf> {
    let here = std::env::current_dir().ok()?;
    here.ancestors()
        .find(|d| d.join("kavach.toml").is_file())
        .map(Path::to_path_buf)
}

/// The package name from `kavach.toml`, or the folder's name.
fn package_name(root: &Path) -> String {
    let fallback = || {
        root.file_name()
            .map_or("app".into(), |n| n.to_string_lossy().to_string())
    };
    kavach_driver::Config::for_file(&root.join("src").join("main.kav"))
        .ok()
        .map(|c| c.package.name)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(fallback)
}

/// Fills in the project's entry file when a command has no `.kav` file:
/// `kavach run -- a b` becomes `kavach run <root>/src/main.kav a b`, and
/// `kavach build --lib` builds `src/lib.kav` into `.kavach/lib/`.
pub(crate) fn expand(args: Vec<String>) -> Vec<String> {
    let Some(command) = args.first().map(String::as_str) else {
        return args;
    };
    if !matches!(
        command,
        "run" | "build" | "test" | "check" | "audit" | "proto"
    ) {
        return args;
    }
    let mut rest: Vec<String> = args[1..].to_vec();
    // Kavach's own flags come before the file, and stay there: one moved
    // behind it would be the program's argument, so `--dry-run` would run
    // the program for real (spec 8.5).
    let own: &[&str] = match command {
        "run" => &["--native", "--dry-run"],
        "build" => &["--lib", "--compress", "--compress=max"],
        _ => &[],
    };
    let mut flags = Vec::new();
    while rest.first().is_some_and(|a| own.contains(&a.as_str())) {
        flags.push(rest.remove(0));
    }
    let lib = flags.iter().any(|f| f == "--lib");
    if rest.first().is_some_and(|a| a.ends_with(".kav")) {
        return args;
    }
    let Some(root) = root() else {
        return args;
    };
    // A library has `src/lib.kav` instead (spec 10.3); `check` and `test`
    // take it when there is no `main.kav`.
    let only_lib = !root.join("src").join("main.kav").is_file()
        && root.join("src").join("lib.kav").is_file()
        && matches!(command, "check" | "test");
    let entry = if lib || only_lib {
        "lib.kav"
    } else {
        "main.kav"
    };
    let main = root.join("src").join(entry).to_string_lossy().to_string();
    // Everything after `--` belongs to the program.
    if let Some(i) = rest.iter().position(|a| a == "--") {
        rest.remove(i);
    }
    let mut out = vec![command.to_string()];
    out.extend(flags);
    out.push(main);
    if lib && !rest.iter().any(|a| a == "-o") {
        let target = root.join(".kavach").join("lib");
        out.extend(["-o".to_string(), target.to_string_lossy().to_string()]);
    } else if command == "build" && !rest.iter().any(|a| a == "-o") {
        let exe = format!("{}{}", package_name(&root), std::env::consts::EXE_SUFFIX);
        let target = root.join(".kavach").join("bin").join(exe);
        if let Some(dir) = target.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        out.extend(["-o".to_string(), target.to_string_lossy().to_string()]);
    }
    out.extend(rest);
    out
}

/// `kavach check`, `test` or `build` with no file at a workspace's root
/// (spec 10.14): the command in each member's folder, in the order
/// `members` lists them, each member's lines headed by its folder. `None`
/// when this isn't such a command.
pub(crate) fn workspace(args: &[String]) -> Option<ExitCode> {
    let command = args.first()?.as_str();
    if !matches!(command, "check" | "test" | "build") || args.iter().any(|a| a.ends_with(".kav")) {
        return None;
    }
    let here = std::env::current_dir().ok()?;
    if !here.join("kavach.toml").is_file() {
        return None;
    }
    let members = match kavach_driver::workspace::members(&here) {
        Ok(m) if !m.is_empty() => m,
        Ok(_) => return None,
        Err(e) => {
            eprintln!("error: {e}");
            return Some(ExitCode::from(2));
        }
    };
    let me = std::env::current_exe().ok()?;
    let mut failed = false;
    for member in members {
        let shown = member
            .strip_prefix(&here)
            .unwrap_or(&member)
            .to_string_lossy()
            .replace('\\', "/");
        println!("== {shown}");
        if command == "build" && !member.join("src").join("main.kav").is_file() {
            println!("a library: the packages that use it build it");
            continue;
        }
        let ran = std::process::Command::new(&me)
            .args(args)
            .current_dir(&member)
            .status();
        failed |= !ran.is_ok_and(|s| s.success());
    }
    Some(if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// Where `kavach test` saves found cases for `file`: `tests/found/` of its
/// project, or next to the file without one (spec 11.4).
pub(crate) fn found_file(file: &Path) -> PathBuf {
    let stem = file
        .file_stem()
        .map_or("main".into(), |s| s.to_string_lossy().to_string());
    let base = kavach_driver::Config::for_file(file)
        .ok()
        .and_then(|c| c.root)
        .unwrap_or_else(|| file.parent().unwrap_or(Path::new(".")).to_path_buf());
    base.join("tests")
        .join("found")
        .join(format!("{stem}.kavf"))
}

/// `kavach new <name>`.
pub(crate) fn new(name: &str) -> ExitCode {
    let valid = !name.is_empty()
        && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.ends_with('-');
    if !valid {
        eprintln!(
            "error: `{name}` is not a valid project name: use lowercase kebab-case, like `my-app`"
        );
        return ExitCode::from(2);
    }
    let root = Path::new(name);
    if root.exists() {
        eprintln!("error: `{name}` already exists");
        return ExitCode::from(2);
    }
    let files = [
        (
            "kavach.toml",
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n"),
        ),
        (
            "src/main.kav",
            "fn main() {\n    print(\"Hello, Kavach!\")\n}\n".to_string(),
        ),
        (
            ".gitignore",
            "# Build output and caches (spec 10.3).\n.kavach/\n# Fetched dependencies (spec 10.12).\nkavach_modules/\n# Environment variables, often secrets (spec 10.13).\n.env\n# Each agent's state and journal (spec 8.5).\nagents/\n".to_string(),
        ),
    ];
    for (path, text) in files {
        let path = root.join(path);
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::write(&path, text));
        if let Err(e) = written {
            eprintln!("error: can't write {}: {e}", path.display());
            return ExitCode::from(2);
        }
    }
    eprintln!("created {name}/\n  cd {name}\n  kavach run");
    ExitCode::SUCCESS
}
