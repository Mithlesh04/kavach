//! Runs the compiler pipeline (lexer → parser → resolver → checker) on one
//! file and renders its messages. The CLI, the interpreter and (later) the
//! language server all use this, so messages look the same everywhere.

mod config;
mod limits;
mod project;
mod schema;
mod std_modules;

pub use config::{
    AgentConfig, AgentMode, AiConfig, Config, Level, Limits, TestConfig, relative_path,
};
use kavach_ast::{File, Item, Span};
pub use kavach_checker::{AiModel, Prim, Ty};
use kavach_resolver::Resolution;
pub use project::{Source, SourceFile, project_sources};
pub use schema::{NO_SCHEMA, ask_schema, json_schema};
use std::collections::HashMap;
use std_modules::{BUILTIN, is_builtin};

/// What the compiler is told about a file, usually from `kavach.toml`
/// (see [`Config::options_for`]).
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// `unsafe = true` in `[package]` (spec 6.4).
    pub allow_unsafe: bool,
    pub limits: Limits,
    /// Whether `kavach.toml` has a `[log.otlp]` collector (14.12), which an
    /// agent's `Audit(otlp: true)` needs (spec 8.6).
    pub log_otlp: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug)]
pub struct Message {
    pub severity: Severity,
    pub text: String,
    pub span: Span,
}

/// A checked program: one file, or every file of a project parsed as one
/// source (`files` says where each starts). `messages` holds errors and
/// warnings from every stage.
pub struct Compiled {
    pub src: String,
    /// The entry file.
    pub path: String,
    pub files: Vec<SourceFile>,
    pub file: File,
    pub resolution: Resolution,
    pub literal_types: HashMap<u32, Prim>,
    /// Constants typed `BigInt`, as digits, by span (spec 2.1).
    pub big_constants: HashMap<(u32, u32), String>,
    /// Float constants rounded once from their exact value (decision 0043).
    pub float_constants: HashMap<(u32, u32), f64>,
    /// The `map` calls that may run on the GPU (spec 15.2), by span.
    pub gpu_maps: std::collections::HashSet<(u32, u32)>,
    /// The type of every checked expression, by span `(start, end)`.
    pub expr_types: HashMap<(u32, u32), Ty>,
    /// The capabilities each function uses, written or inferred (spec 8.1).
    pub uses: HashMap<kavach_resolver::DefId, Vec<String>>,
    /// The type of each field, by record or variant and field name.
    pub field_types: HashMap<(kavach_resolver::DefId, String), Ty>,
    /// The type of every local variable, by its definition.
    pub local_types: HashMap<kavach_resolver::DefId, Ty>,
    /// The record types that implement `Drop` (spec 6.1).
    pub drop_types: std::collections::HashSet<kavach_resolver::DefId>,
    /// Each agent's `model`, by the agent's name (decision 0056).
    pub agent_models: HashMap<String, kavach_checker::AiModel>,
    /// Each `ask`'s own `model:`, by the call's span.
    pub ask_models: HashMap<(u32, u32), kavach_checker::AiModel>,
    /// Each agent's `settings`, as written (decision 0060).
    pub agent_settings: HashMap<String, kavach_checker::AgentSetup>,
    pub messages: Vec<Message>,
}

/// The model an `ask` uses when neither it nor its agent names one
/// (spec 8.4): the smallest current one.
pub const DEFAULT_MODEL: &str = "claude-haiku-5-5";

/// The most tokens an answer may take when its `Model` doesn't say.
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

