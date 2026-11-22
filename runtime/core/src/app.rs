// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.app`'s folders and crash reports (spec 14.30, decision 0118):
//! each system's per-user folder for a program, by its rule, and a report
//! written when a panic ends the program. Dialogs are `runtime/dialog`'s.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Which folder: a program's data, its settings, or its cache.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Data,
    Config,
    Cache,
}

/// The folder of `kind` for the program `name` on system `os`
/// (`windows`, `macos`, or another, which follows XDG), with `env` giving
/// environment variables; or why there is none.
pub fn folder_for(
    kind: Kind,
    name: &str,
    os: &str,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<PathBuf, String> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
        return Err(format!(
            "`{name}` is not a program's name: write it without `/` or `\\`"
        ));
    }
    let var = |v: &str| env(v).filter(|s| !s.is_empty()).map(PathBuf::from);
    // The XDG folders count only when absolute; a relative one is ignored,
    // as the XDG Base Directory spec says.
    let xdg = |v: &str| var(v).filter(|p| p.to_string_lossy().starts_with('/'));
    let home = || var("HOME").ok_or("there is no home folder (`HOME` is not set)");
    let base = match (os, kind) {
        ("windows", Kind::Cache) => {
            return var("LOCALAPPDATA")
                .map(|p| p.join(name).join("cache"))
                .ok_or_else(|| "`LOCALAPPDATA` is not set".into());
        }
        ("windows", _) => var("APPDATA").ok_or("`APPDATA` is not set")?,
        ("macos", Kind::Cache) => home()?.join("Library").join("Caches"),
        ("macos", _) => home()?.join("Library").join("Application Support"),
        (_, Kind::Data) => xdg("XDG_DATA_HOME")
            .map_or_else(|| home().map(|h| h.join(".local").join("share")), Ok)?,
        (_, Kind::Config) => {
            xdg("XDG_CONFIG_HOME").map_or_else(|| home().map(|h| h.join(".config")), Ok)?
        }
        (_, Kind::Cache) => {
            xdg("XDG_CACHE_HOME").map_or_else(|| home().map(|h| h.join(".cache")), Ok)?
        }
    };
    Ok(base.join(name))
}

/// The folder of `kind` for program `name` on this machine, made if
/// missing.
pub fn folder(kind: Kind, name: &str) -> Result<String, String> {
    let path = folder_for(kind, name, std::env::consts::OS, &|v| std::env::var(v).ok())?;
    std::fs::create_dir_all(&path).map_err(|e| format!("{} can't be made: {e}", path.display()))?;
    // The program's own folder is its user's alone, as the XDG spec asks
    // (spec 14.30): others on the machine can't read its saves.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let only_owner = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&path, only_owner)
            .map_err(|e| format!("{} can't be kept private: {e}", path.display()))?;
    }
    Ok(path.to_string_lossy().into_owned())
}

/// Where crash reports go, and the program's name they give.
fn crash_folder() -> &'static Mutex<Option<(PathBuf, String)>> {
    static FOLDER: OnceLock<Mutex<Option<(PathBuf, String)>>> = OnceLock::new();
    FOLDER.get_or_init(Mutex::default)
}

/// What finds the functions running at a panic, for its report: native
/// code's stack walker, given by `crash_reports`' native entry so only a
/// program that asks for reports links it (decision 0024).
static RUNNING: OnceLock<fn() -> Vec<String>> = OnceLock::new();

/// Gives the function that finds the functions running (see `RUNNING`).
pub fn find_running_with(f: fn() -> Vec<String>) {
    let _ = RUNNING.set(f);
}

/// The functions running now, innermost first, when crash reports are on
/// and a finder was given; else none.
pub fn running() -> Vec<String> {
    let on = crash_folder()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some();
    match RUNNING.get() {
        Some(find) if on => find(),
        _ => Vec::new(),
    }
}

