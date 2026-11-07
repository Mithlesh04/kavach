//! OpenTelemetry export of `std.log` events (spec 14.12): OTLP over HTTP
//! with JSON (`POST <endpoint>/v1/logs`). A thread of its own sends them
//! in batches, so a slow collector never slows the program; when too many
//! wait, new ones are dropped and counted. The interpreter and native code
//! both export through here.

use kavach_runtime_core::json::{self, Tree};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// The most events that wait to be sent; more are dropped.
const MAX_WAITING: usize = 10_000;
/// The most events in one request.
const BATCH: usize = 512;
/// How long events may wait before a batch goes.
const EVERY: Duration = Duration::from_secs(1);
/// How long the program's end waits for the last events to go.
const FINISH: Duration = Duration::from_secs(2);

/// One log record.
struct Record {
    level: i64,
    /// The event type's name.
    name: String,
    /// The event's printed form.
    text: String,
    nanos: u128,
    /// More attributes after `event.name`: an audit entry's fields.
    attributes: Vec<(String, String)>,
}

enum Message {
    Record(Record),
    /// Send what waits, then say so.
    Flush(Sender<()>),
}

/// A collector's exporter: its queue, and how many events wait or were
/// dropped.
struct Exporter {
    endpoint: String,
    queue: Sender<Message>,
    waiting: Arc<AtomicUsize>,
    dropped: Arc<AtomicUsize>,
}

static EXPORTERS: Mutex<Vec<Exporter>> = Mutex::new(Vec::new());

/// Queues one event for the collector at `endpoint`, as `service`; the
/// first one starts the exporter's thread.
pub fn export(endpoint: &str, service: &str, level: i64, name: &str, text: &str) {
    queue(endpoint, service, level, name, text, Vec::new());
}

/// Queues an agent's audit entry (spec 8.6): an `AgentAudit` record with
/// the entry's fields as attributes, `WARN` for a refused call.
pub fn export_audit(
    endpoint: &str,
    service: &str,
    warn: bool,
    text: &str,
    attributes: &[(String, String)],
) {
    let level = if warn { 2 } else { 1 };
    queue(
        endpoint,
        service,
        level,
        "AgentAudit",
        text,
        attributes.to_vec(),
    );
}

fn queue(
    endpoint: &str,
    service: &str,
    level: i64,
    name: &str,
    text: &str,
    attributes: Vec<(String, String)>,
) {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut all = EXPORTERS.lock().unwrap_or_else(|e| e.into_inner());
    let i = match all.iter().position(|e| e.endpoint == endpoint) {
        Some(i) => i,
        None => {
            all.push(start(endpoint, service));
            all.len() - 1
        }
    };
    let e = &all[i];
    if e.waiting.load(Ordering::Relaxed) >= MAX_WAITING {
        e.dropped.fetch_add(1, Ordering::Relaxed);
        return;
    }
    e.waiting.fetch_add(1, Ordering::Relaxed);
    let record = Record {
        level,
        name: name.to_string(),
        text: text.to_string(),
        nanos,
        attributes,
    };
    let _ = e.queue.send(Message::Record(record));
}

/// At the program's end: sends the events still waiting, for at most
/// `FINISH`.
pub fn finish() {
    let all = EXPORTERS.lock().unwrap_or_else(|e| e.into_inner());
    let deadline = Instant::now() + FINISH;
    for e in all.iter() {
        let (ack, done) = mpsc::channel();
        if e.queue.send(Message::Flush(ack)).is_ok() {
            let _ = done.recv_timeout(deadline.saturating_duration_since(Instant::now()));
        }
    }
}

fn start(endpoint: &str, service: &str) -> Exporter {
    let (queue, inbox) = mpsc::channel();
    let waiting = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));
    let url = format!("{}/v1/logs", endpoint.trim_end_matches('/'));
    let (service, w, d) = (
        service.to_string(),
        Arc::clone(&waiting),
        Arc::clone(&dropped),
    );
    std::thread::spawn(move || send_batches(&url, &service, &inbox, &w, &d));
    Exporter {
        endpoint: endpoint.to_string(),
        queue,
        waiting,
        dropped,
    }
}

