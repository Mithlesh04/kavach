//! The signatures of the built-in functions of `std.devices`' serial
//! ports (spec 14.18), by the port's number, and of `std.clipboard`
//! (14.17).

use crate::checker::Checker;
use crate::ty::{Prim, Ty};

type Signature = (Vec<(String, Ty, bool)>, Ty);

impl Checker<'_> {
    /// The parameters and result of `devices.<name>`, or `None`.
    pub(crate) fn devices_signature(&mut self, name: &str) -> Option<Signature> {
        let (int, string) = (Ty::Prim(Prim::Int), Ty::Prim(Prim::String));
        let bytes = Ty::Prim(Prim::Bytes);
        let p = |n: &str, t: &Ty| (n.to_string(), t.clone(), false);
        let e = self.builtin_ty("DeviceError");
        let fails = |t: Ty| Ty::Result(Box::new(t), Box::new(e.clone()));
        Some(match name {
            "serial_ports" => (vec![], Ty::List(Box::new(string))),
            "open_serial_id" => (vec![p("path", &string), p("baud", &int)], fails(int)),
            "read_serial" => (vec![p("id", &int), p("max", &int)], fails(bytes)),
            "write_serial" => (vec![p("id", &int), p("data", &bytes)], fails(Ty::Unit)),
            "close_serial" => (vec![p("id", &int)], Ty::Unit),
            _ => return None,
        })
    }

    /// The parameters and result of `clipboard.<name>`, or `None`.
    pub(crate) fn clipboard_signature(&mut self, name: &str) -> Option<Signature> {
        let string = Ty::Prim(Prim::String);
        let e = self.builtin_ty("ClipboardError");
        let fails = |t: Ty| Ty::Result(Box::new(t), Box::new(e.clone()));
        Some(match name {
            "read_text" => (vec![], fails(string)),
            "write_text" => (vec![(String::from("text"), string, false)], fails(Ty::Unit)),
            _ => return None,
        })
    }
}
