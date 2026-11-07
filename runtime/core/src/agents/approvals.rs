//! Approvals that wait (spec 8.3, decision 0072): a request a person
//! answers later, maybe after the program has restarted. Each request is
//! a file in the agent's folder, `approvals/<id>.json`; an answer is
//! `answers/<id>.json`, written by `kavach approve` or `kavach deny`.
//! The runtimes call `step` until it has decided, sleeping in between
//! the way their tasks sleep, so other tasks run while one waits.

use super::{Check, Guard, Mode, ask_person, unix_ms, utc_now, utc_of};
use crate::json::{self, Tree};
use std::path::{Path, PathBuf};

/// How long a request waits without `within`: 7 days.
pub const DEFAULT_WAIT_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// What step 4 of the checks (spec 8.3) came to: decided (`true` when
/// the call is refused), or waiting for a person's answer.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Done(bool),
    Wait,
}

/// An answer written by `kavach approve` or `kavach deny`.
#[derive(Debug, PartialEq, Eq)]
pub struct Answer {
    pub approved: bool,
    pub by: String,
    pub reason: Option<String>,
}

impl Guard {
    /// Step 4: asks who must approve, if anyone. `key` is the call's
    /// key in a durable handler (`call_key`), the same after a restart;
    /// without one, the request has a number of this run's own.
    pub fn step(&self, c: &mut Check, key: Option<&str>) -> Step {
        if c.refused.is_some() {
            return Step::Done(true);
        }
        let Some(label) = c.needed.clone() else {
            return Step::Done(false);
        };
        if self.settings.mode == Mode::Observe {
            c.notes.push(format!("would need approval by {label}"));
            c.needed = None;
            return Step::Done(false);
        }
        let folder = self.folder();
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) || folder.is_none() {
            // At the terminal, or with nowhere to keep a request: asked
            // now, and not approved when no one can answer.
            c.needed = None;
            return Step::Done(self.answered(c, ask_person(&label, &c.shown), &label, None, None));
        }
        let folder = folder.expect("an agent folder");
        let id = c
            .request
            .get_or_insert_with(|| key.map_or_else(|| self.run_key(), str::to_string))
            .clone();
        let deadline = self.request(&folder, &id, c, &label);
        if let Some(a) = read_answer(&folder, &id) {
            forget(&folder, &id, key.is_none());
            c.needed = None;
            let refused = self.answered(c, a.approved, &label, Some(&a.by), a.reason.as_deref());
            return Step::Done(refused);
        }
        if unix_ms() >= deadline {
            let within = shown_wait(deadline - asked_at(&folder, &id).unwrap_or(deadline));
            forget(&folder, &id, true);
            c.needed = None;
            let why = format!("not approved by {label} within {within}");
            c.notes.push(why.clone());
            c.refused = Some(why);
            return Step::Done(true);
        }
        Step::Wait
    }

    /// Notes an answer on `c` for `label` (`finance (amount > 5_000)`),
    /// from `who` when a person named themselves; gives whether the call
    /// is refused.
    fn answered(
        &self,
        c: &mut Check,
        approved: bool,
        label: &str,
        who: Option<&str>,
        reason: Option<&str>,
    ) -> bool {
        let by = who.map_or_else(|| label.to_string(), |w| format!("{w} for {label}"));
        let reason = reason.map(|r| format!(": {r}")).unwrap_or_default();
        if approved {
            c.notes.push(format!("approved by {by}"));
            return false;
        }
        c.notes.push(format!("not approved by {by}{reason}"));
        c.refused = Some(format!("not approved by {label}{reason}"));
        true
    }

    /// The agent's folder, where its state file is: requests and answers
    /// go there. `None` for an agent without one.
    fn folder(&self) -> Option<PathBuf> {
        self.settings
            .state
            .as_ref()?
            .parent()
            .map(Path::to_path_buf)
    }

    /// A number for a request outside a durable handler: the agent, this
    /// run's start and a count, so an answer from an earlier run never
    /// answers it.
    fn run_key(&self) -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static RUN: std::sync::OnceLock<i64> = std::sync::OnceLock::new();
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let run = RUN.get_or_init(unix_ms);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        format!("{}-r{run}-{n}", self.settings.name)
    }

    /// Writes request `id` if it isn't there yet, and says so on standard
    /// error; gives its deadline, which a restart keeps.
    fn request(&self, folder: &Path, id: &str, c: &Check, label: &str) -> i64 {
        if let Some(deadline) = field_int(&request_path(folder, id), "deadline") {
            return deadline;
        }
        let now = unix_ms();
        let deadline = now + c.within_ms;
        let text = |s: &str| Tree::Text(s.to_string());
        // The rule alone, `amount > 5_000`, from `finance (amount > 5_000)`.
        let why = (label.strip_prefix(&format!("{} (", c.role)))
            .and_then(|r| r.strip_suffix(')'))
            .unwrap_or(label);
        let tree = Tree::Object(vec![
            ("id".into(), text(id)),
            ("agent".into(), text(&self.settings.name)),
            ("call".into(), text(&c.shown)),
            ("by".into(), text(&c.role)),
            ("why".into(), text(why)),
            ("asked".into(), text(&utc_now())),
            ("asked_ms".into(), Tree::Int(now)),
            ("deadline".into(), Tree::Int(deadline)),
            ("until".into(), text(&utc_of(deadline))),
        ]);
        let path = request_path(folder, id);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Some(p) = path.to_str() {
            let _ = crate::files::write_durably(p, json::write(&tree, 0).as_bytes());
        }
        eprintln!(
            "approval needed by {}: {} (request {id}): kavach approve {id} --as <name>, or kavach deny",
            c.role, c.shown
        );
        deadline
    }
}

