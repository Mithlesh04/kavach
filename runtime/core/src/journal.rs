//! The journal of a durable agent (spec 8.5, decision 0025): each event
//! with its arguments, each capability call's outcome, and each event's
//! end, as JSON lines. When the agent starts, the events that never ended
//! run again; calls already in the journal give their recorded outcome
//! without running, so nothing happens twice. The interpreter and native
//! code both keep it here, so either can finish what the other began.

use crate::json::{self, Tree};
use std::io::Write;
use std::path::PathBuf;

/// What a call gave: its value, or its error.
#[derive(Clone, Debug)]
pub enum Outcome {
    Ok(Tree),
    Err(Tree),
}

/// An event being handled: its number, and when it is run again, the
/// outcomes recorded for it and how many of its calls were made so far.
#[derive(Debug)]
struct Current {
    event: i64,
    recorded: Vec<(String, Outcome)>,
    made: usize,
}

/// An event that never ended: its number, handler, arguments and calls.
#[derive(Debug)]
pub struct Unfinished {
    event: i64,
    pub handler: String,
    pub args: Vec<Tree>,
    calls: Vec<(String, Outcome)>,
    /// Its lines in the journal, to keep when the file is written again.
    lines: Vec<String>,
    /// How many times it started again after its first start.
    again: u32,
}

impl Unfinished {
    /// The event's number, `17` in `RefundBot-17-2`.
    pub fn number(&self) -> i64 {
        self.event
    }
}

/// The starts after which an event that never finished is set aside
/// (decision 0073): its first, and two more.
pub const STARTS: u32 = 3;

#[derive(Debug)]
pub struct Journal {
    path: PathBuf,
    next: i64,
    current: Vec<Current>,
    /// The events set aside when it was opened: their numbers and
    /// handlers, for the audit log (decision 0073).
    pub set_aside: Vec<(i64, String)>,
}

fn field<'t>(t: &'t Tree, name: &str) -> Option<&'t Tree> {
    match t {
        Tree::Object(fs) => fs.iter().find(|(k, _)| k == name).map(|(_, v)| v),
        _ => None,
    }
}

impl Journal {
    /// The journal at `path` (empty if there is none yet), and its events
    /// that never ended, in order. Events that ended are dropped from the
    /// file, which is written again with only the rest, so it never grows
    /// without end; a `next` line keeps the events' numbers going, as
    /// call keys (spec 14.16) must never repeat. Lines a crash cut off
    /// go too.
    pub fn open(path: PathBuf) -> (Journal, Vec<Unfinished>) {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let mut events: Vec<Unfinished> = Vec::new();
        let mut next = 1;
        let mut dropped = false;
        for line in text.lines() {
            let Ok(t) = json::parse(line, &json::Limits::default()) else {
                dropped = true; // a line cut off by a crash
                continue;
            };
            if let (Some(Tree::Int(n)), Some(Tree::Text(h))) =
                (field(&t, "event"), field(&t, "handler"))
            {
                let args = match field(&t, "args") {
                    Some(Tree::Array(a)) => a.clone(),
                    _ => Vec::new(),
                };
                next = next.max(n + 1);
                events.push(Unfinished {
                    event: *n,
                    handler: h.clone(),
                    args,
                    calls: Vec::new(),
                    lines: vec![line.to_string()],
                    again: 0,
                });
            } else if let (Some(Tree::Int(n)), Some(Tree::Text(call))) =
                (field(&t, "of"), field(&t, "call"))
            {
                let outcome = match (field(&t, "ok"), field(&t, "error")) {
                    (_, Some(e)) => Outcome::Err(e.clone()),
                    (Some(v), None) => Outcome::Ok(v.clone()),
                    (None, None) => Outcome::Ok(Tree::Null),
                };
                if let Some(e) = events.iter_mut().find(|e| e.event == *n) {
                    e.calls.push((call.clone(), outcome));
                    e.lines.push(line.to_string());
                }
            } else if let Some(Tree::Int(n)) = field(&t, "again") {
                if let Some(e) = events.iter_mut().find(|e| e.event == *n) {
                    e.again += 1;
                    e.lines.push(line.to_string());
                }
            } else if let Some(Tree::Int(n)) = field(&t, "done") {
                events.retain(|e| e.event != *n);
                dropped = true;
            } else if let Some(Tree::Int(n)) = field(&t, "next") {
                next = next.max(*n);
            }
        }
        // An event that started `STARTS` times without finishing would
        // only stop the agent again: its lines move to `set-aside`.
        let (stuck, rest): (Vec<Unfinished>, Vec<Unfinished>) =
            events.into_iter().partition(|e| e.again + 1 >= STARTS);
        let mut events = rest;
        let mut set_aside: Vec<(i64, String)> = Vec::new();
        if !stuck.is_empty() {
            let aside = path.with_file_name("set-aside");
            let mut text = std::fs::read_to_string(&aside).unwrap_or_default();
            for line in stuck.iter().flat_map(|e| &e.lines) {
                text.push_str(line);
                text.push('\n');
            }
            match aside.to_str() {
                Some(p) if crate::files::write_durably(p, text.as_bytes()).is_ok() => {
                    set_aside = stuck.iter().map(|e| (e.event, e.handler.clone())).collect();
                    dropped = true;
                }
                // Not written: the events stay, so none is lost.
                _ => events.extend(stuck),
            }
            events.sort_by_key(|e| e.event);
        }
        if dropped {
            let mut kept = format!("{{\"next\":{next}}}\n");
            for e in &events {
                for line in &e.lines {
                    kept.push_str(line);
                    kept.push('\n');
                }
            }
            if let Some(p) = path.to_str() {
                let _ = crate::files::write_durably(p, kept.as_bytes());
            }
        }
        let journal = Journal {
            path,
            next,
            current: Vec::new(),
            set_aside,
        };
        (journal, events)
    }

