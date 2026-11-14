//! Streamed reads in native code (spec 14.3), as in the interpreter
//! (`compiler/interp/src/files.rs`): `fs.lines(path)` and `fs.chunks(path,
//! size)` read as a `for` loop goes. Reading is an effect, so a task
//! running ahead waits for its turn first (`rt_tasks::turn`). Copies of a
//! reader share the open file. Big pieces are read ahead on a helper
//! thread, so the disk and the loop's work overlap (decision 0067).

use crate::heap::{Data, alloc, release};
use crate::rt::{panic_with, type_of};
use crate::shapes::{shape, str_of};
use kavach_kir::{RtOp, Shape, Type};
use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc::{Receiver, sync_channel};
use std::sync::{Arc, Mutex};

/// Pieces of at least this size are read ahead: below it, a read is cheap
/// next to the thread.
const READ_AHEAD: usize = 1024 * 1024;

pub struct Reader {
    file: BufReader<std::fs::File>,
    path: String,
    chunk: Option<usize>,
    /// The pieces the helper thread reads, once it runs.
    ahead: Option<Receiver<std::io::Result<Words>>>,
}

/// A big piece of a file: its bytes in whole words, so a list can take the
/// buffer as it is (`rt_bytes.rs`), and how many bytes.
struct Words(Vec<u64>, usize);

/// One piece of at most `size` bytes of `file`; empty at the end.
fn read_piece(mut file: impl Read, size: usize) -> std::io::Result<Vec<u8>> {
    let mut piece = Vec::with_capacity(size);
    file.by_ref().take(size as u64).read_to_end(&mut piece)?;
    Ok(piece)
}

/// One big piece of at most `size` bytes of `file`: read straight into the
/// memory it stays in. Its last word is filled up with zeros, as a list's
/// buffer must be.
#[allow(unsafe_code)]
fn read_words(mut file: impl Read, size: usize) -> std::io::Result<Words> {
    let room = size.div_ceil(8);
    let mut words: Vec<u64> = Vec::with_capacity(room);
    // SAFETY: the capacity holds `room` words, and only the reads below and
    // the `fill` write to them. A file's read only writes to what it is
    // given, and no byte is read before it is written: `set_len` covers just
    // the bytes read and the zeros after them. Not zeroed first, as that
    // would touch every page of a buffer the disk is about to fill.
    let bytes = unsafe { std::slice::from_raw_parts_mut(words.as_mut_ptr() as *mut u8, room * 8) };
    let mut got = 0;
    while got < size {
        match file.read(&mut bytes[got..size]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    let whole = got.div_ceil(8) * 8;
    bytes[got..whole].fill(0);
    // SAFETY: the first `whole` bytes are written, a whole number of words.
    unsafe { words.set_len(whole / 8) };
    if whole / 8 < room {
        words.shrink_to_fit();
    }
    Ok(Words(words, got))
}

impl Reader {
    /// Starts the helper thread on its own handle of the file, which reads
    /// one piece more than the loop has asked for and waits to hand it
    /// over. It ends at the file's end or when the reader is gone.
    fn start_ahead(&mut self, size: usize) -> std::io::Result<()> {
        let file = self.file.get_ref().try_clone()?;
        let (give, take) = sync_channel(0);
        std::thread::spawn(move || {
            loop {
                let piece = read_words(&file, size);
                let last = !matches!(&piece, Ok(p) if p.1 > 0);
                if give.send(piece).is_err() || last {
                    return;
                }
            }
        });
        self.ahead = Some(take);
        Ok(())
    }

    /// The next big piece of `size` bytes; empty at the end.
    fn next_big(&mut self, size: usize) -> std::io::Result<Words> {
        if self.ahead.is_none() {
            self.start_ahead(size)?;
        }
        match self.ahead.as_ref().map(Receiver::recv) {
            Some(Ok(piece)) => piece,
            // The helper has ended, at the file's end or after an error.
            _ => Ok(Words(Vec::new(), 0)),
        }
    }
}

pub type Shared = Arc<Mutex<Reader>>;

/// 0 with `Denied(path)` set: `path` is outside this task's folders.
pub(crate) fn denied(path: &str) -> i64 {
    let e = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    crate::rt_std::fail("open", path, e)
}

/// `fs.lines(path)` (`size` 0) and `fs.chunks(path, size)`: a reader of
/// type `code`, or 0 with the `FsError` set.
pub fn open(path: i64, size: Option<i64>, code: i64) -> i64 {
    crate::rt_tasks::turn();
    let p = str_of(path);
    release(path);
    if !crate::rt_scopes::allowed(&p) {
        return denied(&p);
    }
    match std::fs::File::open(&p) {
        Ok(f) => alloc(
            type_of(code),
            Data::Reader(Arc::new(Mutex::new(Reader {
                file: BufReader::new(f),
                path: p,
                chunk: size.map(|n| n.max(1) as usize),
                ahead: None,
            }))),
        ),
        Err(e) => crate::rt_std::fail("open", &p, e),
    }
}

/// The next line or piece of reader `r` (owned) as an optional of type
/// `code`: `none` at the end. A failed read is 0 with the `FsError` set
/// when the loop's header handles it (`fails`, spec 14.3), else a panic.
pub fn next(r: i64, code: i64, fails: bool, line: i32) -> i64 {
    crate::rt_tasks::turn();
    let reader = match &*crate::heap::data(r).borrow() {
        Data::Reader(x) => Arc::clone(x),
        _ => return 0,
    };
    release(r);
    let mut rd = reader.lock().unwrap_or_else(|e| e.into_inner());
    let item = match shape(type_of(code)) {
        Shape::Optional(t) => *t,
        _ => Type::Unit,
    };
    let failed = |path: &str, e: std::io::Error| -> i64 {
        if fails {
            return crate::rt_std::fail("read", path, e);
        }
        panic_with(&format!("can't read {path}: {e}"), line)
    };
    match rd.chunk {
        None => {
            let mut raw = Vec::new();
            let n = match rd.file.read_until(b'\n', &mut raw) {
                Ok(n) => n,
                Err(e) => return failed(&rd.path, e),
            };
            if n == 0 {
                return 0;
            }
            if raw.ends_with(b"\n") {
                raw.pop();
                if raw.ends_with(b"\r") {
                    raw.pop();
                }
            }
            match String::from_utf8(raw) {
                Ok(text) => crate::shapes::new_text(&text, item),
                Err(_) => {
                    let e = std::io::Error::new(std::io::ErrorKind::InvalidData, "not UTF-8");
                    failed(&rd.path, e)
                }
            }
        }
        Some(size) if size >= READ_AHEAD => match rd.next_big(size) {
            Ok(Words(_, 0)) => 0,
            Ok(Words(words, len)) => crate::rt_bytes::new_words(words, len, item),
            Err(e) => failed(&rd.path, e),
        },
        Some(size) => {
            let piece = match read_piece(&mut rd.file, size) {
                Ok(piece) => piece,
                Err(e) => return failed(&rd.path, e),
            };
            if piece.is_empty() {
                return 0;
            }
            crate::rt_bytes::new_bytes(piece, item)
        }
    }
}

/// The reader ops (operands owned).
pub fn reader_op(op: RtOp, a: i64, b: i64, code: i64, line: i32) -> i64 {
    match op {
        RtOp::FsLines => open(a, None, code),
        RtOp::FsChunks => open(a, Some(b), code),
        _ => next(a, code, op == RtOp::ReaderTryNext, line),
    }
}
