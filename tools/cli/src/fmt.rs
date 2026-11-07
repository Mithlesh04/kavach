//! `kavach fmt [file.kav] [--check]` (spec 10.5, 12.6): one style, no
//! options. Without a file, it formats every `.kav` file of the project.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn kav_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let hidden = p
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.'));
        if p.is_dir() && !hidden {
            kav_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "kav") {
            out.push(p);
        }
    }
}

/// With `check`, only reports which files are not formatted (exit 1).
pub(crate) fn fmt(args: &[&str]) -> ExitCode {
    let check = args.contains(&"--check");
    let mut files: Vec<PathBuf> = args
        .iter()
        .filter(|a| a.ends_with(".kav"))
        .map(PathBuf::from)
        .collect();
    if files.is_empty() {
        let Some(root) = crate::project::root() else {
            eprintln!("error: give a file (`kavach fmt app.kav`) or run it inside a project");
            return ExitCode::from(2);
        };
        kav_files(&root.join("src"), &mut files);
        kav_files(&root.join("tests"), &mut files);
    }
    let (mut changed, mut failed) = (0, 0);
    for f in &files {
        let src = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: can't read {}: {e}", f.display());
                failed += 1;
                continue;
            }
        };
        // Names first (spec 1.5), then the layout of the lines.
        let (named, left) = kavach_fmt::fix_names(&src, &f.to_string_lossy());
        for note in left {
            eprintln!("note: {}: {note}", f.display());
        }
        match kavach_fmt::format(&named) {
            Ok(out) if out == src => {}
            Ok(out) => {
                changed += 1;
                if check {
                    eprintln!("not formatted: {}", kavach_driver::relative_path(f));
                } else if let Err(e) = std::fs::write(f, out) {
                    eprintln!("error: can't write {}: {e}", f.display());
                    failed += 1;
                } else {
                    eprintln!("formatted {}", kavach_driver::relative_path(f));
                }
            }
            Err(e) => {
                eprintln!("error: {}: {e}", f.display());
                failed += 1;
            }
        }
    }
    let (n, plural) = (files.len(), if files.len() == 1 { "" } else { "s" });
    eprintln!(
        "{n} file{plural}, {changed} {}",
        if check { "need formatting" } else { "changed" }
    );
    ExitCode::from(u8::from(failed > 0 || (check && changed > 0)))
}
