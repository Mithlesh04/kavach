// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.font` in the interpreter (spec 14.31), from
//! `kavach_runtime_font`, as native code does.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use kavach_runtime_font as font;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// `font.metrics_id`, `shape_id`, `atlas_id`, `atlas_base_id`,
    /// `atlas_boxes_id`, `atlas_pixels_id`, and the installed fonts':
    /// `installed_id`, `fallback_id` and `file_id`.
    pub(crate) fn font_call(&mut self, name: &str, args: &[Value<'a>], span: Span) -> Eval<'a> {
        let bytes = match args.first() {
            Some(Value::Bytes(b)) => Rc::clone(b),
            _ => Rc::from(&[][..]),
        };
        let id = match args.first() {
            Some(Value::Int(n, _)) => *n as i64,
            _ => 0,
        };
        let text = match args.get(1) {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let size = match args.get(2) {
            Some(Value::Float(f)) => *f,
            _ => 0.0,
        };
        let first_text = match args.first() {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let numbers = |i: usize| match args.get(i) {
            Some(Value::List(items)) => (items.iter())
                .map(|v| match v {
                    Value::Float(f) => *f,
                    Value::Int(n, _) => *n as f64,
                    _ => 0.0,
                })
                .collect(),
            _ => Vec::new(),
        };
        let floats = |v: Vec<f64>| {
            let items: Vec<Value<'a>> = v.into_iter().map(Value::Float).collect();
            Value::List(Rc::new(items.into()))
        };
        Ok(match name {
            "metrics_id" => match font::metrics(&bytes) {
                Ok(m) => floats(m.to_vec()),
                Err(why) => {
                    let reason = self.str_value(&why);
                    return Err(Flow::Fail(self.builtin_variant(
                        "FontError",
                        "BadFont",
                        vec![("reason", reason)],
                    )));
                }
            },
            "shape_id" => floats(font::shape_flat(&bytes, &text, size)),
            "atlas_id" => {
                let s: Vec<f64> = numbers(2);
                let part = |k: usize| s.get(k).copied().unwrap_or(-1.0);
                match font::atlas_kept(&bytes, &text, part(0), part(1) > 0.5, part(2) as i64) {
                    Ok(n) => Value::Int(i128::from(n), Prim::Int),
                    Err(why) => return Err(self.panic(why, span)),
                }
            }
            "atlas_base_id" => {
                let layout: Vec<i64> = numbers(1).into_iter().map(|v| v as i64).collect();
                let n = font::keep_base(&bytes, &layout, size);
                Value::Int(i128::from(n), Prim::Int)
            }
            "installed_id" => {
                let all = font::installed_flat();
                let items: Vec<Value<'a>> = all.iter().map(|t| self.str_value(t)).collect();
                Value::List(Rc::new(items.into()))
            }
            "fallback_id" => {
                let k = font::fallback(&first_text).map_or(-1, |k| k as i64);
                Value::Int(i128::from(k), Prim::Int)
            }
            "file_id" => Value::Bytes(font::read_installed(&first_text).into()),
            "atlas_boxes_id" => {
                let items: Vec<Value<'a>> = (font::atlas_boxes(id).into_iter())
                    .map(|n| Value::Int(i128::from(n), Prim::Int))
                    .collect();
                Value::List(Rc::new(items.into()))
            }
            _ => Value::Bytes(font::take_atlas_pixels(id).into()),
        })
    }
}
