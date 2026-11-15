//! Spans written as a Chrome trace (spec 14.12): with `KAVACH_TRACE` set
//! to a file's path, every span that ends is written there as a complete
//! event, which Perfetto (ui.perfetto.dev) and `chrome://tracing` show on a
//! timeline, thread by thread. A game that runs each frame as
//! `log.span("frame") { … }` sees its frame times. The file is a JSON
//! array the tools read without its closing `]`, so events are added as
//! they come, and a crash loses none.

use crate::trace::Finished;
use std::io::Write;
use std::sync::{Mutex, OnceLock};

/// The open trace file, if `KAVACH_TRACE` names one.
fn file() -> &'static Mutex<Option<std::fs::File>> {
    static FILE: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
    FILE.get_or_init(|| {
        let path = std::env::var("KAVACH_TRACE").ok().filter(|p| !p.is_empty());
        let file = path
            .and_then(|p| std::fs::File::create(p).ok())
            .and_then(|mut f| {
                f.write_all(b"[\n").ok()?;
                Some(f)
            });
        Mutex::new(file)
    })
}

/// This thread's number in the trace.
fn thread() -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::thread::current().id().hash(&mut h);
    h.finish() % 1_000_000
}

/// `text` as a JSON string's inside.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// `span` as one complete event of the Chrome trace format.
pub fn event(span: &Finished, tid: u64) -> String {
    let name = escape(&span.name);
    let ts = span.start_nanos / 1_000;
    let dur = span.end_nanos.saturating_sub(span.start_nanos) / 1_000;
    let error = match &span.error {
        Some(e) => format!(",\"args\":{{\"error\":\"{}\"}}", escape(e)),
        None => String::new(),
    };
    format!(
        "{{\"name\":\"{name}\",\"ph\":\"X\",\"ts\":{ts},\"dur\":{dur},\"pid\":1,\"tid\":{tid}{error}}},\n"
    )
}

/// Writes `spans` to the trace file, when there is one.
pub fn write(spans: &[Finished]) {
    if spans.is_empty() {
        return;
    }
    let mut f = file().lock().unwrap_or_else(|e| e.into_inner());
    let Some(f) = f.as_mut() else {
        return;
    };
    let tid = thread();
    let text: String = spans.iter().map(|s| event(s, tid)).collect();
    let _ = f.write_all(text.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::event;
    use crate::trace::{Finished, SpanId};

    #[test]
    fn a_span_is_a_complete_event_in_microseconds() {
        let span = Finished {
            name: "frame \"1\"".into(),
            id: SpanId { trace: 1, span: 2 },
            parent: None,
            start_nanos: 5_000_000,
            end_nanos: 21_667_000,
            error: None,
        };
        assert_eq!(
            event(&span, 7),
            "{\"name\":\"frame \\\"1\\\"\",\"ph\":\"X\",\"ts\":5000,\"dur\":16667,\"pid\":1,\"tid\":7},\n"
        );
    }
}
