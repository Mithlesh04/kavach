//! The bookkeeping of an agent's guard (spec 8.3, 8.5, 8.6), shared by the
//! interpreter and native code so both decide alike: limit totals over
//! rolling windows, the state file that keeps them across restarts,
//! asking a person, and the audit log. The rules themselves are Kavach
//! expressions; whoever runs the program evaluates them and reports here.

use crate::Decimal;
use crate::json::Tree;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

/// How an agent's guard acts (spec 8.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Run normally; only note what limits and approvals would have done.
    Observe,
    /// A person approves every call.
    Suggest,
    #[default]
    Enforce,
}

/// A number a `sum(…)` adds up.
#[derive(Clone, Debug, PartialEq)]
pub enum Num {
    Int(i64),
    Float(f64),
    Decimal(Decimal),
}

impl Num {
    /// `self + other`, or `None` when it overflows or the kinds differ.
    pub fn add(&self, other: &Num) -> Option<Num> {
        Some(match (self, other) {
            (Num::Int(a), Num::Int(b)) => Num::Int(a.checked_add(*b)?),
            (Num::Float(a), Num::Float(b)) => Num::Float(a + b),
            (Num::Decimal(a), Num::Decimal(b)) => Num::Decimal(a.add(*b)?),
            _ => return None,
        })
    }

    fn to_tree(&self) -> Tree {
        match self {
            Num::Int(n) => Tree::Int(*n),
            Num::Float(f) => Tree::Float(*f),
            Num::Decimal(d) => Tree::Object(vec![("decimal".into(), Tree::Text(d.to_string()))]),
        }
    }

    fn from_tree(t: &Tree) -> Option<Num> {
        match t {
            Tree::Int(n) => Some(Num::Int(*n)),
            Tree::Float(f) => Some(Num::Float(*f)),
            Tree::Object(fields) => match fields.first() {
                Some((k, Tree::Text(d))) if k == "decimal" => Decimal::parse(d).map(Num::Decimal),
                _ => None,
            },
            _ => None,
        }
    }
}

/// An agent's settings: its `settings` in code, worked out when the program
/// is built (decision 0060).
#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub name: String,
    pub mode: Mode,
    /// The audit log's file (spec 8.6).
    pub audit: Option<PathBuf>,
    /// The state file: limit totals across restarts (spec 8.5).
    pub state: Option<PathBuf>,
    /// One JSON object per entry, for log shippers (spec 8.6).
    pub audit_json: bool,
    /// The OpenTelemetry collector and service each entry goes to as well.
    pub otlp: Option<(String, String)>,
    /// How `tokens` counts the `ask`s still waiting for their answer.
    pub under_way: UnderWay,
}

/// Sends one audit entry to a collector: its endpoint and service, whether
/// it warns (a refused call), its text, and its attributes. The network
/// crate does it; whoever runs the program sets it (`set_audit_sink`).
pub type AuditSink = fn(&str, &str, bool, &str, &[(String, String)]);

static SINK: std::sync::OnceLock<AuditSink> = std::sync::OnceLock::new();

/// How audit entries reach a collector (spec 8.6).
pub fn set_audit_sink(sink: AuditSink) {
    let _ = SINK.set(sink);
}

/// The model that answered an `Ai.ask`, and the tokens it took (`None`
/// on a dry run, which asks no one).
#[derive(Clone, Debug, Default)]
pub struct Asked {
    pub model: String,
    pub tokens: Option<(u64, u64)>,
}

impl Asked {
    fn line(&self) -> String {
        match self.tokens {
            Some((i, o)) => format!("{}, tokens: {i} in, {o} out", self.model),
            None => format!("{} (dry run: no tokens)", self.model),
        }
    }
}

/// One limit rule: what it limits, how (as written), and the calls that
/// ran inside its window with what `sum(…)` added for each.
#[derive(Debug)]
pub struct Rule {
    /// `Payments.refund`, or `Ai` for a whole capability.
    pub target: String,
    /// `sum(amount) <= 100 per day`.
    pub text: String,
    window: Option<Duration>,
    /// Each call's time on the steady clock (`steady_ms`), which is
    /// negative for a call from before the program started; what it
    /// added; and the id of a call still running (0 for one that ended).
    history: Vec<(i64, Option<Num>, u64)>,
}

impl Rule {
    /// A rule; `per` is its window's unit (`day`), if it has one.
    pub fn new(target: &str, expr: &str, per: Option<&str>) -> Rule {
        Rule {
            target: target.to_string(),
            text: match per {
                Some(p) => format!("{expr} per {p}"),
                None => expr.to_string(),
            },
            window: per.and_then(window),
            history: Vec::new(),
        }
    }

