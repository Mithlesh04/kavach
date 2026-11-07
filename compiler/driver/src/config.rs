//! `kavach.toml` (spec 10.4): the settings the compiler and `kavach test`
//! read. It is found by looking in the file's folder, then each parent.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// How strictly a limit is enforced (spec 12.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    #[default]
    Error,
    Warn,
    Off,
}

/// `[limits]` (spec 12.1).
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub file_lines: usize,
    pub fn_lines: usize,
    pub level: Level,
    /// Exceptions: a file (relative to `kavach.toml`) and its own limit.
    pub files: HashMap<String, usize>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            file_lines: 500,
            fn_lines: 80,
            level: Level::Error,
            files: HashMap::new(),
            include: vec!["**/*.kav".into()],
            exclude: Vec::new(),
        }
    }
}

/// `[test]` (spec 11.9).
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestConfig {
    pub cases: usize,
    pub deep_cases: usize,
    /// Per call, such as `"1s"` or `"250ms"`.
    pub time_budget: String,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            cases: 200,
            deep_cases: 10_000,
            time_budget: "1s".into(),
        }
    }
}

impl TestConfig {
    /// The time budget in milliseconds, or `None` if it isn't written
    /// like `"1s"`, `"500ms"` or `"2m"`.
    pub fn budget_ms(&self) -> Option<u64> {
        let t = self.time_budget.trim();
        let (number, unit) = t.split_at(t.find(|c: char| !c.is_ascii_digit())?);
        let n: u64 = number.parse().ok()?;
        Some(match unit {
            "ms" => n,
            "s" => n * 1_000,
            "m" => n * 60_000,
            _ => return None,
        })
    }
}

/// The editions there are (spec 10.8), the newest last.
pub const EDITIONS: [&str; 1] = ["2026"];

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub edition: String,
    #[serde(rename = "unsafe")]
    pub allow_unsafe: bool,
    /// Read only to say where it went: each `spawn` chooses (spec 9.8,
    /// decision 0069).
    tasks: Option<String>,
}

/// `[ai]` (spec 8.4): the provider. The model is chosen in the code
/// (decision 0056), and keys come from the environment, never from this
/// file.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AiConfig {
    pub provider: String,
    /// Read only to say where it went.
    model: Option<String>,
    max_tokens: Option<u32>,
}

impl Default for AiConfig {
    fn default() -> Self {
        AiConfig {
            provider: "anthropic".into(),
            model: None,
            max_tokens: None,
        }
    }
}

impl AiConfig {
    /// An error for the settings that moved into the code.
    pub(crate) fn moved(&self) -> Option<String> {
        let key = match (&self.model, &self.max_tokens) {
            (Some(_), _) => "model",
            (_, Some(_)) => "max_tokens",
            _ => return None,
        };
        Some(format!(
            "`{key}` in `[ai]` moved into the code: give the agent `model Model(name: \"…\")`, or a call `model:` (spec 8.4, decision 0056)"
        ))
    }
}

/// How an agent treats its limits and approvals (spec 8.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    Observe,
    Suggest,
    #[default]
    Enforce,
}

/// `[log]` (spec 14.12): where `std.log` writes.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LogConfig {
    /// The lowest level written: `debug`, `info`, `warn` or `error`.
    pub level: String,
    pub console: bool,
    /// A file to write to as well, relative to `kavach.toml`.
    pub file: Option<String>,
    pub max_bytes: u64,
    pub keep: u32,
    /// `[log.levels]`: lowest levels by module or event type.
    pub levels: HashMap<String, String>,
    pub otlp: Option<OtlpConfig>,
}

/// `[log.otlp]`: an OpenTelemetry collector (spec 14.12).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OtlpConfig {
    pub endpoint: String,
    /// The service name; the package's name without one.
    pub service: Option<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig {
            level: "info".into(),
            console: true,
            file: None,
            max_bytes: 10_000_000,
            keep: 5,
            levels: HashMap::new(),
            otlp: None,
        }
    }
}