    /// Writes line `t` and forces it to the disk before the program goes
    /// on. A journal that can't be written can't keep its promise, so the
    /// caller stops the program, as a database does when its log fails.
    fn append(&self, t: Tree) -> Result<(), String> {
        let new = !self.path.exists();
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut line = json::write(&t, 0);
        line.push('\n');
        let written = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .and_then(|mut f| {
                f.write_all(line.as_bytes())?;
                f.sync_data()
            });
        // A new file's name must survive a power cut too: force its folder.
        #[cfg(unix)]
        if let (true, Ok(()), Some(dir)) = (new, &written, self.path.parent()) {
            let _ = std::fs::File::open(dir).and_then(|d| d.sync_all());
        }
        #[cfg(not(unix))]
        let _ = new;
        let shown = crate::files::shown_path(&self.path);
        written.map_err(|e| format!("the journal {shown} can't be written: {e}"))
    }

    /// A new event of `handler` with `args` starts: gives its number,
    /// which the calls it makes are recorded under. Several events can be
    /// under way at once, in different tasks.
    pub fn begin(&mut self, handler: &str, args: Vec<Tree>) -> Result<i64, String> {
        let event = self.next;
        self.next += 1;
        self.current.push(Current {
            event,
            recorded: Vec::new(),
            made: 0,
        });
        self.append(Tree::Object(vec![
            ("event".into(), Tree::Int(event)),
            ("handler".into(), Tree::Text(handler.into())),
            ("args".into(), Tree::Array(args)),
        ]))?;
        Ok(event)
    }

    /// An event that never ended starts again. The start is written, so
    /// an event that keeps stopping the program is set aside after
    /// `STARTS` starts (decision 0073).
    pub fn again(&mut self, u: Unfinished) -> Result<i64, String> {
        self.current.push(Current {
            event: u.event,
            recorded: u.calls,
            made: 0,
        });
        self.append(Tree::Object(vec![("again".into(), Tree::Int(u.event))]))?;
        Ok(u.event)
    }

    /// The handler of `event` returned.
    pub fn end(&mut self, event: i64) -> Result<(), String> {
        let Some(i) = self.current.iter().position(|c| c.event == event) else {
            return Ok(());
        };
        self.current.remove(i);
        self.append(Tree::Object(vec![("done".into(), Tree::Int(event))]))
    }

    fn of(&mut self, event: i64) -> Option<&mut Current> {
        self.current.iter_mut().find(|c| c.event == event)
    }

