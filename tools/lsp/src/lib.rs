//! The Kavach language server, behind `kavach lsp`: it speaks the Language
//! Server Protocol over stdin and stdout, so any editor can show Kavach's
//! errors as you type, the type under the cursor, and where a name is
//! declared. It uses the same compiler driver as the CLI, so the
//! messages are the same everywhere.

mod checking;
mod complete;
mod deprecated;
mod diagnostics;
mod fixes;
mod navigate;
mod position;
mod rpc;
mod suggest;
mod tokens;
mod uri;

use kavach_driver::Compiled;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;

/// A file open in the editor.
struct Document {
    text: String,
    /// Its path, if its URI is a `file://` one.
    path: Option<String>,
    /// The same path made absolute, to match files of a project.
    canonical: Option<PathBuf>,
}

#[derive(Default)]
struct Server {
    /// Open documents by URI.
    docs: HashMap<String, Document>,
    initialized: bool,
    shut_down: bool,
    /// Documents changed since their diagnostics were sent, in order.
    dirty: Vec<String>,
    /// Counts every change to the open documents.
    version: u64,
    /// The last compile: of which file, at which `version`. Hover, go to
    /// definition and colors reuse it until a document changes.
    last: RefCell<Option<(String, u64, Checked)>>,
    /// The checking thread, when there is one (`checking.rs`), and how
    /// many of the checks sent to it haven't come back.
    checks: Option<Sender<checking::Job>>,
    pending: usize,
}

/// A compile, and the `kavach.toml` error found with it.
type Checked = Arc<(Compiled, Option<String>)>;

/// Serves one editor until it sends `exit` or the input ends. Returns
/// whether the editor asked to shut down first, as the protocol wants.
pub use fixes::fix_file;
pub use tokens::json_line as tokens_json_line;

pub fn serve(input: impl BufRead, output: impl Write) -> std::io::Result<bool> {
    serve_from(&mut rpc::Plain(input), output)
}

/// `serve` on standard input and output, reading the editor's messages
/// on a thread of their own: a file that changed again before its check
/// ran is checked once, for its latest text.
pub fn serve_stdio() -> std::io::Result<bool> {
    serve_from(&mut rpc::Threaded::stdin(), std::io::stdout())
}

fn serve_from(input: &mut impl rpc::Source, mut output: impl Write) -> std::io::Result<bool> {
    let mut server = Server {
        checks: input.checks().map(checking::start),
        ..Server::default()
    };
    // At the input's end, the checks under way are still published.
    let mut ended = false;
    while !ended || server.pending > 0 {
        let outs = match input.next() {
            rpc::Event::Checked(done) => server.done(*done),
            rpc::Event::Read(read) => match read? {
                None => {
                    ended = true;
                    Vec::new()
                }
                Some(Ok(message)) => {
                    let method = message["method"].as_str().unwrap_or("");
                    if method == "exit" {
                        return Ok(server.shut_down);
                    }
                    server.handle(method, &message)
                }
                Some(Err(e)) => {
                    let error = json!({"code": -32700, "message": e});
                    vec![json!({"jsonrpc": "2.0", "id": null, "error": error})]
                }
            },
        };
        for out in outs {
            rpc::write(&mut output, &out)?;
        }
        if !input.waiting() {
            for out in server.flush() {
                rpc::write(&mut output, &out)?;
            }
        }
    }
    Ok(server.shut_down)
}

impl Server {
    /// The messages to send for one message from the editor.
    fn handle(&mut self, method: &str, message: &Value) -> Vec<Value> {
        let params = &message["params"];
        let Some(id) = message.get("id").cloned() else {
            return self.notification(method, params);
        };
        if method.is_empty() {
            return Vec::new(); // an answer to a request of ours
        }
        let result = if !self.initialized && method != "initialize" {
            Err((-32002, "the server is not initialized yet".to_string()))
        } else if self.shut_down {
            Err((-32600, "the server is shutting down".to_string()))
        } else {
            self.request(method, params)
        };
        vec![match result {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err((code, text)) => {
                json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": text}})
            }
        }]
    }

