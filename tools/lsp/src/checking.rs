//! Checks on a thread of their own, so the server answers the editor
//! (hover, completion, colors) while a file is being checked. What a
//! check found comes back as an event among the editor's messages
//! (`rpc::Event`), and is published only if nothing changed since.

use crate::Checked;
use crate::rpc::Event;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// A document to check, as it was at `version`, with the text of every
/// open document by its absolute path.
pub(crate) struct Job {
    pub(crate) uri: String,
    pub(crate) path: String,
    pub(crate) text: String,
    pub(crate) open: Vec<(PathBuf, String)>,
    pub(crate) version: u64,
}

/// A check done; `checked` is `None` when a newer job for the same
/// document made it pointless.
pub(crate) struct Done {
    pub(crate) uri: String,
    pub(crate) path: String,
    pub(crate) version: u64,
    pub(crate) checked: Option<Checked>,
}

/// Starts the checking thread: it takes jobs from the channel returned,
/// and sends what it found to `events`.
pub(crate) fn start(events: Sender<Event>) -> Sender<Job> {
    let (to, jobs) = channel();
    std::thread::spawn(move || work(jobs, events));
    to
}

fn work(jobs: Receiver<Job>, events: Sender<Event>) {
    while let Ok(job) = jobs.recv() {
        // Jobs that came while the last check ran: each document is
        // checked once, at its newest version.
        let mut batch = vec![job];
        batch.extend(jobs.try_iter());
        let newer = |j: &Job| {
            batch
                .iter()
                .any(|k| k.uri == j.uri && k.version > j.version)
        };
        let skip: Vec<bool> = batch.iter().map(newer).collect();
        for (job, skip) in batch.into_iter().zip(skip) {
            // One left out is still sent: the server counts it as done.
            let checked = (!skip).then(|| check(&job));
            let done = Done {
                uri: job.uri,
                path: job.path,
                version: job.version,
                checked,
            };
            if events.send(Event::Checked(Box::new(done))).is_err() {
                return;
            }
        }
    }
}

fn check(job: &Job) -> Checked {
    Arc::new(crate::diagnostics::check(&job.path, &job.text, |p| {
        let full = std::fs::canonicalize(p).ok()?;
        job.open
            .iter()
            .find(|(path, _)| *path == full)
            .map(|(_, text)| text.clone())
    }))
}

#[cfg(test)]
mod tests {
    use crate::rpc;
    use serde_json::{Value, json};

    const URI: &str = "file:///tmp/kavach-lsp-thread.kav";

    /// The editor's messages, framed as on the wire.
    fn wire(messages: &[Value]) -> std::io::Cursor<Vec<u8>> {
        let mut bytes = Vec::new();
        for m in messages {
            rpc::write(&mut bytes, m).unwrap();
        }
        std::io::Cursor::new(bytes)
    }

    fn published(output: &[u8]) -> Vec<String> {
        let mut rest = output;
        let mut out = Vec::new();
        while let Some(m) = rpc::read(&mut rest).unwrap() {
            let m = m.unwrap();
            if m["method"] == "textDocument/publishDiagnostics" {
                out.push(m["params"]["diagnostics"].to_string());
            }
        }
        out
    }

    /// Checked on the checking thread, and published even though the
    /// input ended before the check did.
    #[test]
    fn a_check_on_its_own_thread_is_published() {
        let input = wire(&[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {"uri": URI, "text": "print(missing)\n"}}}),
        ]);
        let mut output = Vec::new();
        crate::serve_from(&mut rpc::Threaded::new(input), &mut output).unwrap();
        let out = published(&output);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].contains("`missing`"), "{out:?}");
    }

    /// A check of a text that changed since is not published: the
    /// document is checked again.
    #[test]
    fn a_stale_check_is_done_again() {
        let mut server = crate::Server::default();
        let open = json!({"textDocument": {"uri": URI, "text": "print(1)\n"}});
        server.notification("textDocument/didOpen", &open);
        server.dirty.clear();
        server.pending = 1;
        let checked = server.checked("/tmp/kavach-lsp-thread.kav", "print(1)\n");
        let done = super::Done {
            uri: URI.to_string(),
            path: "/tmp/kavach-lsp-thread.kav".to_string(),
            version: server.version - 1,
            checked: Some(checked),
        };
        assert!(server.done(done).is_empty());
        assert_eq!(server.dirty, [URI]);
        assert_eq!(server.pending, 0);
    }
}
