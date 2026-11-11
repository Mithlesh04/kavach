//! Environment variables and `.env` files (spec 10.13, decision 0084):
//! reading a file, and looking a name up in the order that decides which
//! value wins. The process's own environment is never changed, so other
//! threads never see it change under them.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

type Vars = BTreeMap<String, String>;

/// `--env` values, which beat the real environment; then `[env.vars]`
/// and the file's values, which the real environment beats.
static SET: RwLock<(Vars, Vars)> = RwLock::new((BTreeMap::new(), BTreeMap::new()));

/// Sets what `get` adds to the real environment: `forced` beats it,
/// `defaults` only fill in what it lacks.
pub fn set(forced: Vec<(String, String)>, defaults: Vec<(String, String)>) {
    if let Ok(mut s) = SET.write() {
        *s = (forced.into_iter().collect(), defaults.into_iter().collect());
    }
}

/// The value of variable `name`: from `--env`, the real environment, then
/// `[env.vars]` and the file.
pub fn get(name: &str) -> Option<String> {
    let s = SET.read().ok()?;
    s.0.get(name)
        .cloned()
        .or_else(|| std::env::var(name).ok())
        .or_else(|| s.1.get(name).cloned())
}

/// What a process the program starts must be given, so it sees the same
/// values: every `--env` value, and each file value the real environment
/// lacks.
pub fn for_child() -> Vec<(String, String)> {
    let Ok(s) = SET.read() else {
        return Vec::new();
    };
    let missing = s.1.iter().filter(|(k, _)| std::env::var_os(k).is_none());
    s.0.iter()
        .chain(missing)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Whether `name` can be a variable's name: letters, digits and `_`, not
/// starting with a digit.
pub fn is_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The variables of a `.env` file's `text`, in order; `file` names it in
/// errors.
pub fn parse(text: &str, file: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    for (i, raw) in text.lines().enumerate() {
        let wrong = |why: &str| format!("{file}:{}: {why}", i + 1);
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").map_or(line, str::trim_start);
        let Some((name, value)) = line.split_once('=') else {
            return Err(wrong("expected `NAME=value`"));
        };
        let name = name.trim();
        if !is_name(name) {
            return Err(wrong(&format!(
                "`{name}` is not a variable name: letters, digits and `_`"
            )));
        }
        out.push((name.to_string(), unquote(value.trim()).map_err(wrong)?));
    }
    Ok(out)
}

/// A value as written after `=`: in double quotes with `\n`, `\"` and
/// `\\`; in single quotes as written; otherwise as it is.
fn unquote(v: &str) -> Result<String, &'static str> {
    if let Some(rest) = v.strip_prefix('\'') {
        return match rest.split_once('\'') {
            Some((inner, "")) => Ok(inner.to_string()),
            Some(_) => Err("text after the closing `'`"),
            None => Err("no closing `'`"),
        };
    }
    let Some(rest) = v.strip_prefix('"') else {
        return Ok(v.to_string());
    };
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' if chars.as_str().trim().is_empty() => return Ok(out),
            '"' => return Err("text after the closing `\"`"),
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some(c @ ('"' | '\\')) => out.push(c),
                _ => return Err("`\\` is followed by `n`, `\"` or `\\` only"),
            },
            c => out.push(c),
        }
    }
    Err("no closing `\"`")
}

/// The text of a `.env` file that holds `vars`, each read back as it is.
pub fn write(vars: &[(String, String)]) -> String {
    let mut out = String::from("# Written by `kavach build` (spec 10.13): don't commit it.\n");
    for (k, v) in vars {
        let quoted = v
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        out.push_str(&format!("{k}=\"{quoted}\"\n"));
    }
    out
}

/// Set when a built program starts: its first `env` reads the file
/// beside it (`load_once`). Only a flag, so a program that never reads a
/// variable doesn't link the reading (decision 0024).
pub static BUILT: AtomicBool = AtomicBool::new(false);

static LOADED: OnceLock<Result<(), String>> = OnceLock::new();

/// In a built program, the first time a variable is read: its `.env`.
pub fn load_once() -> Result<(), String> {
    if !BUILT.load(Ordering::Relaxed) {
        return Ok(());
    }
    LOADED.get_or_init(load_for_program).clone()
}

/// For a built program: the file `KAVACH_ENV_FILE` names, or else the
/// `.env` beside the executable when there is one.
fn load_for_program() -> Result<(), String> {
    let path = match std::env::var_os("KAVACH_ENV_FILE") {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let beside = exe.with_file_name(".env");
            if !beside.is_file() {
                return Ok(());
            }
            beside
        }
    };
    let shown = path.display().to_string();
    let text = std::fs::read_to_string(&path).map_err(|e| format!("can't read {shown}: {e}"))?;
    set(Vec::new(), parse(&text, &shown)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_read_as_the_spec_says() {
        let text = "\u{feff}# keys\n\nA=1\nexport B = two words \nC=\"x\\ny \\\"q\\\" \\\\\"\nD='$HOME \\n'\nE=$HOME\nF=\nG=a=b\r\n";
        let got = parse(text, ".env").unwrap();
        let want = [
            ("A", "1"),
            ("B", "two words"),
            ("C", "x\ny \"q\" \\"),
            ("D", "$HOME \\n"),
            ("E", "$HOME"),
            ("F", ""),
            ("G", "a=b"),
        ];
        let want: Vec<(String, String)> = want
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn wrong_lines_name_the_file_and_line() {
        let cases = [
            ("A=1\nnothing here", ".env:2: expected `NAME=value`"),
            ("1A=x", ".env:1: `1A` is not a variable name"),
            ("A=\"open", ".env:1: no closing `\"`"),
            ("A=\"x\" y", ".env:1: text after the closing `\"`"),
            ("A='x", ".env:1: no closing `'`"),
            ("A=\"\\t\"", ".env:1: `\\` is followed by"),
        ];
        for (text, want) in cases {
            let got = parse(text, ".env").unwrap_err();
            assert!(got.starts_with(want), "{text:?}: {got}");
        }
    }

    #[test]
    fn written_files_read_back_the_same() {
        let vars = vec![
            ("A".to_string(), "plain".to_string()),
            ("B".to_string(), "line\nbreak \"q\" \\ $X".to_string()),
        ];
        assert_eq!(parse(&write(&vars), "x").unwrap(), vars);
    }
}