impl Config {
    /// The `[log]` settings, the file's path made absolute.
    pub fn log_settings(&self) -> kavach_runtime_core::log::Settings {
        let l = &self.log;
        kavach_runtime_core::log::Settings {
            level: kavach_runtime_core::log::level_of(&l.level),
            console: l.console,
            file: l
                .file
                .as_ref()
                .map(|f| self.root.clone().unwrap_or_default().join(f)),
            max_bytes: l.max_bytes,
            keep: l.keep,
            levels: l
                .levels
                .iter()
                .map(|(k, v)| (k.clone(), kavach_runtime_core::log::level_of(v)))
                .collect(),
            otlp: l.otlp.as_ref().map(|o| {
                let service = o
                    .service
                    .clone()
                    .unwrap_or_else(|| self.package.name.clone());
                (o.endpoint.clone(), service)
            }),
        }
    }
}

/// `[agents]` (spec 8.5): where every agent's folder goes. The rest of an
/// agent's settings are in its code (decision 0060).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct AgentsSection {
    /// `<folder>/<Name>/` holds each agent's files; `None`: `agents`.
    pub folder: Option<String>,
    /// `[agents.<Name>]` tables, kept only to say where they went.
    #[serde(flatten)]
    pub moved: HashMap<String, toml::Value>,
}

/// An agent's settings as both runs use them: its `settings` in code,
/// with the defaults and the folder worked out (`Compiled::agent_config`).
#[derive(Clone, Debug, Default)]
pub struct AgentConfig {
    pub mode: AgentMode,
    /// The audit log's file (spec 8.6).
    pub audit_log: Option<String>,
    /// Where limit counters and pending approvals are kept.
    pub state: Option<String>,
    /// The journal that makes the agent durable (spec 8.5, decision 0025).
    pub journal: Option<String>,
    /// `"text"` (the default) or `"json"`: one JSON object per audit
    /// entry, for log shippers (spec 8.6).
    pub audit_format: Option<String>,
    /// Each audit entry to the `[log.otlp]` collector too (spec 8.6).
    pub audit_otlp: bool,
}

/// `kavach.toml`: every key and section is listed in spec 10.11, and any
/// other is an error, so a misspelled setting isn't ignored silently.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub package: Package,
    pub limits: Limits,
    pub test: TestConfig,
    pub ai: AiConfig,
    pub agents: AgentsSection,
    pub log: LogConfig,
    /// Sections described where they are used (spec 10.4), accepted but
    /// not read yet.
    pub dependencies: Option<toml::Value>,
    pub build: Option<toml::Value>,
    pub workspace: Option<toml::Value>,
    pub architecture: Option<toml::Value>,
    /// The folder `kavach.toml` was found in; `None` without one.
    #[serde(skip)]
    pub root: Option<PathBuf>,
}

impl Config {
    /// The `[log.otlp]` collector and service, where an agent's audit
    /// entries go besides its file when its `Audit` says `otlp: true`.
    pub fn otlp_collector(&self) -> Option<(String, String)> {
        self.log_settings().otlp
    }

