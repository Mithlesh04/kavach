// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The standard library (spec 14): more methods of built-in types, sets,
//! and the modules brought in with `use std.<module>`.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{ItemKind, Span, TypeExpr};
use kavach_checker::Prim;
use kavach_runtime_core::regex::Pattern;
use std::io::IsTerminal;
use std::rc::Rc;

/// The error a failed file operation gives: a clear message.
/// Writes `text` to `path` through a temporary file that is then renamed,
/// so a crash never leaves half a file (spec 14.3).
pub(crate) fn write_atomically(path: &str, text: impl AsRef<[u8]>) -> std::io::Result<()> {
    kavach_runtime_core::files::write_atomically(path, text.as_ref())
}

impl<'a> Interp<'a> {
    /// A failed file operation as an `FsError` from `std.fs` (spec 14.3).
    pub(crate) fn fs_error(&self, path: &str, e: std::io::Error) -> Flow<'a> {
        let at = ("path", self.str_value(path));
        let error = match kavach_runtime_core::files::error_variant(path, &e) {
            "Failed" => {
                let reason = ("reason", self.str_value(&e.to_string()));
                self.builtin_variant("FsError", "Failed", vec![at, reason])
            }
            variant => self.builtin_variant("FsError", variant, vec![at]),
        };
        Flow::Fail(error)
    }

    /// Remembers which names `use std.<module>` brought in.
    pub(crate) fn collect_std_modules(&mut self) {
        for item in &self.c.file.items {
            let ItemKind::Use(u) = &item.kind else {
                continue;
            };
            let segs = &u.path.segments;
            if segs.len() == 2 && segs[0].name == "std" && u.names.is_empty() {
                let name = u.alias.as_ref().unwrap_or(&segs[1]);
                if let Some(&def) = self.decl.get(&name.span.start) {
                    self.std_modules.insert(def, segs[1].name.clone());
                }
            }
        }
    }

    pub(crate) fn std_constant(&self, module: &str, name: &str) -> Option<Value<'a>> {
        match (module, name) {
            ("math", "pi") => Some(Value::Float(std::f64::consts::PI)),
            ("math", "e") => Some(Value::Float(std::f64::consts::E)),
            _ => None,
        }
    }

    /// `module.name(args)`: a standard library function (spec 14.3–14.6).
    pub(crate) fn std_call(
        &mut self,
        module: &str,
        name: &str,
        generics: &'a [TypeExpr],
        args: Vec<Value<'a>>,
        span: Span,
    ) -> Eval<'a> {
        let text = |i: usize| match args.get(i) {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let float = |i: usize| match args.get(i) {
            Some(Value::Float(f)) => *f,
            _ => 0.0,
        };
        if module == "time" {
            return self.time_call(name, args, span);
        }
        if module == "clipboard" {
            return self.clipboard_call(name, &args);
        }
        match module {
            "crypto" => return self.crypto_call(name, &args, span),
            "base64" => return self.base64_call(name, &args),
            "compress" => return self.compress_call(name, &args),
            "window" => return self.window_call(name, &args),
            "input" => return self.input_call(name, &args),
            "audio" => return self.audio_call(name, &args),
            "image" => return self.image_call(name, &args, span),
            "font" => return self.font_call(name, &args, span),
            "app" => return self.app_call(name, &args),
            "process" => return self.process_call(name, &args, span),
            "system" => return self.system_call(name, &args),
            "matrix" => return self.matrix_call(name, &args, span),
            "gpu" => return self.gpu_call(name, &args, span),
            "log" => return self.log_call(name, &args),
            "fs" if name.starts_with("writer_") => return self.writer_call(name, &args),
            "fs" if name.starts_with("watch_") => return self.watch_call(name, &args),
            "text" if name.starts_with("nf") => return Ok(self.normal_form(name, &text(0))),
            _ => {}
        }
        if module == "devices" {
            return self.devices_call(name, &args);
        }
        // What the run's `ask`s took, or one agent's (`ai.spent`, 8.4).
        if (module, name) == ("ai", "spent_text") {
            let spent = kavach_runtime_core::agents::costs::spent_json(&text(0));
            return Ok(self.str_value(&spent));
        }
        if (module, name) == ("agents", "request_fields")
            || (module, name) == ("agents", "answer_request")
        {
            return self.approvals_call(name, &args);
        }
        // The running call's idempotency key (spec 14.16): only inside a
        // capability's implementation, in a durable agent's event.
        if (module, name) == ("agents", "call_key") {
            let key = match (self.in_call, self.running.last()) {
                (0, _) | (_, None) => None,
                (_, Some(agent)) => agent
                    .journal
                    .as_ref()
                    .zip(self.events.last())
                    .and_then(|(j, e)| j.borrow().call_key(*e, &agent.decl.name.name)),
            };
            return Ok(key.map_or(Value::None, |k| self.str_value(&k)));
        }
        if module == "net" {
            return self.net_call(name, &args, None, span);
        }
        let path = text(0);
        // Outside the folders of `uses Fs("…")` (spec 8.1): `rename` and
        // `copy` name two paths, and both must be inside.
        let inside = |p: &str| kavach_runtime_core::fs_scope::inside(&self.fs_scopes, p);
        let outside = match (inside(&path), name) {
            (false, _) => Some(path.clone()),
            (true, "rename" | "copy" | "symlink") if !inside(&text(1)) => Some(text(1)),
            _ => None,
        };
        if module == "fs"
            && let Some(path) = outside
        {
            if name == "exists" {
                return Ok(Value::Bool(false));
            }
            let at = ("path", self.str_value(&path));
            return Err(Flow::Fail(self.builtin_variant(
                "FsError",
                "Denied",
                vec![at],
            )));
        }
        // A generated test's files are in memory (spec 11.5).
        if module == "fs" && self.faking("Fs") {
            return self.fake_fs(name, &args);
        }
        if module == "fs" && crate::std_fs::DETAILS.contains(&name) {
            return self.fs_details_call(name, &args);
        }
        Ok(match (module, name) {
            ("fs", "read_text") => self
                .str_value(&std::fs::read_to_string(&path).map_err(|e| self.fs_error(&path, e))?),
            ("fs", "lines") => return self.open_reader(&path, None),
            ("fs", "chunks") => {
                let size = match args.get(1) {
                    Some(Value::Int(n, _)) => (*n).max(1) as usize,
                    _ => 1,
                };
                return self.open_reader(&path, Some(size));
            }
            ("fs", "read_bytes") => {
                let b = std::fs::read(&path).map_err(|e| self.fs_error(&path, e))?;
                Value::Bytes(b.into())
            }
            ("fs", "write_bytes") => {
                let data = match args.get(1) {
                    Some(Value::Bytes(b)) => b.clone(),
                    _ => Rc::from(Vec::new()),
                };
                write_atomically(&path, &*data).map_err(|e| self.fs_error(&path, e))?;
                Value::Unit
            }
            ("fs", "write_durable") => {
                let data = match args.get(1) {
                    Some(Value::Bytes(b)) => b.clone(),
                    _ => Rc::from(Vec::new()),
                };
                kavach_runtime_core::files::write_durably(&path, &data)
                    .map_err(|e| self.fs_error(&path, e))?;
                Value::Unit
            }
            ("fs", "write_text") => {
                write_atomically(&path, text(1)).map_err(|e| self.fs_error(&path, e))?;
                Value::Unit
            }
            ("fs", "append_text") => {
                use std::io::Write;
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .map_err(|e| self.fs_error(&path, e))?;
                f.write_all(text(1).as_bytes())
                    .map_err(|e| self.fs_error(&path, e))?;
                Value::Unit
            }
            ("fs", "exists") => Value::Bool(std::path::Path::new(&path).exists()),
            ("fs", "list") => {
                let mut names: Vec<String> = std::fs::read_dir(&path)
                    .map_err(|e| self.fs_error(&path, e))?
                    .flatten()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();
                names.sort();
                Value::List(Rc::new(names.iter().map(|n| self.str_value(n)).collect()))
            }
            ("fs", "remove") => {
                let p = std::path::Path::new(&path);
                let done = if p.is_dir() {
                    std::fs::remove_dir(p)
                } else {
                    std::fs::remove_file(p)
                };
                done.map_err(|e| self.fs_error(&path, e))?;
                Value::Unit
            }
            ("fs", "create_dir") => {
                std::fs::create_dir_all(&path).map_err(|e| self.fs_error(&path, e))?;
                Value::Unit
            }
            // A `Float32` is worked out as a `Float`, then rounded once, as in
            // native code (spec 14.5).
            ("math", n)
                if matches!(args.first(), Some(Value::Float32(_))) && n != "pi" && n != "e" =>
            {
                let wide = |i: usize| match args.get(i) {
                    Some(Value::Float32(f)) => f64::from(*f),
                    _ => 0.0,
                };
                let x = wide(0);
                match crate::std_math::float_fn(name, x, wide(1)) {
                    Some(v) => Value::Float32(v as f32),
                    None => {
                        return Err(self.panic(format!("`math.{name}` takes no `Float32`"), span));
                    }
                }
            }
            ("math", n) if crate::std_math::float_fn(n, 0.0, 0.0).is_some() => {
                Value::Float(crate::std_math::float_fn(n, float(0), float(1)).unwrap_or(0.0))
            }
            ("math", "sqrt") => Value::Float(float(0).sqrt()),
            ("math", "pow") => Value::Float(kavach_runtime_core::math::pow(float(0), float(1))),
            ("math", "floor") => Value::Float(float(0).floor()),
            ("math", "ceil") => Value::Float(float(0).ceil()),
            ("math", "round") => Value::Float(float(0).round()),
            ("io", "read_line" | "read_hidden") if self.testing => Value::None,
            ("io", "read_hidden") => {
                self.show_live();
                match kavach_runtime_core::input::read_hidden() {
                    Some(line) => self.str_value(&line),
                    None => Value::None,
                }
            }
            // Tests capture what is printed (spec 11.5): not a terminal.
            ("io", "is_terminal") => Value::Bool(!self.testing && std::io::stdout().is_terminal()),
            ("io", "colors") => Value::Bool(!self.testing && kavach_runtime_core::log::colors()),
            ("io", "write") => {
                if let Some(Value::Str(s)) = args.first() {
                    self.out.push_str(s);
                }
                self.show_live();
                Value::Unit
            }
            ("io", "read_line") => {
                // A question printed first shows before the program waits.
                self.show_live();
                let mut line = String::new();
                match std::io::stdin().read_line(&mut line) {
                    Ok(0) | Err(_) => Value::None,
                    Ok(_) => self.str_value(line.trim_end_matches(['\n', '\r'])),
                }
            }
            ("cli", "args") => Value::List(Rc::new(
                self.args
                    .clone()
                    .iter()
                    .map(|a| self.str_value(a))
                    .collect(),
            )),
            ("cli", "exit") => {
                let code = match args.first() {
                    Some(Value::Int(n, _)) => *n as i32,
                    _ => 0,
                };
                return Err(Flow::Exit(code));
            }
            ("cli", "parse") => return self.cli_parse(generics, span),
            ("cli", "stop_hold") => {
                let ms = match args.first() {
                    Some(Value::Int(n, _)) => *n as i64,
                    _ => 0,
                };
                Value::Int(
                    i128::from(kavach_runtime_core::stop_holds::hold(ms)),
                    Prim::Int,
                )
            }
            ("cli", "stop_asked") => Value::Bool(kavach_runtime_core::stop_holds::asked()),
            ("cli", "stop_done") => {
                if let Some(Value::Int(n, _)) = args.first() {
                    kavach_runtime_core::stop_holds::done(*n as i64);
                }
                Value::Unit
            }
            ("regex", "new" | "advanced") => {
                let pattern = Pattern::new(&text(0), name == "advanced")
                    .map_err(|e| Flow::Fail(Value::BuiltinError(e.into())))?;
                Value::Regex(Rc::new(pattern))
            }
            _ => return Err(self.panic(format!("std.{module} has no function `{name}`"), span)),
        })
    }
}