    /// The outcome recorded for the next call, which is `call`: `Ok(None)`
    /// when the journal has no more for this event; `Err` when it recorded
    /// another call there.
    pub fn recorded(&mut self, event: i64, call: &str) -> Result<Option<Outcome>, String> {
        let Some(c) = self.of(event) else {
            return Ok(None);
        };
        let Some((was, outcome)) = c.recorded.get(c.made).cloned() else {
            return Ok(None);
        };
        if was != call {
            return Err(format!(
                "the journal of event #{} recorded `{was}` here, but the handler now calls `{call}`",
                c.event
            ));
        }
        c.made += 1;
        Ok(Some(outcome))
    }

    /// The key of the call running now in agent `agent`'s `event`: the
    /// same in the run after a crash, since the calls before it are made
    /// again in order (spec 14.16). `None` for an event not under way.
    pub fn call_key(&self, event: i64, agent: &str) -> Option<String> {
        let c = self.current.iter().find(|c| c.event == event)?;
        Some(format!("{agent}-{}-{}", c.event, c.made + 1))
    }

    /// The outcome of a call that ran.
    pub fn record(&mut self, event: i64, call: &str, outcome: Outcome) -> Result<(), String> {
        let Some(c) = self.of(event) else {
            return Ok(());
        };
        c.made += 1;
        let event = c.event;
        let (key, value) = match outcome {
            Outcome::Ok(v) => ("ok", v),
            Outcome::Err(e) => ("error", e),
        };
        self.append(Tree::Object(vec![
            ("of".into(), Tree::Int(event)),
            ("call".into(), Tree::Text(call.into())),
            (key.into(), value),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An event that keeps stopping the program is set aside after its
    /// third start (decision 0073): its lines move to `set-aside`, and the
    /// agent starts with the other events.
    #[test]
    fn an_event_that_never_finishes_is_set_aside() {
        let dir = std::env::temp_dir().join(format!("kavach-journal-aside-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("journal");
        let (mut j, _) = Journal::open(path.clone());
        j.begin("stuck", vec![Tree::Int(1)]).unwrap();
        j.begin("fine", vec![Tree::Int(2)]).unwrap();
        // Two more starts of each: the program stopped in both every time.
        for _ in 0..2 {
            let (mut j, unfinished) = Journal::open(path.clone());
            assert_eq!(unfinished.len(), 2);
            assert!(j.set_aside.is_empty());
            for u in unfinished {
                j.again(u).unwrap();
            }
        }
        // The fourth start: both started 3 times, so both are set aside.
        let (j, unfinished) = Journal::open(path.clone());
        assert!(unfinished.is_empty());
        assert_eq!(
            j.set_aside,
            vec![(1, "stuck".to_string()), (2, "fine".to_string())]
        );
        let aside = std::fs::read_to_string(dir.join("set-aside")).unwrap();
        assert_eq!(aside.lines().count(), 6, "{aside}");
        // They are gone from the journal, and the numbers go on.
        let (mut j, unfinished) = Journal::open(path.clone());
        assert!(unfinished.is_empty() && j.set_aside.is_empty());
        j.begin("next", Vec::new()).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"event\":3"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A crash cut the last line off: what is written next still reads
    /// back, so after a second crash the call's outcome is still there
    /// and the call doesn't run a second time.
    #[test]
    fn a_line_cut_off_by_a_crash_does_not_swallow_the_next() {
        let dir = std::env::temp_dir().join(format!("kavach-journal-cut-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("journal");
        let text = "{\"event\":1,\"handler\":\"on_order\",\"args\":[7]}\n{\"of\":1,\"call\":\"pa";
        std::fs::write(&path, text).unwrap();
        let (mut journal, unfinished) = Journal::open(path.clone());
        assert_eq!(unfinished.len(), 1);
        for u in unfinished {
            let event = journal.again(u).unwrap();
            journal
                .record(event, "pay", Outcome::Ok(Tree::Bool(true)))
                .unwrap();
        }
        let (_, unfinished) = Journal::open(path);
        assert_eq!(unfinished.len(), 1);
        assert_eq!(unfinished[0].calls.len(), 1, "the call's outcome was lost");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Events that ended are dropped when the journal opens, so it never
    /// grows without end; the next event still gets a new number.
    #[test]
    fn a_journal_keeps_only_its_unfinished_events() {
        let dir = std::env::temp_dir().join(format!("kavach-journal-keep-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("journal");
        let (mut j, _) = Journal::open(path.clone());
        let done = j.begin("done_one", Vec::new()).unwrap();
        j.end(done).unwrap();
        let cut = j.begin("cut_off", Vec::new()).unwrap();
        j.record(cut, "pay", Outcome::Ok(Tree::Int(1))).unwrap();
        let (mut j, unfinished) = Journal::open(path.clone());
        assert_eq!(unfinished.len(), 1);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("done_one"), "{text}");
        assert!(text.starts_with("{\"next\":3}"), "{text}");
        for u in unfinished {
            let event = j.again(u).unwrap();
            j.end(event).unwrap();
        }
        let (mut j, unfinished) = Journal::open(path.clone());
        assert!(unfinished.is_empty());
        let later = j.begin("later", Vec::new()).unwrap();
        assert_eq!(j.call_key(later, "Bot").as_deref(), Some("Bot-3-1"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Two events under way at once, as two tasks make them: each call
    /// and each end goes to its own event, and a restart gives each its
    /// own outcomes.
    #[test]
    fn events_under_way_at_once_keep_their_own_calls() {
        let dir = std::env::temp_dir().join(format!("kavach-journal-both-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("journal");
        let (mut j, _) = Journal::open(path.clone());
        let first = j.begin("request", vec![Tree::Int(1)]).unwrap();
        let second = j.begin("request", vec![Tree::Int(2)]).unwrap();
        j.record(first, "pay", Outcome::Ok(Tree::Int(10))).unwrap();
        j.record(second, "pay", Outcome::Ok(Tree::Int(20))).unwrap();
        assert_eq!(j.call_key(first, "Bot").as_deref(), Some("Bot-1-2"));
        j.end(first).unwrap();
        let (mut j, unfinished) = Journal::open(path.clone());
        assert_eq!(unfinished.len(), 1);
        let event = j.again(unfinished.into_iter().next().unwrap()).unwrap();
        assert_eq!(event, second);
        let found = j.recorded(event, "pay").unwrap();
        assert!(
            matches!(found, Some(Outcome::Ok(Tree::Int(20)))),
            "{found:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A journal that can't be written says so: the caller stops.
    #[test]
    fn a_journal_that_cant_be_written_says_so() {
        let dir = std::env::temp_dir().join(format!("kavach-journal-ro-{}", std::process::id()));
        // A folder where the journal file should be.
        std::fs::create_dir_all(dir.join("journal")).unwrap();
        let (mut j, _) = Journal::open(dir.join("journal"));
        let why = j.begin("x", Vec::new()).unwrap_err();
        assert!(why.contains("can't be written"), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A corrupted journal never panics: broken lines are skipped, cut
    /// lines ignored (to-do item 4). Deterministic xorshift.
    #[test]
    fn a_corrupted_journal_opens_without_panicking() {
        let valid = concat!(
            "{\"event\":1,\"handler\":\"on_order\",\"args\":[{\"id\":7}]}\n",
            "{\"of\":1,\"call\":\"pay\",\"ok\":true}\n",
            "{\"of\":1,\"call\":\"send\",\"error\":\"down\"}\n",
            "{\"ended\":1}\n",
        );
        let mut seed = 0x0005_EED5_u64;
        let mut next = move || {
            seed ^= seed >> 12;
            seed ^= seed << 25;
            seed ^= seed >> 27;
            seed.wrapping_mul(0x2545_F491_4F6C_DD1D)
        };
        let dir = std::env::temp_dir().join(format!("kavach-journal-fuzz-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for case in 0..200 {
            let text: String = match next() % 3 {
                // Truncated mid-line.
                0 => {
                    let cut = (next() as usize % (valid.len() + 1)..=valid.len())
                        .find(|i| valid.is_char_boundary(*i))
                        .unwrap();
                    valid[..cut].to_string()
                }
                // A byte changed.
                1 => {
                    let mut bytes = valid.as_bytes().to_vec();
                    let at = next() as usize % bytes.len();
                    bytes[at] = (next() % 256) as u8;
                    String::from_utf8_lossy(&bytes).into_owned()
                }
                // Byte soup.
                _ => (0..next() % 120)
                    .map(|_| (next() % 256) as u8 as char)
                    .collect(),
            };
            let path = dir.join(format!("j{case}.jsonl"));
            std::fs::write(&path, &text).unwrap();
            let (_, unfinished) = Journal::open(path);
            // Opening also decided which events to replay; both survived.
            let _ = unfinished.len();
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
