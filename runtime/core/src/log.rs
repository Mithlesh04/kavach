//! Where `std.log` writes (spec 14.12), shared by the interpreter and
//! native code: standard output, and a file that starts afresh when it
//! grows past a size, as `[log]` in `kavach.toml` sets.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The `[log]` settings.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// The lowest level written: 0 `debug`, 1 `info`, 2 `warn`, 3 `error`.
    pub level: i64,
    /// Whether lines go to standard output.
    pub console: bool,
    /// The file lines go to as well, if any.
    pub file: Option<PathBuf>,
    /// The size at which the file is set aside and a new one started.
    pub max_bytes: u64,
    /// How many files set aside are kept (`app.log.1` … `app.log.<keep>`).
    pub keep: u32,
    /// Lowest levels of their own, by module or by event type.
    pub levels: HashMap<String, i64>,
    /// An OpenTelemetry collector events also go to: its endpoint, and
    /// the service name to send as.
    pub otlp: Option<(String, String)>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            level: 1,
            console: true,
            file: None,
            max_bytes: 10_000_000,
            keep: 5,
            levels: HashMap::new(),
            otlp: None,
        }
    }
}

/// The number of a level's name (`"warn"` is 2); `info` for others.
pub fn level_of(name: &str) -> i64 {
    match name {
        "debug" => 0,
        "warn" => 2,
        "error" => 3,
        _ => 1,
    }
}

impl Settings {
    /// The lowest level written for events of type `event`, declared in
    /// `module`: the event type's own, else its module's, else `level`.
    pub fn level_for(&self, module: &str, event: &str) -> i64 {
        let own = self.levels.get(event).or_else(|| self.levels.get(module));
        own.copied().unwrap_or(self.level)
    }

    /// Writes `line` (without its line end) to the file, if there is one:
    /// after the time, and after setting the file aside if it would grow
    /// past `max_bytes`.
    pub fn to_file(&self, line: &str) {
        let Some(path) = &self.file else {
            return;
        };
        let entry = format!("{} {line}\n", crate::agents::utc_now());
        let size = std::fs::metadata(path).map_or(0, |m| m.len());
        if size > 0 && size + entry.len() as u64 > self.max_bytes {
            rotate(path, self.keep);
        }
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = f.write_all(entry.as_bytes());
        }
    }
}

/// `path` with `.n` added: `app.log.1`.
/// `line` as standard output shows it (spec 14.12): its level word in
/// color when the output is a terminal and `NO_COLOR` is not set, so log
/// files and pipes never get escape codes.
pub fn for_console(line: &str) -> String {
    if !colors() {
        return line.to_string();
    }
    colored(line)
}

/// Whether standard output shows colors (`io.colors()`, spec 14.6): it is
/// a terminal, and `NO_COLOR` is not set.
pub fn colors() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
}

/// `line` with its level word (`INFO`, …) in color.
fn colored(line: &str) -> String {
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    let color = match word {
        "DEBUG" => "90",
        "INFO" => "32",
        "WARN" => "33",
        "ERROR" => "1;31",
        _ => return line.to_string(),
    };
    format!("\x1b[{color}m{word}\x1b[0m {rest}")
}

fn numbered(path: &Path, n: u32) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".{n}"));
    PathBuf::from(name)
}

/// Sets `path` aside as `path.1`, moving older ones up and dropping
/// what is past `keep` (with `keep` 0, the file is just started again).
fn rotate(path: &Path, keep: u32) {
    if keep == 0 {
        let _ = std::fs::remove_file(path);
        return;
    }
    let _ = std::fs::remove_file(numbered(path, keep));
    for n in (1..keep).rev() {
        let _ = std::fs::rename(numbered(path, n), numbered(path, n + 1));
    }
    let _ = std::fs::rename(path, numbered(path, 1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_colored_and_the_rest_left_alone() {
        assert_eq!(
            colored("WARN Retrying(attempt: 2)"),
            "\x1b[33mWARN\x1b[0m Retrying(attempt: 2)"
        );
        assert_eq!(colored("ERROR Disk"), "\x1b[1;31mERROR\x1b[0m Disk");
        assert_eq!(colored("not a level"), "not a level");
    }

    #[test]
    fn levels_by_name() {
        for (name, n) in [
            ("debug", 0),
            ("info", 1),
            ("warn", 2),
            ("error", 3),
            ("loud", 1),
        ] {
            assert_eq!(level_of(name), n, "{name}");
        }
    }

    #[test]
    fn an_event_type_wins_over_its_module() {
        let mut s = Settings::default();
        s.levels.insert("billing".into(), 0);
        s.levels.insert("Retrying".into(), 3);
        for (module, event, want) in [
            ("main", "Started", 1),
            ("billing", "Charged", 0),
            ("billing", "Retrying", 3),
            ("main", "Retrying", 3),
        ] {
            assert_eq!(s.level_for(module, event), want, "{module}.{event}");
        }
    }

    #[test]
    fn a_full_file_is_set_aside_and_old_ones_dropped() {
        let dir = std::env::temp_dir().join(format!("kavach-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("app.log");
        let s = Settings {
            file: Some(path.clone()),
            // Each line is 20 (time) + 1 + 4 + 1 bytes: one fits.
            max_bytes: 30,
            keep: 2,
            ..Settings::default()
        };
        for line in ["INFO", "WARN", "ERR1", "ERR2"] {
            s.to_file(line);
        }
        let read = |p: PathBuf| std::fs::read_to_string(p).unwrap_or_default();
        assert!(read(path.clone()).ends_with(" ERR2\n"));
        assert!(read(numbered(&path, 1)).ends_with(" ERR1\n"));
        assert!(read(numbered(&path, 2)).ends_with(" WARN\n"));
        assert!(!numbered(&path, 3).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
