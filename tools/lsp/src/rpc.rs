//! The wire format of the Language Server Protocol: JSON-RPC 2.0 messages,
//! each after a `Content-Length` header and a blank line.

use serde_json::Value;
use std::collections::VecDeque;
use std::io::{BufRead, Error, ErrorKind, Write};
use std::sync::mpsc::{Receiver, Sender, channel};

/// One message read: the input's end, a body that isn't JSON, or a
/// message.
pub type Read = std::io::Result<Option<Result<Value, String>>>;

/// What the server waits for: a message from the editor, or a check
/// done on the checking thread (`checking.rs`).
pub enum Event {
    Read(Read),
    Checked(Box<crate::checking::Done>),
}

/// Where the server's messages come from, and whether another one has
/// come already: then a check of a file that is about to change again
/// can wait (`Server::flush`).
pub trait Source {
    fn next(&mut self) -> Event;
    fn waiting(&mut self) -> bool;
    /// Where a checking thread sends what it found. `None`: files are
    /// checked on the server's own thread.
    fn checks(&self) -> Option<Sender<Event>> {
        None
    }
}

/// Messages read as they are asked for: none is ever waiting.
pub struct Plain<R>(pub R);

impl<R: BufRead> Source for Plain<R> {
    fn next(&mut self) -> Event {
        Event::Read(read(&mut self.0))
    }
    fn waiting(&mut self) -> bool {
        false
    }
}

/// Messages read by a thread of their own, so the server sees what has
/// come while it worked; checks run on another thread, and what they
/// found comes in among the messages.
pub struct Threaded {
    to: Sender<Event>,
    from: Receiver<Event>,
    ahead: VecDeque<Event>,
}

impl Threaded {
    pub fn stdin() -> Threaded {
        Threaded::new(std::io::BufReader::new(std::io::stdin()))
    }

    pub fn new(mut input: impl BufRead + Send + 'static) -> Threaded {
        let (to, from) = channel();
        let reader = to.clone();
        std::thread::spawn(move || {
            loop {
                let message = read(&mut input);
                let last = !matches!(message, Ok(Some(_)));
                if reader.send(Event::Read(message)).is_err() || last {
                    break;
                }
            }
        });
        Threaded {
            to,
            from,
            ahead: VecDeque::new(),
        }
    }
}

impl Source for Threaded {
    fn next(&mut self) -> Event {
        match self.ahead.pop_front() {
            Some(event) => event,
            None => self.from.recv().unwrap_or(Event::Read(Ok(None))),
        }
    }
    fn waiting(&mut self) -> bool {
        while let Ok(event) = self.from.try_recv() {
            self.ahead.push_back(event);
        }
        !self.ahead.is_empty()
    }
    fn checks(&self) -> Option<Sender<Event>> {
        Some(self.to.clone())
    }
}

/// Reads one message: `Ok(None)` when the input ends, `Ok(Some(Err(…)))`
/// when its body is not JSON (the server answers that with an error).
pub fn read(input: &mut impl BufRead) -> Read {
    let mut length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            if length.is_some() {
                break;
            }
            continue;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0; length.unwrap_or(0)];
    input
        .read_exact(&mut body)
        .map_err(|e| Error::new(ErrorKind::UnexpectedEof, format!("message cut short: {e}")))?;
    Ok(Some(
        serde_json::from_slice(&body).map_err(|e| e.to_string()),
    ))
}

/// Writes one message.
pub fn write(output: &mut impl Write, message: &Value) -> std::io::Result<()> {
    let body = message.to_string();
    write!(output, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::{Event, Source};
    use serde_json::{Value, json};
    use std::collections::VecDeque;

    /// Every message has come before the first is read, as when the
    /// editor sends edits faster than they are checked.
    struct Ahead(VecDeque<Value>);

    impl Source for Ahead {
        fn next(&mut self) -> Event {
            Event::Read(Ok(self.0.pop_front().map(Ok)))
        }
        fn waiting(&mut self) -> bool {
            !self.0.is_empty()
        }
    }

    #[test]
    fn edits_that_come_together_are_checked_once() {
        let uri = "file:///tmp/kavach-lsp-ahead.kav";
        let change = |text: &str| {
            json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
                "textDocument": {"uri": uri}, "contentChanges": [{"text": text}]}})
        };
        let messages = vec![
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {"uri": uri, "text": "print(a)\n"}}}),
            change("print(b)\n"),
            change("print(c)\n"),
        ];
        let mut output = Vec::new();
        crate::serve_from(&mut Ahead(messages.into()), &mut output).unwrap();
        let mut rest = &output[..];
        let mut published = Vec::new();
        while let Some(m) = super::read(&mut rest).unwrap() {
            let m = m.unwrap();
            if m["method"] == "textDocument/publishDiagnostics" {
                published.push(m["params"]["diagnostics"].to_string());
            }
        }
        // One check, of the last text: `c` is unknown, `a` and `b` never
        // were checked.
        assert_eq!(published.len(), 1, "{published:?}");
        assert!(published[0].contains("`c`"), "{published:?}");
    }
}
