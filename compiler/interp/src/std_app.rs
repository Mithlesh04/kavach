// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.app` in the interpreter (spec 14.30): folders and crash reports
//! from `kavach_runtime_core::app`, dialogs from `kavach_runtime_dialog`,
//! as native code does. Under `kavach test` folders are fake paths and no
//! folder is made.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_runtime_core::app::{self, Kind};

impl<'a> Interp<'a> {
    /// `app.dir_id`, `crash_id` and `dialog_id`.
    pub(crate) fn app_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        let int = |i: usize| match args.get(i) {
            Some(Value::Int(n, _)) => *n as i64,
            _ => 0,
        };
        let text = |i: usize| match args.get(i) {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let fail = |me: &Self, why: String| {
            let reason = me.str_value(&why);
            Flow::Fail(me.builtin_variant("AppError", "Unavailable", vec![("reason", reason)]))
        };
        // Under `kavach test` the files are fake (11.5): no real folder is
        // made, and no report would be written.
        if self.faking("Fs") && name != "dialog_id" {
            return Ok(match name {
                "dir_id" => {
                    let kind = ["data", "config", "cache"][int(0).clamp(0, 2) as usize];
                    self.str_value(&format!("/fake/{kind}/{}", text(1)))
                }
                _ => Value::Unit,
            });
        }
        Ok(match name {
            "dir_id" => {
                let kind = [Kind::Data, Kind::Config, Kind::Cache][int(0).clamp(0, 2) as usize];
                match app::folder(kind, &text(1)) {
                    Ok(path) => self.str_value(&path),
                    Err(why) => return Err(fail(self, why)),
                }
            }
            "crash_id" => match app::crash_reports(&text(0), &self.c.path) {
                Ok(()) => Value::Unit,
                Err(why) => return Err(fail(self, why)),
            },
            _ => {
                let got = kavach_runtime_dialog::show(int(0), &text(1), &text(2));
                self.str_value(&got)
            }
        })
    }
}
