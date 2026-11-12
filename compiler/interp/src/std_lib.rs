//! The standard library (spec 14): more methods of built-in types, sets,
//! and the modules brought in with `use std.<module>`.

use crate::interp::Interp;
use crate::map::MapValue;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{ItemKind, Span, TypeExpr};
use kavach_checker::Prim;
use kavach_runtime_core::regex::Pattern;
use std::io::IsTerminal;
use std::rc::Rc;

fn int<'v>(n: usize) -> Value<'v> {
    Value::Int(n as i128, Prim::Int)
}

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
        let error = match e.kind() {
            std::io::ErrorKind::NotFound => self.builtin_variant("FsError", "NotFound", vec![at]),
            std::io::ErrorKind::PermissionDenied => {
                self.builtin_variant("FsError", "Denied", vec![at])
            }
            _ => {
                let reason = ("reason", self.str_value(&e.to_string()));
                self.builtin_variant("FsError", "Failed", vec![at, reason])
            }
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
        if module == "crypto" {
            return self.crypto_call(name, &args, span);
        }
        if module == "devices" {
            return self.devices_call(name, &args);
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
            (true, "rename" | "copy") if !inside(&text(1)) => Some(text(1)),
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
        if module == "fs" && self.fake.is_some() {
            return self.fake_fs(name, &args);
        }
        if module == "fs"
            && matches!(
                name,
                "stat" | "set_read_only" | "set_permissions" | "rename" | "copy" | "remove_all"
            )
        {
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
            ("math", "sqrt" | "floor" | "ceil" | "round" | "pow")
                if matches!(args.first(), Some(Value::Float32(_))) =>
            {
                let wide = |i: usize| match args.get(i) {
                    Some(Value::Float32(f)) => f64::from(*f),
                    _ => 0.0,
                };
                let x = wide(0);
                Value::Float32(match name {
                    "sqrt" => x.sqrt(),
                    "floor" => x.floor(),
                    "ceil" => x.ceil(),
                    "round" => x.round(),
                    _ => x.powf(wide(1)),
                } as f32)
            }
            ("math", "sqrt") => Value::Float(float(0).sqrt()),
            ("math", "pow") => Value::Float(float(0).powf(float(1))),
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

    /// More methods of built-in types (spec 14.2), or `None` if `name`
    /// isn't one of them.
    pub(crate) fn more_method(
        &mut self,
        recv: &Value<'a>,
        name: &str,
        args: &[Value<'a>],
        span: Span,
    ) -> Result<Option<Value<'a>>, Flow<'a>> {
        if let Some(v) = self.number_method(recv, name, args, span)? {
            return Ok(Some(v));
        }
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Unit);
        Ok(Some(match (recv, name) {
            (Value::Str(s), _) => return Ok(self.more_text(s, name, args)),
            (Value::Regex(r), _) => return self.regex_method(r, name, args, span).map(Some),
            (Value::List(items), _) => return self.more_list(items, name, args, span),
            (Value::Set(set), _) => match name {
                "contains" => Value::Bool(set.get(&arg(0)).is_some()),
                "len" => int(set.len()),
                "is_empty" => Value::Bool(set.is_empty()),
                "to_list" => Value::List(Rc::new(
                    set.entries.iter().map(|(k, _)| k.clone()).collect(),
                )),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        }))
    }

    fn more_text(&self, s: &str, name: &str, args: &[Value<'a>]) -> Option<Value<'a>> {
        let text = |i: usize| match args.get(i) {
            Some(Value::Str(t)) => t.to_string(),
            _ => String::new(),
        };
        let number = |i: usize| match args.get(i) {
            Some(Value::Int(n, _)) => (*n).max(0) as usize,
            _ => 0,
        };
        Some(match name {
            "replace" => self.str_value(&s.replace(&text(0), &text(1))),
            "lines" => Value::List(Rc::new(s.lines().map(|l| self.str_value(l)).collect())),
            "chars" => Value::List(Rc::new(s.chars().map(Value::Char).collect())),
            "index_of" => match s.find(&text(0)) {
                Some(byte) => int(s[..byte].chars().count()),
                None => Value::None,
            },
            "slice" => self.str_value(
                &s.chars()
                    .skip(number(0))
                    .take(number(1).saturating_sub(number(0)))
                    .collect::<String>(),
            ),
            _ => return None,
        })
    }

    fn more_list(
        &mut self,
        items: &Rc<Vec<Value<'a>>>,
        name: &str,
        args: &[Value<'a>],
        span: Span,
    ) -> Result<Option<Value<'a>>, Flow<'a>> {
        let order = crate::ops::list_order;
        let arg = args.first().cloned().unwrap_or(Value::Unit);
        let list = |xs: Vec<Value<'a>>| Value::List(Rc::new(xs));
        let count = |v: &Value<'a>| match v {
            Value::Int(n, _) => (*n).max(0) as usize,
            _ => 0,
        };
        Ok(Some(match name {
            "join" => {
                let parts: Vec<String> = items.iter().map(|v| v.display()).collect();
                self.str_value(&parts.join(&arg.display()))
            }
            "sort" => {
                let mut xs = items.to_vec();
                xs.sort_by(order);
                list(xs)
            }
            "sort_by" => {
                let mut keyed = Vec::new();
                for x in items.iter() {
                    keyed.push((
                        self.call_value(arg.clone(), vec![x.clone()], span)?,
                        x.clone(),
                    ));
                }
                keyed.sort_by(|a, b| order(&a.0, &b.0));
                list(keyed.into_iter().map(|(_, x)| x).collect())
            }
            "reverse" => list(items.iter().rev().cloned().collect()),
            "take" => list(items.iter().take(count(&arg)).cloned().collect()),
            "skip" => list(items.iter().skip(count(&arg)).cloned().collect()),
            "find" | "count" => {
                let mut found = Vec::new();
                for x in items.iter() {
                    if self.call_value(arg.clone(), vec![x.clone()], span)? == Value::Bool(true) {
                        found.push(x.clone());
                    }
                }
                match name {
                    "find" => found.into_iter().next().unwrap_or(Value::None),
                    _ => int(found.len()),
                }
            }
            "index_of" => items
                .iter()
                .position(|x| *x == arg)
                .map_or(Value::None, int),
            "min" => items
                .iter()
                .min_by(|a, b| order(a, b))
                .cloned()
                .unwrap_or(Value::None),
            "max" => items
                .iter()
                .max_by(|a, b| order(a, b))
                .cloned()
                .unwrap_or(Value::None),
            _ => return Ok(None),
        }))
    }

    /// A method of a compiled regular expression (spec 14.8).
    fn regex_method(&self, r: &Pattern, name: &str, args: &[Value<'a>], span: Span) -> Eval<'a> {
        let text = |i: usize| match args.get(i) {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let strs =
            |xs: Vec<String>| Value::List(Rc::new(xs.iter().map(|x| self.str_value(x)).collect()));
        let gave_up = |_| {
            self.panic(
                "regex gave up: this advanced pattern needed too much work on this text (spec 14.8)",
                span,
            )
        };
        let t = text(0);
        Ok(match name {
            "is_match" => Value::Bool(r.is_match(&t).map_err(gave_up)?),
            "find" => r
                .find(&t)
                .map_err(gave_up)?
                .map_or(Value::None, |m| self.str_value(&m)),
            "find_all" => strs(r.find_all(&t).map_err(gave_up)?),
            "split" => strs(r.split(&t).map_err(gave_up)?),
            "captures" => match r.captures(&t).map_err(gave_up)? {
                Some(groups) => Value::List(Rc::new(
                    groups
                        .iter()
                        .map(|g| g.as_ref().map_or(Value::None, |s| self.str_value(s)))
                        .collect(),
                )),
                None => Value::None,
            },
            "named_captures" => match r.named_captures(&t).map_err(gave_up)? {
                Some(pairs) => {
                    let mut map = MapValue::default();
                    for (k, v) in pairs {
                        map.insert(self.str_value(&k), self.str_value(&v));
                    }
                    Value::Map(Rc::new(map))
                }
                None => Value::None,
            },
            "replace" | "replace_all" => self.str_value(
                &r.replace(&t, &text(1), name == "replace_all")
                    .map_err(gave_up)?,
            ),
            _ => return Err(self.panic(format!("a regex has no method `{name}`"), span)),
        })
    }

    /// `Set.of(a, b, …)`.
    pub(crate) fn set_of(&self, values: Vec<Value<'a>>) -> Value<'a> {
        let mut set = MapValue::default();
        for v in values {
            set.insert(v, Value::Unit);
        }
        Value::Set(Rc::new(set))
    }
}