    fn request(&mut self, method: &str, params: &Value) -> Result<Value, (i64, String)> {
        match method {
            "initialize" => {
                self.initialized = true;
                Ok(json!({
                    "capabilities": {
                        // Open, close and changes as edits (2: incremental).
                        "textDocumentSync": {"openClose": true, "change": 2},
                        "hoverProvider": true,
                        "definitionProvider": true,
                        "completionProvider": {"triggerCharacters": ["."]},
                        "semanticTokensProvider": {"legend": tokens::legend(), "full": true},
                        "codeActionProvider": true,
                    },
                    "serverInfo": {"name": "kavach", "version": env!("CARGO_PKG_VERSION")},
                }))
            }
            "shutdown" => {
                self.shut_down = true;
                Ok(Value::Null)
            }
            "textDocument/hover" | "textDocument/definition" => Ok(self.navigate(method, params)),
            "textDocument/completion" => Ok(self.complete(params)),
            "textDocument/semanticTokens/full" => Ok(self.tokens(params)),
            "textDocument/codeAction" => Ok(self.code_actions(params)),
            _ => Err((-32601, format!("unknown method `{method}`"))),
        }
    }

    fn notification(&mut self, method: &str, params: &Value) -> Vec<Value> {
        let uri = params["textDocument"]["uri"]
            .as_str()
            .unwrap_or("")
            .to_string();
        match method {
            "textDocument/didOpen" => {
                let text = params["textDocument"]["text"].as_str().unwrap_or("");
                let path = uri::to_path(&uri);
                let canonical = path.as_ref().and_then(|p| std::fs::canonicalize(p).ok());
                let text = text.to_string();
                self.docs.insert(
                    uri.clone(),
                    Document {
                        text,
                        path,
                        canonical,
                    },
                );
                self.changed(&uri)
            }
            "textDocument/didChange" => {
                let Some(doc) = self.docs.get_mut(&uri) else {
                    return Vec::new();
                };
                for change in params["contentChanges"].as_array().into_iter().flatten() {
                    let text = change["text"].as_str().unwrap_or("");
                    match change.get("range") {
                        Some(range) => {
                            let start = position::offset(&doc.text, &range["start"]);
                            let end = position::offset(&doc.text, &range["end"]).max(start);
                            doc.text.replace_range(start..end, text);
                        }
                        None => doc.text = text.to_string(),
                    }
                }
                self.changed(&uri)
            }
            "textDocument/didClose" => {
                self.docs.remove(&uri);
                self.version += 1;
                self.dirty.retain(|u| *u != uri);
                vec![publish(&uri, Vec::new())]
            }
            _ => Vec::new(),
        }
    }

    /// Document `uri` changed: its check waits until no other message
    /// has come (`flush`).
    fn changed(&mut self, uri: &str) -> Vec<Value> {
        self.version += 1;
        if !self.dirty.iter().any(|u| u == uri) {
            self.dirty.push(uri.to_string());
        }
        Vec::new()
    }

    /// The diagnostics of every document changed since they were sent:
    /// checked here, or sent to the checking thread.
    fn flush(&mut self) -> Vec<Value> {
        let dirty = std::mem::take(&mut self.dirty);
        let Some(checks) = self.checks.clone() else {
            return dirty.iter().flat_map(|uri| self.publish(uri)).collect();
        };
        for uri in dirty {
            let Some(path) = self.docs.get(&uri).and_then(|d| d.path.clone()) else {
                continue; // not saved yet: no project, no kavach.toml
            };
            let job = checking::Job {
                text: self.docs[&uri].text.clone(),
                open: self
                    .docs
                    .values()
                    .filter_map(|d| Some((d.canonical.clone()?, d.text.clone())))
                    .collect(),
                uri,
                path,
                version: self.version,
            };
            if checks.send(job).is_ok() {
                self.pending += 1;
            }
        }
        Vec::new()
    }

    /// A check from the checking thread: published if nothing changed
    /// since it started, or else checked again.
    fn done(&mut self, done: checking::Done) -> Vec<Value> {
        self.pending -= 1;
        if !self.docs.contains_key(&done.uri) {
            return Vec::new();
        }
        // None: a newer check of the document is on its way.
        let Some(checked) = done.checked else {
            return Vec::new();
        };
        if done.version != self.version {
            if !self.dirty.contains(&done.uri) {
                self.dirty.push(done.uri);
            }
            return Vec::new();
        }
        *self.last.borrow_mut() = Some((done.path.clone(), done.version, checked.clone()));
        self.published(&done.uri, &done.path, &checked)
    }

