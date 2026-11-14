//! `std.fs`'s `FileWriter` in the interpreter (spec 14.3), from
//! `kavach_runtime_core::writers`, by the writer's number, as native code
//! does. A path is checked against `uses Fs("…")` when its writer opens;
//! after that the calls name the writer. Under `kavach test` the file is
//! the fake file system's (11.5): the pieces are gathered here and written
//! there at `finish`.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_checker::Prim;
use kavach_runtime_core::writers;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A writer on the fake file system: its path, the bytes so far, and
/// whether it is finished.
type FakeWriter = (String, Vec<u8>, bool);

thread_local! {
    static FAKES: RefCell<HashMap<i64, FakeWriter>> = RefCell::new(HashMap::new());
}

impl<'a> Interp<'a> {
    /// `fs.writer_open_id`, `writer_write_id`, `writer_finish_id` and
    /// `writer_close_id`.
    pub(crate) fn writer_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        let id = match args.first() {
            Some(Value::Int(n, _)) => *n as i64,
            _ => 0,
        };
        if self.faking("Fs") {
            return self.fake_writer_call(name, id, args);
        }
        Ok(match name {
            "writer_open_id" => {
                let path = match args.first() {
                    Some(Value::Str(s)) => s.to_string(),
                    _ => String::new(),
                };
                if !kavach_runtime_core::fs_scope::inside(&self.fs_scopes, &path) {
                    let e = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
                    return Err(self.fs_error(&path, e));
                }
                let n = writers::open(&path).map_err(|e| self.fs_error(&path, e))?;
                Value::Int(i128::from(n), Prim::Int)
            }
            "writer_write_id" => {
                let data = match args.get(1) {
                    Some(Value::Bytes(b)) => Rc::clone(b),
                    _ => Rc::from(&[][..]),
                };
                writers::write(id, &data).map_err(|e| self.fs_error(&writers::path(id), e))?;
                Value::Unit
            }
            "writer_finish_id" => {
                let sync = matches!(args.get(1), Some(Value::Bool(true)));
                let n =
                    writers::finish(id, sync).map_err(|e| self.fs_error(&writers::path(id), e))?;
                Value::Int(i128::from(n), Prim::Int)
            }
            _ => {
                writers::close(id);
                Value::Unit
            }
        })
    }

    /// The same on the fake file system of `kavach test`.
    fn fake_writer_call(&mut self, name: &str, id: i64, args: &[Value<'a>]) -> Eval<'a> {
        let finished = |path: &str, me: &Self| -> Flow<'a> {
            me.fs_error(path, std::io::Error::other("finished"))
        };
        Ok(match name {
            "writer_open_id" => {
                let path = args.first().cloned().unwrap_or(Value::Unit);
                // Made empty now, as a real one is.
                self.fake_fs(
                    "write_bytes",
                    &[path.clone(), Value::Bytes(Rc::from(&[][..]))],
                )?;
                let text = match path {
                    Value::Str(s) => s.to_string(),
                    _ => String::new(),
                };
                let n = FAKES.with(|f| {
                    let mut f = f.borrow_mut();
                    let n = -(f.len() as i64) - 1;
                    f.insert(n, (text, Vec::new(), false));
                    n
                });
                Value::Int(i128::from(n), Prim::Int)
            }
            "writer_write_id" => {
                let data = match args.get(1) {
                    Some(Value::Bytes(b)) => b.to_vec(),
                    _ => Vec::new(),
                };
                let done = FAKES.with(|f| match f.borrow_mut().get_mut(&id) {
                    Some((_, bytes, false)) => {
                        bytes.extend(data);
                        None
                    }
                    Some((path, _, true)) => Some(path.clone()),
                    None => Some(String::new()),
                });
                if let Some(path) = done {
                    return Err(finished(&path, self));
                }
                Value::Unit
            }
            "writer_finish_id" => {
                let got = FAKES.with(|f| {
                    f.borrow_mut().get_mut(&id).map(|(path, bytes, done)| {
                        let first = !*done;
                        *done = true;
                        (path.clone(), bytes.clone(), first)
                    })
                });
                let Some((path, bytes, first)) = got else {
                    return Err(finished("", self));
                };
                if first {
                    let at = self.str_value(&path);
                    self.fake_fs(
                        "write_bytes",
                        &[at, Value::Bytes(Rc::from(bytes.as_slice()))],
                    )?;
                }
                Value::Int(bytes.len() as i128, Prim::Int)
            }
            _ => {
                let gone = FAKES.with(|f| f.borrow_mut().remove(&id));
                if let Some((path, _, false)) = gone {
                    let at = self.str_value(&path);
                    let _ = self.fake_fs("remove", &[at]);
                }
                Value::Unit
            }
        })
    }
}
