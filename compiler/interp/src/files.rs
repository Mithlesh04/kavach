//! Streamed reads (spec 14.3): `fs.lines(path)` and `fs.chunks(path,
//! size)`, read as a `for` loop goes, so only the part being looked at
//! is in memory. Native code does the same (`runtime/native/src/rt_files.rs`).

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{Expr, ExprKind, Span};
use std::cell::RefCell;
use std::io::{BufRead, BufReader, Read};
use std::rc::Rc;

/// An open file being read: its lines, or pieces of `chunk` bytes.
pub struct Reader {
    file: BufReader<Box<dyn Read>>,
    path: String,
    chunk: Option<usize>,
}

impl std::fmt::Debug for Reader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Reader({})", self.path)
    }
}

pub type Shared = Rc<RefCell<Reader>>;

/// The next line or piece of `r`, or `None` at the end; `Err` is the panic
/// message (native code says the same).
pub fn next(r: &mut Reader) -> std::io::Result<Option<Next>> {
    match r.chunk {
        None => {
            let mut raw = Vec::new();
            let n = r.file.read_until(b'\n', &mut raw)?;
            if n == 0 {
                return Ok(None);
            }
            if raw.ends_with(b"\n") {
                raw.pop();
                if raw.ends_with(b"\r") {
                    raw.pop();
                }
            }
            let line = String::from_utf8(raw).map_err(|_| not_utf8())?;
            Ok(Some(Next::Line(line)))
        }
        Some(size) => {
            let mut piece = Vec::with_capacity(size);
            (&mut r.file).take(size as u64).read_to_end(&mut piece)?;
            Ok((!piece.is_empty()).then_some(Next::Piece(piece)))
        }
    }
}

/// A line that isn't UTF-8 (spec 14.3): `Failed(path, "not UTF-8")`.
pub fn not_utf8() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, "not UTF-8")
}

pub enum Next {
    Line(String),
    Piece(Vec<u8>),
}

impl<'a> Interp<'a> {
    /// `fs.lines(path)` and `fs.chunks(path, size)`: the file opened.
    pub(crate) fn open_reader(&mut self, path: &str, chunk: Option<usize>) -> Eval<'a> {
        let file = std::fs::File::open(path).map_err(|e| self.fs_error(path, e))?;
        Ok(Value::Reader(Rc::new(RefCell::new(Reader {
            file: BufReader::new(Box::new(file)),
            path: path.to_string(),
            chunk,
        }))))
    }

    /// A reader of `data` held in memory (a generated test's file).
    pub(crate) fn reader_of(&self, data: Vec<u8>, path: &str, chunk: Option<usize>) -> Value<'a> {
        Value::Reader(Rc::new(RefCell::new(Reader {
            file: BufReader::new(Box::new(std::io::Cursor::new(data))),
            path: path.to_string(),
            chunk,
        })))
    }

    /// The next item of a `for` loop over a reader, or `None` at the end.
    /// A failed read goes to the `try` or `catch` of the loop's header
    /// `iter` (spec 14.3): `try` returns the error, `catch` runs its
    /// handler and ends the loop. Without either it is a panic.
    pub(crate) fn reader_next(
        &mut self,
        r: &Shared,
        iter: &'a Expr,
        span: Span,
    ) -> Result<Option<Value<'a>>, Flow<'a>> {
        let read = next(&mut r.borrow_mut());
        let item = match read {
            Ok(item) => item,
            Err(e) => {
                let path = r.borrow().path.clone();
                return match &iter.kind {
                    ExprKind::Try { .. } | ExprKind::Catch { .. } => {
                        let Flow::Fail(err) = self.fs_error(&path, e) else {
                            unreachable!("fs_error gives a failure")
                        };
                        self.header_failed(iter, err).map(|()| None)
                    }
                    _ => Err(self.panic(format!("can't read {path}: {e}"), span)),
                };
            }
        };
        Ok(item.map(|n| match n {
            Next::Line(l) => self.str_value(&l),
            Next::Piece(p) => Value::Bytes(p.into()),
        }))
    }

    /// Error `err` from a read in the loop whose header is `iter`, handled
    /// as the header's `try` or `catch` handles one (spec 14.3): `Ok`
    /// when the loop should end.
    fn header_failed(&mut self, iter: &'a Expr, err: Value<'a>) -> Result<(), Flow<'a>> {
        match &iter.kind {
            ExprKind::Try { context, .. } => {
                let err = match context {
                    Some(c) => {
                        let text = self.eval(c)?.display();
                        Value::Context(Rc::from(text), Box::new(err))
                    }
                    None => err,
                };
                // It leaves the function (spec 5.3).
                self.leaving = true;
                Err(Flow::Fail(err))
            }
            ExprKind::Catch {
                binding, handler, ..
            } => {
                if let Some(def) = binding
                    .as_ref()
                    .and_then(|b| self.decl.get(&b.span.start).copied())
                {
                    self.frame().vars.insert(def, err);
                }
                match self.eval(handler) {
                    Ok(_) | Err(Flow::Break(None) | Flow::Continue(None)) => Ok(()),
                    Err(other) => Err(other),
                }
            }
            _ => Ok(()),
        }
    }
}