    /// The compile of the open document at `path`, as its text is now:
    /// the last one, when nothing changed since.
    fn checked(&self, path: &str, text: &str) -> Checked {
        if let Some((p, v, c)) = &*self.last.borrow()
            && p == path
            && *v == self.version
        {
            return c.clone();
        }
        let c = Arc::new(diagnostics::check(path, text, |p| {
            self.open(p).map(|(_, d)| d.text.clone())
        }));
        *self.last.borrow_mut() = Some((path.to_string(), self.version, c.clone()));
        c
    }

    /// Checks a document and publishes the diagnostics of every file
    /// checked with it.
    fn publish(&self, uri: &str) -> Vec<Value> {
        let Some(doc) = self.docs.get(uri) else {
            return Vec::new();
        };
        let Some(path) = &doc.path else {
            return Vec::new(); // not saved yet: no project, no kavach.toml
        };
        let checked = self.checked(path, &doc.text);
        self.published(uri, path, &checked)
    }

    /// The diagnostics of every file a check of document `uri` (at
    /// `path`) went through.
    fn published(&self, uri: &str, path: &str, checked: &Checked) -> Vec<Value> {
        let (compiled, config_error) = &**checked;
        let mut out = Vec::new();
        for (file, mut list) in diagnostics::diagnostics(compiled) {
            let file_uri = match self.open(&file) {
                _ if file == *path => uri.to_string(),
                Some((u, _)) => u.clone(),
                None => uri::from_path(&file),
            };
            if file_uri == uri {
                list.extend(config_error.as_deref().map(diagnostics::config_diagnostic));
            }
            out.push(publish(&file_uri, list));
        }
        out
    }

    /// Hover or go to definition in an open, saved document: from the
    /// same compile as its diagnostics.
    fn navigate(&self, method: &str, params: &Value) -> Value {
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
        let at = &params["position"];
        let Some(doc) = self.docs.get(uri) else {
            return Value::Null;
        };
        let Some(path) = &doc.path else {
            return Value::Null;
        };
        let checked = self.checked(path, &doc.text);
        let compiled = &checked.0;
        if method == "textDocument/hover" {
            return navigate::hover(compiled, path, &doc.text, at);
        }
        match navigate::definition(compiled, path, &doc.text, at) {
            Some((file, range)) => {
                let file_uri = match self.open(&file) {
                    _ if file == *path => uri.to_string(),
                    Some((u, _)) => u.clone(),
                    None => uri::from_path(&file),
                };
                json!({"uri": file_uri, "range": range})
            }
            None => Value::Null,
        }
    }

    /// Code completion in an open, saved document.
    fn complete(&self, params: &Value) -> Value {
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
        let Some(doc) = self.docs.get(uri) else {
            return Value::Null;
        };
        let Some(path) = &doc.path else {
            return Value::Null;
        };
        let check = |text: &str| {
            diagnostics::check(path, text, |p| self.open(p).map(|(_, d)| d.text.clone())).0
        };
        complete::complete(path, &doc.text, &params["position"], check)
    }

    /// Semantic tokens for an open, saved document (`tokens.rs`).
    fn tokens(&self, params: &Value) -> Value {
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
        let Some((doc, path)) = self.docs.get(uri).and_then(|d| Some((d, d.path.as_ref()?))) else {
            return json!({"data": []});
        };
        tokens::tokens(&self.checked(path, &doc.text).0, path, &doc.text)
    }

    /// The open document at `path`, with its URI.
    fn open(&self, path: &str) -> Option<(&String, &Document)> {
        let full = std::fs::canonicalize(Path::new(path)).ok()?;
        self.docs
            .iter()
            .find(|(_, d)| d.canonical.as_ref() == Some(&full))
    }
}

fn publish(uri: &str, diagnostics: Vec<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {"uri": uri, "diagnostics": diagnostics},
    })
}

#[cfg(test)]
mod tests;
