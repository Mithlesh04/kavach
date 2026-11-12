//! A file's details and permissions, renaming, copying and removing a
//! folder with its contents (spec 14.3), through
//! `kavach_runtime_core::file_info`, as native code does.

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_checker::Prim;
use kavach_runtime_core::file_info;
use std::rc::Rc;

impl<'a> Interp<'a> {
    pub(crate) fn fs_details_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        let text = |i: usize| match args.get(i) {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let path = text(0);
        let done = match name {
            "stat" => {
                let n = file_info::stat(&path).map_err(|e| self.fs_error(&path, e))?;
                let items = n
                    .iter()
                    .map(|v| Value::Int(i128::from(*v), Prim::Int))
                    .collect();
                return Ok(Value::List(Rc::new(items)));
            }
            "set_read_only" => {
                file_info::set_read_only(&path, matches!(args.get(1), Some(Value::Bool(true))))
            }
            "set_permissions" => {
                let mode = match args.get(1) {
                    Some(Value::Int(n, _)) => i64::try_from(*n).unwrap_or(-1),
                    _ => -1,
                };
                file_info::set_permissions(&path, mode)
            }
            "rename" => file_info::rename(&path, &text(1)),
            "copy" => file_info::copy(&path, &text(1)),
            _ => file_info::remove_all(&path),
        };
        done.map_err(|e| self.fs_error(&path, e))?;
        Ok(Value::Unit)
    }
}
