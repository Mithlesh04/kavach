//! `kavach query` and `kavach context` (spec 13.2, 13.4). They answer
//! from the program's index (13.9); inside a project they need no file.

use kavach_ai::{Index, NoIndex};
use std::process::ExitCode;

/// `kavach query <kind> <name> [file]`.
pub(crate) fn query(kind: &str, name: &str, path: Option<&str>) -> ExitCode {
    match index(path) {
        Ok(index) => answer(kavach_ai::query(&index, kind, name)),
        Err(code) => code,
    }
}

/// `kavach context <name> [file] [--budget N]`.
pub(crate) fn context(name: &str, path: Option<&str>, budget: &str) -> ExitCode {
    let Ok(budget) = budget.parse() else {
        eprintln!("error: the budget must be a number of tokens, like `--budget 2000`");
        return ExitCode::from(2);
    };
    match index(path) {
        Ok(index) => answer(kavach_ai::context(&index, name, budget)),
        Err(code) => code,
    }
}

fn answer(result: Result<String, String>) -> ExitCode {
    match result {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

/// The index of `path`'s program, or of the current project's.
fn index(path: Option<&str>) -> Result<Index, ExitCode> {
    let entry = match path {
        Some(p) => p.to_string(),
        None => project_entry().ok_or_else(|| {
            eprintln!("error: no file given, and no kavach.toml in this folder or above it");
            ExitCode::from(2)
        })?,
    };
    kavach_ai::index_for(&entry).map_err(|e| match e {
        NoIndex::Errors(messages) => {
            eprint!("{messages}");
            ExitCode::from(1)
        }
        NoIndex::Failed(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    })
}

/// The project's `src/main.kav` (or `src/lib.kav`), relative to the
/// current folder, so answers show short paths.
fn project_entry() -> Option<String> {
    let root = crate::project::root()?;
    let here = std::env::current_dir().ok()?;
    let depth = here.strip_prefix(&root).ok()?.components().count();
    let file = ["main.kav", "lib.kav"]
        .into_iter()
        .find(|f| root.join("src").join(f).is_file())?;
    Some(format!("{}src/{file}", "../".repeat(depth)))
}