impl Compiled {
    /// Agent `name`'s settings as a run uses them (spec 8.5, decision
    /// 0060): its `settings`, with what survives a restart on in a
    /// project, in `agents/<Name>/` beside `kavach.toml` (or `[agents]
    /// folder`, or its own `folder`). `fresh` (`kavach test`, a dry run)
    /// keeps no state and no journal.
    pub fn agent_config(&self, name: &str, fresh: bool) -> AgentConfig {
        let program = std::path::Path::new(&self.path);
        let config = Config::for_file(program).unwrap_or_default();
        let setup = self.agent_settings.get(name).cloned().unwrap_or_default();
        // Paths are relative to the folder of `kavach.toml`, or the
        // program's own without one.
        let base = (config.root.clone()).unwrap_or_else(|| {
            program
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default()
        });
        let folder = match (&setup.folder, &config.root) {
            (Some(f), _) => Some(base.join(f)),
            (None, Some(root)) => {
                let all = config.agents.folder.as_deref().unwrap_or("agents");
                Some(root.join(all).join(name))
            }
            (None, None) => None,
        };
        let path = |p: std::path::PathBuf| p.to_string_lossy().to_string();
        let kept = |on: Option<bool>, file: &str| match (fresh, on) {
            (true, _) | (_, Some(false)) => None,
            _ => folder.as_ref().map(|f| path(f.join(file))),
        };
        let audit = setup.audit.as_ref();
        AgentConfig {
            mode: match setup.mode.as_deref() {
                Some("observe") => AgentMode::Observe,
                Some("suggest") => AgentMode::Suggest,
                _ => AgentMode::Enforce,
            },
            audit_log: audit.map(|a| match &a.file {
                Some(f) => path(base.join(f)),
                None => path(folder.clone().unwrap_or(base.clone()).join("audit.log")),
            }),
            state: kept(setup.state, "state.json"),
            journal: kept(setup.journal, "journal"),
            audit_format: audit.and_then(|a| a.json.then(|| "json".to_string())),
            audit_otlp: audit.is_some_and(|a| a.otlp),
        }
    }

    /// The project's agents folder, where `agents.waiting()` and
    /// `agents.answer` look (spec 14.16): `agents/` beside `kavach.toml`,
    /// or `[agents] folder`. `None` outside a project, and when `fresh`.
    pub fn agents_folder(&self, fresh: bool) -> Option<String> {
        let config = Config::for_file(std::path::Path::new(&self.path)).unwrap_or_default();
        let all = config.agents.folder.as_deref().unwrap_or("agents");
        let root = config.root.filter(|_| !fresh)?;
        Some(root.join(all).to_string_lossy().to_string())
    }

    /// The model an `ask` at `call` uses, inside `agent`'s handlers or
    /// outside any: its own `model:`, else its agent's, else the default
    /// (spec 8.4, decision 0056).
    pub fn ask_model(&self, call: (u32, u32), agent: Option<&str>) -> kavach_checker::AiModel {
        self.ask_models
            .get(&call)
            .or_else(|| agent.and_then(|a| self.agent_models.get(a)))
            .cloned()
            .unwrap_or(kavach_checker::AiModel {
                name: DEFAULT_MODEL.to_string(),
                ..Default::default()
            })
    }
}

/// A model's settings as the runtime takes them (`kavach_runtime_net::ai::
/// Request::new`): the interpreter and native code send the same request.
pub fn model_settings(m: &kavach_checker::AiModel) -> kavach_runtime_core::json::Tree {
    use kavach_runtime_core::json::Tree;
    let text = |s: &str| Tree::Text(s.to_string());
    let max_tokens = m.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
    let mut fields = vec![
        ("model".to_string(), text(&m.name)),
        ("max_tokens".to_string(), Tree::Int(i64::from(max_tokens))),
    ];
    let mut add = |k: &str, v: Option<Tree>| {
        if let Some(v) = v {
            fields.push((k.to_string(), v));
        }
    };
    add("effort", m.effort.as_deref().map(text));
    add("thinking", m.thinking.map(Tree::Bool));
    add("instructions", m.instructions.as_deref().map(text));
    let stop = (!m.stop.is_empty()).then(|| Tree::Array(m.stop.iter().map(|s| text(s)).collect()));
    add("stop", stop);
    add("timeout_ms", m.timeout_ms.map(|n| Tree::Int(n as i64)));
    add("retries", m.retries.map(|n| Tree::Int(i64::from(n))));
    add("api_url", m.api_url.as_deref().map(text));
    add("api_key_env", m.api_key_env.as_deref().map(text));
    add("extra", m.extra.as_deref().map(text));
    Tree::Object(fields)
}

impl Compiled {}

impl Compiled {
    pub fn has_errors(&self) -> bool {
        self.messages.iter().any(|m| m.severity == Severity::Error)
    }

    /// All messages, rendered for a terminal.
    pub fn render(&self) -> String {
        self.messages
            .iter()
            .map(|m| {
                let f = self.file_at(m.span.start);
                render_at(&self.src, &display_path(&f.path), f.first_line, m)
            })
            .collect()
    }

    /// The file and line of a byte offset, the file as messages show it
    /// (`display_path`): for a panic's `at …` and `debug`.
    pub fn shown_location(&self, offset: u32) -> (String, usize) {
        let (path, line) = self.location(offset);
        (display_path(path), line)
    }

