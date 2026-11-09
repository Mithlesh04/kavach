//! What built a program (spec 10.5): the compiler's version and commit,
//! and the text each executable `kavach build` writes carries, which
//! `kavach version` finds in it again.

use crate::Config;
use std::path::Path;

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

/// What a program built from `file` says about itself, on two lines
/// (`my-app 0.1.0`, `built by kavach 0.1.0 (commit 8f8ae051)`), and
/// whether it answers `--version` itself (`[build] version_flag`).
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
    let text = format!("{first}\nbuilt by kavach {}", compiler_version());
    (text, config.build.version_flag)
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
    fn a_program_from_one_file_has_its_name_and_no_version() {
        let dir = std::env::temp_dir().join(format!("kavach-version-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (text, flag) = program_version(&dir.join("hello.kav"));
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(text.starts_with("hello\nbuilt by kavach "), "{text}");
        assert!(!flag);
    }
}
