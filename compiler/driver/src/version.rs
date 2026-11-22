// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What built a program (spec 10.5): the compiler's version and commit,
//! and the text each executable `kavach build` writes carries, which
//! `kavach version` finds in it again.

use crate::Config;
use std::path::Path;
use std::sync::OnceLock;

/// Where the text starts and ends inside an executable.
const START: &str = "<kavach-version>";
const END: &str = "</kavach-version>";

/// The compiler's version and commit: `0.1.0 (commit 8f8ae051)`.
pub fn compiler_version() -> String {
    format!(
        "{} (commit {})",
        env!("CARGO_PKG_VERSION"),
        env!("KAVACH_COMMIT")
    )
}

/// How `kavach build` builds the program: `built <moment> for <system>
/// with <options>`. Unset under `kavach run`, which keeps no executable.
static BUILT: OnceLock<String> = OnceLock::new();

/// Says that this run of the compiler is `kavach build` with `options`
/// (`--tracy`, `--compress`, …), before the program is lowered; the moment
/// is the clock's, or `SOURCE_DATE_EPOCH`'s (spec 10.5).
pub fn set_build_options(options: &[String]) {
    let epoch = std::env::var("SOURCE_DATE_EPOCH").ok();
    let _ = BUILT.set(build_line(epoch.as_deref(), options));
}

/// The third line of a built program's text (spec 10.5).
fn build_line(epoch: Option<&str>, options: &[String]) -> String {
    let ms = match epoch.and_then(|s| s.trim().parse::<i64>().ok()) {
        Some(secs) => secs.saturating_mul(1000),
        None => std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64),
    };
    let moment = kavach_runtime_core::agents::utc_of(ms);
    let (arch, os) = (std::env::consts::ARCH, std::env::consts::OS);
    let mut line = format!("built {moment} for {arch}-{os}");
    if !options.is_empty() {
        line = format!("{line} with {}", options.join(" "));
    }
    line
}

/// What a program built from `file` says about itself, on two lines
/// (`my-app 0.1.0`, `built by kavach 0.1.0 (commit 8f8ae051)`), and a
/// third under `kavach build` (`set_build_options`); and whether it
/// answers `--version` itself (`[build] version_flag`).
pub fn program_version(file: &Path) -> (String, bool) {
    let config = Config::for_file(file).unwrap_or_default();
    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned());
    let name = match config.package.name.as_str() {
        "" => stem.unwrap_or_default(),
        name => name.to_string(),
    };
    let first = match config.package.version.as_str() {
        "" => name,
        version => format!("{name} {version}"),
    };
    let mut text = format!("{first}\nbuilt by kavach {}", compiler_version());
    if let Some(line) = BUILT.get() {
        text = format!("{text}\n{line}");
    }
    (text, config.build.version_flag)
}

/// The bytes of the icon `[build] icon` names for a program built from
/// `file` (spec 10.5), if it names one; the path is relative to
/// `kavach.toml`.
pub fn program_icon(file: &Path) -> Result<Option<Vec<u8>>, String> {
    let config = Config::for_file(file)?;
    let Some(icon) = config.build.icon else {
        return Ok(None);
    };
    let start = file.parent().unwrap_or(Path::new("."));
    let start = std::fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());
    let dir = start.ancestors().find(|d| d.join("kavach.toml").is_file());
    let path = dir.unwrap_or(&start).join(&icon);
    std::fs::read(&path)
        .map(Some)
        .map_err(|e| format!("can't read the icon `{icon}` that `[build]` names: {e}"))
}

/// `text` as an executable carries it, between its marks.
pub fn marked(text: &str) -> String {
    format!("{START}{text}{END}")
}

/// The text an executable carries, if `kavach build` wrote it.
pub fn find_marked(bytes: &[u8]) -> Option<String> {
    let mut from = 0;
    while let Some(at) = find(&bytes[from..], START.as_bytes()) {
        let start = from + at + START.len();
        let len = find(&bytes[start..], END.as_bytes())?;
        let text = std::str::from_utf8(&bytes[start..start + len]).ok();
        // Only a real one: the compiler itself has the marks apart.
        if let Some(t) = text.filter(|t| t.contains("\nbuilt by kavach ")) {
            return Some(t.to_string());
        }
        from = start;
    }
    None
}

/// Where `needle` first is in `hay`.
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_is_found_between_its_marks_in_any_bytes() {
        let text = "app 1.2.0\nbuilt by kavach 0.1.0 (commit abc)";
        let mut bytes = b"\x7fELF\0\0<kavach-version></kavach-version>junk".to_vec();
        bytes.extend_from_slice(marked(text).as_bytes());
        bytes.extend_from_slice(b"\0more");
        assert_eq!(find_marked(&bytes).as_deref(), Some(text));
        assert_eq!(find_marked(b"no marks here"), None);
    }

    #[test]
    fn the_build_line_names_the_moment_the_system_and_the_options() {
        let system = format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS);
        let cases: [(Option<&str>, &[&str], String); 3] = [
            (
                Some("0"),
                &[],
                format!("built 1970-01-01T00:00:00Z for {system}"),
            ),
            (
                Some("1763890867"),
                &["--tracy", "--compress"],
                format!("built 2025-11-23T09:41:07Z for {system} with --tracy --compress"),
            ),
            (
                Some(" 86400 "),
                &[],
                format!("built 1970-01-02T00:00:00Z for {system}"),
            ),
        ];
        for (epoch, options, line) in cases {
            let options: Vec<String> = options.iter().map(|o| o.to_string()).collect();
            assert_eq!(build_line(epoch, &options), line);
        }
        // Without a usable `SOURCE_DATE_EPOCH`, the clock's moment.
        assert!(build_line(Some("soon"), &[]).starts_with("built 20"));
    }

    #[test]
    fn a_program_from_one_file_has_its_name_and_no_version() {
        let dir = std::env::temp_dir().join(format!("kavach-version-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (text, flag) = program_version(&dir.join("hello.kav"));
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(text.starts_with("hello\nbuilt by kavach "), "{text}");
        assert!(!flag);
    }
}