fn request_path(folder: &Path, id: &str) -> PathBuf {
    folder.join("approvals").join(format!("{id}.json"))
}

fn answer_path(folder: &Path, id: &str) -> PathBuf {
    folder.join("answers").join(format!("{id}.json"))
}

fn read_tree(path: &Path) -> Option<Vec<(String, Tree)>> {
    let text = std::fs::read_to_string(path).ok()?;
    match json::parse(&text, &json::Limits::default()).ok()? {
        Tree::Object(fields) => Some(fields),
        _ => None,
    }
}

fn field_int(path: &Path, name: &str) -> Option<i64> {
    read_tree(path)?
        .into_iter()
        .find_map(|(k, v)| match (k == name, v) {
            (true, Tree::Int(n)) => Some(n),
            _ => None,
        })
}

fn asked_at(folder: &Path, id: &str) -> Option<i64> {
    field_int(&request_path(folder, id), "asked_ms")
}

/// The answer to request `id`, once one is written.
pub fn read_answer(folder: &Path, id: &str) -> Option<Answer> {
    let fields = read_tree(&answer_path(folder, id))?;
    let get = |name: &str| fields.iter().find(|(k, _)| k == name).map(|(_, v)| v);
    let text = |name: &str| match get(name) {
        Some(Tree::Text(s)) => Some(s.clone()),
        _ => None,
    };
    Some(Answer {
        approved: matches!(get("approved"), Some(Tree::Bool(true))),
        by: text("by").unwrap_or_default(),
        reason: text("reason"),
    })
}

/// Why an answer wasn't written: `ApprovalError` (spec 14.16).
#[derive(Debug, PartialEq, Eq)]
pub enum AnswerError {
    /// No request with that number waits: never made, already answered
    /// or out of time.
    NotWaiting,
    Failed(String),
}