/// The exporter's thread: gathers records for up to `EVERY` (or `BATCH`
/// of them) and sends them; a flush sends at once.
fn send_batches(
    url: &str,
    service: &str,
    inbox: &Receiver<Message>,
    waiting: &AtomicUsize,
    dropped: &AtomicUsize,
) {
    loop {
        let mut batch = Vec::new();
        let mut flushed = None;
        let first = match inbox.recv() {
            Ok(m) => m,
            Err(_) => return,
        };
        let until = Instant::now() + EVERY;
        let mut next = Some(first);
        while let Some(m) = next.take() {
            match m {
                Message::Record(r) => batch.push(r),
                Message::Flush(ack) => {
                    flushed = Some(ack);
                    break;
                }
            }
            if batch.len() >= BATCH {
                break;
            }
            next = match inbox.recv_timeout(until.saturating_duration_since(Instant::now())) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => None,
            };
        }
        waiting.fetch_sub(batch.len(), Ordering::Relaxed);
        let lost = dropped.swap(0, Ordering::Relaxed);
        if !batch.is_empty() || lost > 0 {
            let body = request(service, &batch, lost);
            // A collector that is down or slow loses these events; the
            // program goes on.
            let _ = crate::ai::http::post(
                url,
                &[("content-type", "application/json")],
                body.as_bytes(),
                crate::ai::DEADLINE,
            );
        }
        if let Some(ack) = flushed {
            let _ = ack.send(());
        }
    }
}

fn text(s: &str) -> Tree {
    Tree::Text(s.to_string())
}

fn attribute(key: &str, value: &str) -> Tree {
    Tree::Object(vec![
        ("key".into(), text(key)),
        (
            "value".into(),
            Tree::Object(vec![("stringValue".into(), text(value))]),
        ),
    ])
}

/// OTLP's severity numbers (DEBUG 5, INFO 9, WARN 13, ERROR 17) and texts.
fn severity(level: i64) -> (i64, &'static str) {
    match level {
        0 => (5, "DEBUG"),
        1 => (9, "INFO"),
        2 => (13, "WARN"),
        _ => (17, "ERROR"),
    }
}

fn record(r: &Record) -> Tree {
    let (number, name) = severity(r.level);
    Tree::Object(vec![
        ("timeUnixNano".into(), text(&r.nanos.to_string())),
        ("severityNumber".into(), Tree::Int(number)),
        ("severityText".into(), text(name)),
        (
            "body".into(),
            Tree::Object(vec![("stringValue".into(), text(&r.text))]),
        ),
        (
            "attributes".into(),
            Tree::Array(
                std::iter::once(attribute("event.name", &r.name))
                    .chain(r.attributes.iter().map(|(k, v)| attribute(k, v)))
                    .collect(),
            ),
        ),
    ])
}

/// The request body: the records, and one more when some were dropped.
fn request(service: &str, batch: &[Record], lost: usize) -> String {
    let mut records: Vec<Tree> = batch.iter().map(record).collect();
    if lost > 0 {
        let nanos = batch.last().map_or(0, |r| r.nanos);
        records.push(record(&Record {
            level: 2,
            name: "LogEventsDropped".into(),
            text: format!("LogEventsDropped(count: {lost})"),
            nanos,
            attributes: Vec::new(),
        }));
    }
    let scope = Tree::Object(vec![
        (
            "scope".into(),
            Tree::Object(vec![("name".into(), text("kavach"))]),
        ),
        ("logRecords".into(), Tree::Array(records)),
    ]);
    let resource = Tree::Object(vec![(
        "attributes".into(),
        Tree::Array(vec![attribute("service.name", service)]),
    )]);
    let logs = Tree::Object(vec![
        ("resource".into(), resource),
        ("scopeLogs".into(), Tree::Array(vec![scope])),
    ]);
    json::write(
        &Tree::Object(vec![("resourceLogs".into(), Tree::Array(vec![logs]))]),
        0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_batch_is_otlp_json() {
        let r = Record {
            level: 2,
            name: "Retrying".into(),
            text: "Retrying(attempt: 2)".into(),
            nanos: 7,
            attributes: Vec::new(),
        };
        let body = request("shop", &[r], 3);
        let tree = json::parse(&body, &json::Limits::default()).unwrap();
        let w = json::write(&tree, 0);
        assert!(w.contains("\"service.name\""), "{w}");
        assert!(
            w.contains("\"severityNumber\":13,\"severityText\":\"WARN\""),
            "{w}"
        );
        assert!(
            w.contains("\"stringValue\":\"Retrying(attempt: 2)\""),
            "{w}"
        );
        assert!(w.contains("LogEventsDropped(count: 3)"), "{w}");
    }
}