    /// The key its calls are kept under in the state file: what it limits
    /// and how, so a rule that changed starts again from nothing.
    fn key(&self) -> String {
        format!("{}: {}", self.target, self.text)
    }
}

fn window(per: &str) -> Option<Duration> {
    let hour = 3600;
    Some(Duration::from_secs(match per {
        "minute" => 60,
        "hour" => hour,
        "day" => 24 * hour,
        "week" => 7 * 24 * hour,
        "month" => 30 * 24 * hour,
        _ => return None,
    }))
}

/// Whether a rule or approval for `target` covers a call of `call`
/// (`Cap.method`): the method itself, or its whole capability.
pub fn covers(target: &str, call: &str) -> bool {
    target == call || call.split('.').next() == Some(target)
}

/// A started agent's guard.
#[derive(Debug)]
pub struct Guard {
    pub settings: Settings,
    pub rules: Vec<Rule>,
    /// The number of the event being handled, for the audit log.
    request: u64,
    /// The audit log can't be written (decision 0073): why, and how many
    /// entries went to standard error instead. Until a line can be
    /// written again, the agent's calls are refused.
    broken: std::sync::Mutex<Option<(String, usize)>>,
}

/// One call while it is checked (steps 3 and 4 of spec 8.3) and run.
#[derive(Debug)]
pub struct Check {
    /// `Payments.refund`.
    pub call: String,
    /// The call as the audit log writes it.
    pub shown: String,
    /// Lines for the audit log.
    pub notes: Vec<String>,
    /// For an `Ai.ask`: the model that answered and the tokens it took
    /// (spec 8.6).
    pub model: Option<Asked>,
    /// Why the call is refused; `None` while it may run.
    pub refused: Option<String>,
    /// Who must approve it, once a condition said so: the role and the
    /// condition (`finance (amount > 5_000)`).
    needed: Option<String>,
    /// The role alone, and how long its request may wait (decision 0072).
    role: String,
    within_ms: i64,
    /// The request's number, once it is asked (`approvals.rs`).
    request: Option<String>,
    /// For each rule that used `sum` or `count`, what to add once the
    /// call has run.
    record: Vec<(usize, Option<Num>)>,
    /// The rules that read `tokens`: they add what the `ask` took.
    pub tokens: Vec<usize>,
    /// For an `ask`: the most it can take (`largest_ask`), else 0.
    pub bound: i64,
    /// The calls counted when it started (`Guard::start`): each rule's
    /// index and the id of its entry.
    started: Vec<(usize, u64)>,
}

impl Check {
    /// The `ask` took `n` tokens: what each rule reading `tokens` adds.
    pub fn took(&mut self, n: i64) {
        for (i, x) in &mut self.record {
            if self.tokens.contains(i) {
                *x = Some(Num::Int(n));
            }
        }
    }
}

impl Guard {
    /// A guard with these rules; their calls are read from the state
    /// file, if there is one.
    pub fn new(settings: Settings, rules: Vec<Rule>) -> Guard {
        let mut g = Guard {
            settings,
            rules,
            request: 0,
            broken: std::sync::Mutex::new(None),
        };
        g.load_state();
        g
    }

    /// A handler starts: its calls belong to the next request.
    pub fn next_request(&mut self) {
        self.request += 1;
    }

    /// Starts checking a call of `call` (`Cap.method`), shown as `shown`.
    /// While the audit log can't be written, the call is refused (decision
    /// 0073), unless a line can be written to it now.
    pub fn begin(&self, call: &str, shown: &str) -> Check {
        let refused = self.audit_refuses();
        Check {
            call: call.to_string(),
            shown: shown.to_string(),
            notes: refused
                .iter()
                .map(|why| format!("refused: {why}"))
                .collect(),
            model: None,
            refused,
            needed: (self.settings.mode == Mode::Suggest).then(|| "a person".to_string()),
            role: "a person".to_string(),
            within_ms: approvals::DEFAULT_WAIT_MS,
            request: None,
            record: Vec::new(),
            tokens: Vec::new(),
            bound: 0,
            started: Vec::new(),
        }
    }

    /// The indexes of the rules that cover `call`, in order.
    pub fn rules_for(&self, call: &str) -> Vec<usize> {
        (0..self.rules.len())
            .filter(|i| covers(&self.rules[*i].target, call))
            .collect()
    }