    fn file_at(&self, offset: u32) -> &SourceFile {
        let i = self
            .files
            .partition_point(|f| f.start <= offset)
            .saturating_sub(1);
        &self.files[i]
    }

    /// The file and the line in it (1-based) of a byte offset.
    pub fn location(&self, offset: u32) -> (&str, usize) {
        let f = self.file_at(offset);
        (&f.path, self.line_of(offset) - f.first_line + 1)
    }

    /// The items of the program's own files, without the standard
    /// library modules it uses (`BUILTIN`): what tools show.
    pub fn own_items(&self) -> &[Item] {
        let n = self
            .file
            .items
            .partition_point(|i| self.is_own(i.span.start));
        &self.file.items[..n]
    }

    /// Whether the source at `offset` is in the program's own files, not
    /// in a standard library module (`BUILTIN`).
    pub fn is_own(&self, offset: u32) -> bool {
        let end = self
            .files
            .iter()
            .find(|f| is_builtin(&f.path))
            .map_or(u32::MAX, |f| f.start);
        offset < end
    }

    /// The 1-based line of a byte offset.
    pub fn line_of(&self, offset: u32) -> usize {
        self.src[..offset as usize].matches('\n').count() + 1
    }
}

/// Checks one file. Later stages run even after errors, so one run reports
/// as many problems as possible.
pub fn compile(src: &str, path: &str, options: Options) -> Compiled {
    let module = std::path::Path::new(path)
        .file_stem()
        .map_or("main".into(), |s| s.to_string_lossy().to_string());
    let source = Source {
        path: path.to_string(),
        module,
        text: src.to_string(),
    };
    compile_sources(vec![source], options)
}