/// From now on, a panic that ends the program writes a report into
/// `folder`, which is made now. `main_file` is the program's main source
/// file: the report names the program by it (spec 14.30).
pub fn crash_reports(folder: &str, main_file: &str) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|e| format!("{folder} can't be made: {e}"))?;
    let name = std::path::Path::new(main_file)
        .file_stem()
        .map_or("program".into(), |s| s.to_string_lossy().into_owned());
    *crash_folder().lock().unwrap_or_else(|e| e.into_inner()) = Some((PathBuf::from(folder), name));
    Ok(())
}

/// Writes the report of a panic ending the program, if crash reports
/// were asked for: `message`, `place` (`file:line`), the functions that
/// were `running` (innermost first, each `name (file:line)`), the
/// program, Kavach's version, the system and the time. A report that
/// can't be written is let go: the panic is reported on the terminal
/// anyway.
pub fn report(message: &str, place: &str, running: &[String]) {
    let Some((folder, program)) = crash_folder()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
    else {
        return;
    };
    let secs = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH))
        .map_or(0, |d| d.as_secs());
    let calls: String = running.iter().map(|f| format!("  in {f}\n")).collect();
    let text = format!(
        "panic: {message}\n  at {place}\n{calls}program: {program}\nkavach: {}\nsystem: {} {}\ntime: {secs} (seconds since 1970, UTC)\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
    // Two crashes in one second each keep their report: `crash-{secs}.txt`,
    // then `crash-{secs}-2.txt`, and so on.
    for k in 1..100 {
        let name = match k {
            1 => format!("crash-{secs}.txt"),
            k => format!("crash-{secs}-{k}.txt"),
        };
        let made = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(folder.join(name));
        if let Ok(mut file) = made {
            use std::io::Write;
            let _ = file.write_all(text.as_bytes());
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |v| {
            pairs
                .iter()
                .find(|(k, _)| *k == v)
                .map(|(_, x)| x.to_string())
        }
    }

    #[test]
    fn each_system_has_its_folders() {
        let win = env(&[("APPDATA", "C:/U/r"), ("LOCALAPPDATA", "C:/U/l")]);
        let at = |k, os, e: &dyn Fn(&str) -> Option<String>| {
            folder_for(k, "Sky", os, e).map(|p| p.to_string_lossy().replace('\\', "/"))
        };
        assert_eq!(at(Kind::Data, "windows", &win).unwrap(), "C:/U/r/Sky");
        assert_eq!(at(Kind::Config, "windows", &win).unwrap(), "C:/U/r/Sky");
        assert_eq!(
            at(Kind::Cache, "windows", &win).unwrap(),
            "C:/U/l/Sky/cache"
        );
        let linux = env(&[("HOME", "/h"), ("XDG_CONFIG_HOME", "/xc")]);
        assert_eq!(
            at(Kind::Data, "linux", &linux).unwrap(),
            "/h/.local/share/Sky"
        );
        assert_eq!(at(Kind::Config, "linux", &linux).unwrap(), "/xc/Sky");
        assert_eq!(at(Kind::Cache, "linux", &linux).unwrap(), "/h/.cache/Sky");
        // A relative XDG folder is ignored, as the XDG spec says.
        let relative = env(&[("HOME", "/h"), ("XDG_DATA_HOME", "rel")]);
        let data = at(Kind::Data, "linux", &relative).unwrap();
        assert_eq!(data, "/h/.local/share/Sky");
        let mac = env(&[("HOME", "/Users/a")]);
        assert_eq!(
            at(Kind::Data, "macos", &mac).unwrap(),
            "/Users/a/Library/Application Support/Sky"
        );
        assert_eq!(
            at(Kind::Cache, "macos", &mac).unwrap(),
            "/Users/a/Library/Caches/Sky"
        );
        assert!(
            at(Kind::Data, "linux", &env(&[])).is_err(),
            "no home folder"
        );
        assert!(folder_for(Kind::Data, "a/b", "linux", &linux).is_err());
        assert!(folder_for(Kind::Data, "..", "linux", &linux).is_err());
    }
}