    /// Rule `i`'s calls inside its window: their `sum(…)` total (`None`
    /// when nothing was added) and their count. Older calls are dropped.
    pub fn totals(&mut self, i: usize) -> (Option<Num>, i64) {
        let now = steady_ms();
        let rule = &mut self.rules[i];
        if let Some(w) = rule.window {
            rule.history
                .retain(|(t, _, _)| now - t < w.as_millis() as i64);
        }
        let mut sum: Option<Num> = None;
        for x in rule.history.iter().filter_map(|(_, x, _)| x.as_ref()) {
            sum = Some(match sum {
                None => x.clone(),
                Some(s) => s.add(x).unwrap_or(s),
            });
        }
        (sum, rule.history.len() as i64)
    }

    /// Rule `i` was checked for the call: `ok` is its value; `used` says
    /// whether it used `sum` or `count`, and `added` what `sum` added.
    /// Gives whether the call is refused (checking stops there).
    pub fn limit(&self, c: &mut Check, i: usize, ok: bool, used: bool, added: Option<Num>) -> bool {
        if ok {
            if used {
                c.record
                    .push((i, self.counted_while_under_way(c, i, added)));
            }
            return false;
        }
        let why = format!("limit broken: {}", self.rules[i].text);
        if self.settings.mode == Mode::Observe {
            c.notes.push(format!("would be refused ({why})"));
            return false;
        }
        c.notes.push(format!("refused ({why})"));
        c.refused = Some(why);
        true
    }

    /// An `approve … when <when> by <by> within <within_ms>` whose
    /// condition held: the first one decides who approves, and how long
    /// the request may wait (7 days without `within`).
    pub fn need(&self, c: &mut Check, by: &str, when: &str, within_ms: Option<i64>) {
        if c.needed.as_deref().is_none_or(|n| n == "a person") {
            c.needed = Some(format!("{by} ({when})"));
            c.role = by.to_string();
            c.within_ms = within_ms.unwrap_or(approvals::DEFAULT_WAIT_MS);
        }
    }

    /// The call passed its checks and starts: it counts from now, and the
    /// state file is written, so a crash while it runs can't leave it
    /// uncounted (spec 8.5, decision 0062).
    pub fn start(&mut self, c: &mut Check) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let now = steady_ms();
        for (i, x) in &c.record {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            self.rules[*i].history.push((now, x.clone(), id));
            c.started.push((*i, id));
        }
        self.save_state();
    }

    /// The call ran: it counts, with what it added (an `ask`'s tokens are
    /// known only now), and the state file is written.
    pub fn record(&mut self, c: &Check) {
        if c.started.is_empty() {
            let now = steady_ms();
            for (i, x) in &c.record {
                self.rules[*i].history.push((now, x.clone(), 0));
            }
        }
        for ((i, id), (_, x)) in c.started.iter().zip(&c.record) {
            if let Some(entry) = self.rules[*i].history.iter_mut().find(|e| e.2 == *id) {
                *entry = (entry.0, x.clone(), 0);
            }
        }
        self.save_state();
    }

    /// The call failed: it no longer counts.
    pub fn unrecord(&mut self, c: &Check) {
        for (i, id) in &c.started {
            self.rules[*i].history.retain(|e| e.2 != *id);
        }
        if !c.started.is_empty() {
            self.save_state();
        }
    }
}

/// Asks a person at the terminal. With no terminal there is no one to ask,
/// so the call is not approved (approvals that wait come with durable
/// execution).
pub(super) fn ask_person(by: &str, shown: &str) -> bool {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        return false;
    }
    eprint!("approval needed by {by}: {shown}\napprove? [y/N] ");
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
    matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Milliseconds on a clock that never jumps, from when the program
/// started: limit windows are measured on it (spec 8.4). A time from
/// before the start, read back from the state file, is negative; an
/// `Instant` can't go back past the machine's own start, so after a
/// reboot it would lose older calls and loosen a limit.
pub(crate) fn steady_ms() -> i64 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as i64
}

pub(crate) fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

mod approvals;
pub use approvals::{
    Answer, AnswerError, DEFAULT_WAIT_MS, Step, answer_request, read_answer, request_fields,
    waiting,
};
mod audit;
mod lock;
pub use lock::{lock_folder, unlock_all};
mod state;
pub use audit::{utc_now, utc_of};
mod under_way;
pub use under_way::{TRIES, UnderWay, largest_ask};

#[cfg(test)]
mod tests;
