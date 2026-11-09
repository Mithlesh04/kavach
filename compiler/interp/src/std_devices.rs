//! What is connected to the machine (spec 14.17, 14.18): the clipboard
//! and serial ports, each behind its capability; under `kavach test` the
//! clipboard is the case's fake.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_checker::Prim;

impl<'a> Interp<'a> {
    /// Serial ports (spec 14.18), by the port's number.
    pub(crate) fn devices_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        use kavach_runtime_core::serial::{self, DevError};
        let int = |i: usize| match args.get(i) {
            Some(Value::Int(n, _)) => (*n).clamp(i64::MIN as i128, i64::MAX as i128) as i64,
            _ => 0,
        };
        let done = match name {
            "serial_ports" => {
                let names = serial::ports().iter().map(|n| self.str_value(n)).collect();
                return Ok(Value::List(std::rc::Rc::new(names)));
            }
            "close_serial" => {
                serial::close(int(0));
                return Ok(Value::Unit);
            }
            "open_serial_id" => {
                let path = match args.first() {
                    Some(Value::Str(s)) => s.to_string(),
                    _ => String::new(),
                };
                serial::open(&path, int(1)).map(|id| Value::Int(id as i128, Prim::Int))
            }
            "read_serial" => serial::read(int(0), int(1)).map(|b| Value::Bytes(b.into())),
            _ => {
                let data = match args.get(1) {
                    Some(Value::Bytes(b)) => b.to_vec(),
                    _ => Vec::new(),
                };
                serial::write(int(0), &data).map(|()| Value::Unit)
            }
        };
        done.map_err(|e| {
            let text = |s: &str| self.str_value(s);
            let error = match e {
                DevError::NotFound(path) => {
                    self.builtin_variant("DeviceError", "NotFound", vec![("path", text(&path))])
                }
                DevError::Denied(path) => {
                    self.builtin_variant("DeviceError", "Denied", vec![("path", text(&path))])
                }
                DevError::Failed(path, why) => self.builtin_variant(
                    "DeviceError",
                    "Failed",
                    vec![("path", text(&path)), ("reason", text(&why))],
                ),
            };
            Flow::Fail(error)
        })
    }

    /// `clipboard.read_text()` and `write_text(text)` (spec 14.17): the
    /// system's clipboard, or under `kavach test` the case's fake one.
    pub(crate) fn clipboard_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        use kavach_runtime_core::clipboard::{self, ClipError};
        let text = match args.first() {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let done = match (&self.fake, name) {
            (Some(world), "read_text") => world.borrow().clipboard.clone().ok_or(ClipError::Empty),
            (Some(world), _) => {
                world.borrow_mut().clipboard = Some(text);
                Ok(String::new())
            }
            (None, "read_text") => clipboard::read_text(),
            (None, _) => clipboard::write_text(&text).map(|()| String::new()),
        };
        match (done, name) {
            (Ok(text), "read_text") => Ok(self.str_value(&text)),
            (Ok(_), _) => Ok(Value::Unit),
            (Err(ClipError::Empty), _) => Err(Flow::Fail(self.builtin_variant(
                "ClipboardError",
                "Empty",
                Vec::new(),
            ))),
            (Err(ClipError::Unavailable(why)), _) => {
                let reason = vec![("reason", self.str_value(&why))];
                Err(Flow::Fail(self.builtin_variant(
                    "ClipboardError",
                    "Unavailable",
                    reason,
                )))
            }
        }
    }
}
