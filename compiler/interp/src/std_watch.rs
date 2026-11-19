// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.fs`'s `Watcher` in the interpreter (spec 14.3), from
//! `kavach_runtime_watch`, by the watcher's number, as native code does.
//! A path is checked against `uses Fs("…")` when its watcher starts.
//! Under `kavach test` the files are fake (11.5), and a watcher of them
//! sees no changes.

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_checker::Prim;
use kavach_runtime_watch as watch;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// `fs.watch_open_id`, `watch_next_id`, `watch_poll_id` and
    /// `watch_close_id`.
    pub(crate) fn watch_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        let id = match args.first() {
            Some(Value::Int(n, _)) => *n as i64,
            _ => 0,
        };
        let row = |me: &Self, (kind, path): (u8, String)| me.str_value(&format!("{kind} {path}"));
        let fake = self.faking("Fs");
        Ok(match name {
            "watch_open_id" if fake => Value::Int(-1, Prim::Int),
            "watch_open_id" => {
                let path = match args.first() {
                    Some(Value::Str(s)) => s.to_string(),
                    _ => String::new(),
                };
                if !kavach_runtime_core::fs_scope::inside(&self.fs_scopes, &path) {
                    let e = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
                    return Err(self.fs_error(&path, e));
                }
                let n = watch::watch(&path).map_err(|e| self.fs_error(&path, e))?;
                Value::Int(i128::from(n), Prim::Int)
            }
            // The fake file system never changes (spec 14.3).
            "watch_next_id" if fake || id < 0 => self.str_value(""),
            "watch_next_id" => {
                // Held while waiting: the task keeps its turn, as for a window.
                self.flush();
                match watch::next(id) {
                    Some(c) => row(self, c),
                    None => self.str_value(""),
                }
            }
            "watch_poll_id" => {
                let all = watch::poll(id).into_iter().map(|c| row(self, c)).collect();
                Value::List(Rc::new(all))
            }
            _ => {
                watch::close(id);
                Value::Unit
            }
        })
    }
}