/// Answers request `id` of an agent under `agents`, the project's agents
/// folder: `kavach approve` and `kavach deny`, and `agents.answer` in
/// code. A program waiting on it sees it within a second.
pub fn answer_request(agents: &Path, id: &str, a: &Answer) -> Result<(), AnswerError> {
    // A request's number starts with its agent's name: `RefundBot-17-2`.
    let Some((agent, _)) = id.split_once('-') else {
        return Err(AnswerError::NotWaiting);
    };
    let folder = agents.join(agent);
    if !request_path(&folder, id).exists() || answer_path(&folder, id).exists() {
        return Err(AnswerError::NotWaiting);
    }
    write_answer(&folder, id, a).map_err(AnswerError::Failed)
}

fn write_answer(folder: &Path, id: &str, a: &Answer) -> Result<(), String> {
    let mut fields = vec![
        ("approved".into(), Tree::Bool(a.approved)),
        ("by".into(), Tree::Text(a.by.clone())),
        ("at".into(), Tree::Text(utc_now())),
    ];
    if let Some(r) = &a.reason {
        fields.push(("reason".into(), Tree::Text(r.clone())));
    }
    let path = answer_path(folder, id);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let p = path.to_str().ok_or("the folder's name isn't text")?;
    crate::files::write_durably(p, json::write(&Tree::Object(fields), 0).as_bytes())
        .map_err(|e| format!("can't write the answer: {e}"))
}

/// The requests waiting in the agents' folders under `agents`: each
/// request's agent folder and its fields (`kavach approvals`).
pub fn waiting(agents: &Path) -> Vec<(PathBuf, Vec<(String, Tree)>)> {
    let mut found = Vec::new();
    for agent in std::fs::read_dir(agents).into_iter().flatten().flatten() {
        let folder = agent.path();
        for r in std::fs::read_dir(folder.join("approvals"))
            .into_iter()
            .flatten()
            .flatten()
        {
            let id = r
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned());
            let answered = id
                .as_ref()
                .is_some_and(|id| answer_path(&folder, id).exists());
            if let (Some(fields), false) = (read_tree(&r.path()), answered) {
                found.push((folder.clone(), fields));
            }
        }
    }
    found
}

/// What `agents.waiting()` gives (spec 14.16), oldest first: seven texts
/// for each request, its `id`, `agent`, `call`, `by`, `why`, `asked` and
/// `until`.
pub fn request_fields(agents: &Path) -> Vec<String> {
    let mut requests: Vec<_> = waiting(agents).into_iter().map(|(_, f)| f).collect();
    let asked = |f: &[(String, Tree)]| {
        f.iter().find_map(|(k, v)| match (k.as_str(), v) {
            ("asked_ms", Tree::Int(n)) => Some(*n),
            _ => None,
        })
    };
    requests.sort_by_key(|f| asked(f));
    let mut texts = Vec::new();
    for f in &requests {
        for name in ["id", "agent", "call", "by", "why", "asked", "until"] {
            let value = f.iter().find_map(|(k, v)| match (k == name, v) {
                (true, Tree::Text(s)) => Some(s.clone()),
                _ => None,
            });
            texts.push(value.unwrap_or_default());
        }
    }
    texts
}

/// Removes request `id` once it is answered or out of time, and with
/// `answer_too` its answer: a request outside a durable handler is used
/// once. A durable one keeps its answer, so a run after a crash finds it.
fn forget(folder: &Path, id: &str, answer_too: bool) {
    let _ = std::fs::remove_file(request_path(folder, id));
    if answer_too {
        let _ = std::fs::remove_file(answer_path(folder, id));
    }
}

/// `2 days`, `3 hours`, `10 minutes`, `30 seconds`.
fn shown_wait(ms: i64) -> String {
    let s = ms.max(0) / 1000;
    let (n, unit) = match s {
        s if s >= 86_400 && s % 86_400 == 0 => (s / 86_400, "day"),
        s if s >= 3_600 && s % 3_600 == 0 => (s / 3_600, "hour"),
        s if s >= 60 && s % 60 == 0 => (s / 60, "minute"),
        s => (s, "second"),
    };
    format!("{n} {unit}{}", if n == 1 { "" } else { "s" })
}
