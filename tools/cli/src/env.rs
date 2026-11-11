//! `.env` files and single variables for `kavach run`, `test` and
//! `build` (spec 10.13, decision 0084): `--env-file <path>`, `--env
//! NAME=value`, `--no-env-file` and `[env]` in `kavach.toml`.

use kavach_runtime_core::env_file;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Default)]
pub(crate) struct Flags {
    file: Option<String>,
    vars: Vec<(String, String)>,
    none: bool,
}

/// What `kavach build` writes beside the executable, worked out first.
static FOR_BUILD: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

/// Takes `--env-file <path>`, `--env NAME=value` and `--no-env-file`
/// from among Kavach's own flags, before the file (spec 10.5): one behind
/// it is the program's argument.
pub(crate) fn take_flags(mut args: Vec<String>) -> Result<(Vec<String>, Flags), String> {
    let mut flags = Flags::default();
    if !matches!(
        args.first().map(String::as_str),
        Some("run" | "test" | "build")
    ) {
        return Ok((args, flags));
    }
    let mut i = 1;
    while i < args.len() && args[i].starts_with("--") && args[i] != "--" {
        match args[i].as_str() {
            "--no-env-file" => {
                flags.none = true;
                args.remove(i);
            }
            "--env-file" | "--env" if i + 1 < args.len() => {
                let value = args.remove(i + 1);
                if args.remove(i) == "--env-file" {
                    flags.file = Some(value);
                } else {
                    let Some((name, v)) =
                        value.split_once('=').filter(|(n, _)| env_file::is_name(n))
                    else {
                        return Err(format!(
                            "`--env {value}`: write `--env NAME=value`, with a name of letters, digits and `_`"
                        ));
                    };
                    flags.vars.push((name.to_string(), v.to_string()));
                }
            }
            "--env-file" | "--env" => return Err(format!("`{}` needs a value", args[i])),
            _ => i += 1,
        }
    }
    Ok((args, flags))
}

/// The file's variables, then `[env.vars]` (which override them), for
/// the program at `path`.
fn defaults(flags: &Flags, path: &str) -> Result<Vec<(String, String)>, String> {
    let full = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
    let root = full
        .ancestors()
        .find(|d| d.join("kavach.toml").is_file())
        .map(Path::to_path_buf);
    let config = match &root {
        Some(_) => kavach_driver::Config::for_file(Path::new(path))?.env,
        None => Default::default(),
    };
    let named = flags
        .file
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| Some(root.as_ref()?.join(config.file.as_ref()?)));
    let file = match (&named, &root) {
        _ if flags.none => None,
        (Some(f), _) => Some(f.clone()),
        (None, Some(r)) => Some(r.join(".env")).filter(|f| f.is_file()),
        (None, None) => None,
    };
    let mut out = Vec::new();
    if let Some(f) = file {
        let shown = kavach_driver::relative_path(&f);
        let text = std::fs::read_to_string(&f).map_err(|e| format!("can't read {shown}: {e}"))?;
        out = env_file::parse(&text, &shown)?;
    }
    out.extend(config.vars_as_text()?);
    Ok(out)
}

/// Before `command` runs the program at `path`: for `run` and `test`, the
/// values the program sees; for `build`, what goes beside it.
pub(crate) fn prepare(command: &str, flags: Flags, path: &str) -> Result<(), String> {
    let defaults = defaults(&flags, path)?;
    if command == "build" {
        // `--no-env-file` builds without one at all.
        let merged: BTreeMap<String, String> = defaults.into_iter().chain(flags.vars).collect();
        if let Ok(mut b) = FOR_BUILD.lock() {
            *b = if flags.none {
                Vec::new()
            } else {
                merged.into_iter().collect()
            };
        }
    } else {
        env_file::set(flags.vars, defaults);
    }
    Ok(())
}

/// After `kavach build` wrote `out`: its `.env` beside it, or none. A
/// `.env` there that `kavach build` wrote before is removed when there is
/// nothing to write; any other is left alone.
pub(crate) fn write_beside(out: &Path) -> Result<(), String> {
    let vars = FOR_BUILD.lock().map(|v| v.clone()).unwrap_or_default();
    let beside = out.with_file_name(".env");
    if vars.is_empty() {
        let ours = std::fs::read_to_string(&beside)
            .is_ok_and(|t| t.starts_with("# Written by `kavach build`"));
        if ours {
            let _ = std::fs::remove_file(&beside);
        }
        return Ok(());
    }
    std::fs::write(&beside, env_file::write(&vars))
        .map_err(|e| format!("can't write {}: {e}", beside.display()))?;
    only_owner_reads(&beside);
    eprintln!(
        "wrote {} ({} variables)",
        kavach_driver::relative_path(&beside),
        vars.len()
    );
    Ok(())
}

/// Secrets: on Unix, only the file's owner may read it.
#[cfg(unix)]
fn only_owner_reads(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn only_owner_reads(_: &Path) {}