/// Checks the files of a program together, the entry file first
/// (spec 10.1–10.3).
pub fn compile_sources(mut sources: Vec<Source>, options: Options) -> Compiled {
    // A standard library file given as the program (`kavach test
    // stdlib/log/log.kav`) is that module, not a second copy of it.
    for s in &mut sources {
        if let Some((_, module, _)) = BUILTIN.iter().find(|(.., text)| *text == s.text) {
            s.module = module.to_string();
        }
    }
    // Only the modules the program uses (decision 0024).
    let used = std_modules::used(&sources);
    sources.extend(used);
    // One source, each file ending in a line break.
    let mut src = String::new();
    let mut files = Vec::new();
    let mut modules: Vec<kavach_resolver::Module> = Vec::new();
    // Lines so far, counted file by file: each is counted once.
    let mut lines = 0;
    for s in &sources {
        let start = src.len() as u32;
        files.push(SourceFile {
            path: s.path.clone(),
            start,
            first_line: lines + 1,
        });
        src.push_str(&s.text);
        if !s.text.ends_with('\n') {
            src.push('\n');
        }
        lines += src[start as usize..].matches('\n').count();
        // A module's test file follows it (spec 11.8): one range for both.
        match modules.last_mut() {
            Some(m) if m.name == s.module && s.path.ends_with(".test.kav") => {
                m.end = src.len() as u32;
            }
            _ => modules.push(kavach_resolver::Module {
                name: s.module.clone(),
                start,
                end: src.len() as u32,
            }),
        }
    }
    let src = src.as_str();
    let mut parsed = kavach_parser::parse(src);
    let resolution = kavach_resolver::resolve_modules(&parsed.file, &modules);
    project::unqualify(&mut parsed.file, &resolution);
    let checker_options = kavach_checker::Options {
        allow_unsafe: options.allow_unsafe,
    };
    let checked = kavach_checker::check(&parsed.file, &resolution, checker_options);

    let error = |text: &String, span: Span| Message {
        severity: Severity::Error,
        text: text.clone(),
        span,
    };
    let mut messages: Vec<Message> = parsed
        .errors
        .iter()
        .map(|e| error(&e.message, e.span))
        .collect();
    messages.extend(resolution.errors.iter().map(|e| error(&e.message, e.span)));
    // A test file with no module beside it (spec 11.8).
    for (f, s) in files.iter().zip(&sources) {
        let Some(stem) = s.path.strip_suffix(".test.kav") else {
            continue;
        };
        if sources
            .iter()
            .any(|o| o.module == s.module && !o.path.ends_with(".test.kav"))
        {
            continue;
        }
        let name = stem.rsplit(['/', '\\']).next().unwrap_or(stem);
        messages.push(error(
            &format!("`{name}.test.kav` holds the tests of `{name}.kav`, but there is no `{name}.kav` beside it: name it after the module it tests (spec 11.8)"),
            Span::new(f.start as usize, f.start as usize),
        ));
    }
    messages.extend(checked.errors.iter().map(|e| error(&e.message, e.span)));
    for s in checked.agent_settings.values() {
        if s.audit.as_ref().is_some_and(|a| a.otlp) && !options.log_otlp {
            messages.push(error(
                &"`otlp: true` sends the audit log to a collector: add a `[log.otlp]` section with its `endpoint` to `kavach.toml` (spec 8.6, 14.12)".to_string(),
                Span::new(s.span.0 as usize, s.span.1 as usize),
            ));
        }
    }
    // A `Drop` value counts as used: it lives until its block ends (6.1).
    let held: std::collections::HashSet<(u32, u32)> = (checked.drop_locals.iter())
        .filter_map(|d| resolution.defs[*d].span.map(|s| (s.start, s.end)))
        .collect();
    let warnings = (resolution.warnings.iter())
        .filter(|w| !held.contains(&(w.span.start, w.span.end)))
        .chain(&checked.warnings);
    messages.extend(warnings.map(|w| Message {
        severity: Severity::Warning,
        text: w.message.clone(),
        span: w.span,
    }));
    for (f, s) in files.iter().zip(&sources) {
        if is_builtin(&s.path) {
            continue;
        }
        let end = f.start + s.text.len() as u32;
        let items = &parsed.file.items;
        let a = items.partition_point(|i| i.span.start < f.start);
        let b = items.partition_point(|i| i.span.start < end);
        messages.extend(limits::check(
            &items[a..b],
            src,
            f.start,
            &s.text,
            &s.path,
            &options.limits,
        ));
    }
    messages.sort_by_key(|m| (m.span.start, m.severity == Severity::Warning));
    // The program's files come first, so text the parser can't close in
    // them (a string left open) runs on into the standard library's
    // files: then only the program's own messages are worth showing.
    let builtin_start = files
        .iter()
        .zip(&sources)
        .find(|(_, s)| is_builtin(&s.path))
        .map_or(u32::MAX, |(f, _)| f.start);
    if messages.iter().any(|m| m.span.start < builtin_start) {
        messages.retain(|m| m.span.start < builtin_start);
    }

    Compiled {
        src: src.to_string(),
        path: sources.first().map(|s| s.path.clone()).unwrap_or_default(),
        files,
        file: parsed.file,
        resolution,
        literal_types: checked.literal_types,
        big_constants: checked.big_constants,
        float_constants: checked.float_constants,
        gpu_maps: checked.gpu_maps,
        expr_types: checked.expr_types,
        uses: checked.uses,
        field_types: checked.field_types,
        drop_types: checked.drop_types.clone(),
        local_types: checked.local_types,
        agent_models: checked.agent_models,
        agent_settings: checked.agent_settings,
        ask_models: checked.ask_models,
        messages,
    }
}

/// Renders a message like this:
///
/// ```text
/// error: `totl` is not defined; did you mean `total`?
///  --> hello.kav:3:11
///   |
/// 3 |     print(totl)
///   |           ^^^^
/// ```
pub fn render(src: &str, path: &str, m: &Message) -> String {
    render_at(src, path, 1, m)
}

/// Like [`render`], for a file whose first line is line `first_line` of
/// `src`.
fn render_at(src: &str, path: &str, first_line: usize, m: &Message) -> String {
    let start = m.span.start as usize;
    let line_start = src[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = src[start..].find('\n').map_or(src.len(), |i| start + i);
    let line_no = src[..start].matches('\n').count() + 2 - first_line;
    let col = src[line_start..start].chars().count() + 1;
    let end = (m.span.end as usize).clamp(start, line_end);
    let width = src[start..end].chars().count().max(1);
    let gutter = " ".repeat(line_no.to_string().len());
    let label = match m.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    format!(
        "{label}: {}\n{gutter}--> {path}:{line_no}:{col}\n{gutter} |\n{line_no} | {}\n{gutter} | {}{}\n\n",
        m.text,
        &src[line_start..line_end],
        " ".repeat(col - 1),
        "^".repeat(width)
    )
}

/// A source file as messages show it: a project's from where `kavach`
/// runs (`src/main.kav`), not from the disk's root; a standard library
/// module's as it is (`std/term.kav`).
pub fn display_path(path: &str) -> String {
    match is_builtin(path) {
        true => path.to_string(),
        false => relative_path(std::path::Path::new(path)),
    }
}