    /// A `kavach.toml`'s text as a `Config`.
    pub fn parse(text: &str) -> Result<Config, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }

    /// `text`, the contents of the file at `path`, as a `Config`; an
    /// error reads like the compiler's: what is wrong, then where.
    fn parse_file(text: &str, path: &Path) -> Result<Config, String> {
        let config: Config = Config::parse_toml(text, path)?;
        let edition = &config.package.edition;
        if !edition.is_empty() && !EDITIONS.contains(&edition.as_str()) {
            let line = (text
                .lines()
                .position(|l| l.trim_start().starts_with("edition")))
            .map_or(1, |n| n + 1);
            let known = EDITIONS.map(|e| format!("`{e}`")).join(", ");
            return Err(format!(
                "Kavach has no edition `{edition}`: its editions are {known} (spec 10.8)
  --> {}:{line}",
                relative_path(path)
            ));
        }
        Ok(config)
    }

    fn parse_toml(text: &str, path: &Path) -> Result<Config, String> {
        toml::from_str(text).map_err(|e| {
            let at = e.span().map_or(0, |s| s.start).min(text.len());
            let line = text[..at].matches('\n').count() + 1;
            let column = at - text[..at].rfind('\n').map_or(0, |n| n + 1) + 1;
            // The section the error is in: the last `[...]` line above it.
            let section = (text[..at].lines().rev())
                .map(str::trim)
                .find(|l| l.starts_with('['))
                .map(|l| format!(" in {l}"))
                .unwrap_or_default();
            // The key on the error's line, for a value it doesn't take.
            let key = (text.lines().nth(line - 1))
                .and_then(|l| l.split_once('='))
                .map(|(k, _)| k.trim().to_string());
            let message = (e.message().trim())
                .replace("unknown field", "unknown key")
                .replace("unknown variant", "unknown value");
            let message = match message.split_once("`, expected") {
                Some((name, rest)) if message.starts_with("unknown key") => {
                    format!("{name}`{section}: expected{rest}")
                }
                Some((value, rest)) if message.starts_with("unknown value") => {
                    let of = key.map(|k| format!(" for `{k}`")).unwrap_or_default();
                    format!("{value}`{of}{section}: expected{rest}")
                }
                _ => message,
            };
            format!("{message}\n  --> {}:{line}:{column}", relative_path(path))
        })
    }

    /// The configuration for `file`: from the nearest `kavach.toml`, or the
    /// defaults when there is none.
    pub fn for_file(file: &Path) -> Result<Config, String> {
        let start = file.parent().unwrap_or(Path::new("."));
        let start = std::fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());
        for dir in start.ancestors() {
            let path = dir.join("kavach.toml");
            if let Ok(text) = std::fs::read_to_string(&path) {
                let mut config = Config::parse_file(&text, &path)?;
                if let Some(moved) = config.ai.moved() {
                    return Err(format!("{moved}\n  --> {}", relative_path(&path)));
                }
                if config.package.tasks.is_some() {
                    return Err(format!(
                        "`tasks` in `[package]` moved into the code: write `spawn thread …` or `spawn cores …` where a task should run another way; a plain `spawn` is a coroutine (spec 9.8, decision 0069)\n  --> {}",
                        relative_path(&path)
                    ));
                }
                let mut moved: Vec<&String> = config.agents.moved.keys().collect();
                moved.sort();
                if let Some(name) = moved.first() {
                    return Err(format!(
                        "`[agents.{name}]` moved into the code: give agent `{name}` `settings AgentSettings(…)` (spec 8.5, decision 0060). `mode = \"observe\"` is `mode: AgentMode.Observe`; `audit_log`, `audit_format` and `audit_otlp` are `audit: Audit(file: \"…\", format: AuditFormat.JsonLines, otlp: true)`; `state` and `journal` are on by default, in `agents/{name}/`. `kavach.toml` keeps only `[agents] folder`\n  --> {}",
                        relative_path(&path)
                    ));
                }
                config.root = Some(dir.to_path_buf());
                return Ok(config);
            }
        }
        Ok(Config::default())
    }

    /// The compiler options for `file`, with its own line limit if
    /// `[limits.files]` gives it one.
    pub fn options_for(&self, file: &Path) -> crate::Options {
        let log_otlp = self.log.otlp.is_some();
        let mut limits = self.limits.clone();
        if let Some(root) = &self.root {
            let full = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
            if let Ok(rel) = full.strip_prefix(root) {
                let rel = rel.to_string_lossy().replace('\\', "/");
                if let Some(n) = self.limits.files.get(&rel) {
                    limits.file_lines = *n;
                }
            }
        }
        crate::Options {
            allow_unsafe: self.package.allow_unsafe,
            limits,
            log_otlp,
        }
    }
}

/// `path` as a person reads it: relative to where they are, without
/// Windows' `\\?\` prefix.
pub fn relative_path(path: &Path) -> String {
    let unprefixed = |p: &Path| {
        let text = p.display().to_string();
        PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text))
    };
    let plain = unprefixed(path);
    let here = std::env::current_dir()
        .ok()
        .and_then(|d| std::fs::canonicalize(d).ok());
    let relative = here.and_then(|d| {
        plain
            .strip_prefix(unprefixed(&d))
            .ok()
            .map(Path::to_path_buf)
    });
    relative
        .unwrap_or(plain)
        .display()
        .to_string()
        .replace('\\', "/")
}
