// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The standard library's types (spec 14): `Set`, and the modules
//! brought in with `use std.<module>` (more methods of built-in types are
//! in `more_methods.rs`).

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, File, ItemKind, Span, TypeExpr};
use kavach_resolver::DefId;

/// The modules of spec 14, and the capability each one needs.
pub(crate) const MODULES: &[(&str, Option<&str>)] = &[
    ("fs", Some("Fs")),
    ("time", Some("Clock")),
    ("math", None),
    ("io", Some("Console")),
    ("cli", Some("Env")),
    ("test", None),
    ("regex", None),
    ("json", None),
    ("net", Some("Net")),
    ("agents", None),
    ("clipboard", Some("Clipboard")),
    ("devices", Some("Serial")),
    ("crypto", None),
    ("base64", None),
    ("compress", None),
    ("window", Some("Window")),
    ("input", Some("Input")),
    ("audio", Some("Audio")),
    ("image", None),
    ("font", None),
    ("app", Some("Fs")),
    ("process", Some("Process")),
    ("system", Some("System")),
    ("matrix", None),
    ("gpu", Some("Window")),
    ("log", None),
    ("text", None),
];

pub(crate) fn number(t: &Ty) -> bool {
    matches!(t, Ty::Prim(p) if p.is_int() || matches!(p, Prim::Float | Prim::Float32 | Prim::Decimal | Prim::BigInt))
}

impl Checker<'_> {
    /// Remembers which names `use std.<module>` brought in.
    pub(crate) fn collect_uses(&mut self, file: &File) {
        for item in &file.items {
            let ItemKind::Use(u) = &item.kind else {
                continue;
            };
            let segs: Vec<&str> = u.path.segments.iter().map(|s| s.name.as_str()).collect();
            let [first, module] = segs.as_slice() else {
                continue;
            };
            if *first != "std" || !u.names.is_empty() {
                continue;
            }
            let name = u.alias.as_ref().unwrap_or(&u.path.segments[1]);
            // A module of the runtime, or one written in Kavach (`std.text`).
            let in_kavach = self
                .res
                .modules
                .iter()
                .any(|m| m.name == format!("std.{module}"));
            if !in_kavach && !MODULES.iter().any(|(m, _)| m == module) {
                self.error(
                    u.path.span,
                    format!("`std.{module}` is not a standard library module (spec 14)"),
                );
                continue;
            }
            if self.matrix_gone(module, u.path.span) {
                continue;
            }
            if let Some(&def) = self.decl.get(&name.span.start) {
                self.std_modules.insert(def, module.to_string());
            }
        }
    }

    /// `Set.of(a, b, …)`.
    pub(crate) fn set_of(&mut self, args: &[Arg]) -> Ty {
        let item = self.fresh();
        for a in args {
            self.check_expect(&a.value, &item);
        }
        Ty::Set(Box::new(item))
    }

    /// `module.name` for a constant, such as `math.pi`.
    pub(crate) fn std_constant(&mut self, module_def: DefId, name: &str) -> Option<Ty> {
        match (self.std_modules.get(&module_def)?.as_str(), name) {
            ("math", "pi" | "e") => Some(Ty::Prim(Prim::Float)),
            _ => None,
        }
    }

    /// A call to a standard library function: `fs.read_text(path)`.
    pub(crate) fn std_call(
        &mut self,
        module_def: DefId,
        name: &str,
        generics: &[TypeExpr],
        args: &[Arg],
        span: Span,
    ) -> Option<Ty> {
        let module = self.std_modules.get(&module_def)?.clone();
        // `gpu.pipeline(v, f)`: typed by its gpu fns (spec 15.3).
        if module == "gpu" && name == "pipeline" {
            return Some(self.pipeline_call(args, span));
        }
        if module == "ai" {
            return Some(self.ai_call(name, generics, args, span));
        }
        if self.needs_a_block(&module, name, span) {
            return Some(Ty::Error);
        }
        // Files, the network and sleeping wait (spec 6.2).
        if matches!(module.as_str(), "fs" | "net")
            || (module == "time" && (name.starts_with("sleep") || name.starts_with("every")))
        {
            self.no_wait_in_update(span, &format!("`{module}.{name}`"));
        }
        let (int, string) = (Ty::Prim(Prim::Int), Ty::Prim(Prim::String));
        let fails =
            |t: Ty, e: &'static str| Ty::Result(Box::new(t), Box::new(Ty::Builtin(e, Vec::new())));
        let p = |n: &str, t: &Ty| (n.to_string(), t.clone(), false);
        let with_default = |n: &str, t: &Ty| (n.to_string(), t.clone(), true);
        let json = self.builtin_ty("Json");
        // File operations fail with `FsError`, from `std.fs` (14.3).
        let fs_error = self.builtin_ty("FsError");
        let fs_fails = |t: Ty| Ty::Result(Box::new(t), Box::new(fs_error.clone()));
        // Any value that can be encoded as JSON (checked after the call).
        let any = self.fresh();
        // `math.sqrt` and friends take a `Float` or a `Float32` and give
        // the same (spec 14.5): a float literal's variable, settled by the
        // argument.
        let real = self.inf.fresh(crate::infer::VarKind::FloatLiteral);
        let (params, ret) = match (module.as_str(), name) {
            ("fs", "read_text") => (vec![p("path", &string)], fs_fails(string.clone())),
            ("fs", "write_text" | "append_text") => (
                vec![p("path", &string), p("text", &string)],
                fs_fails(Ty::Unit),
            ),
            ("fs", "exists") => (vec![p("path", &string)], Ty::Prim(Prim::Bool)),
            ("fs", "read_bytes" | "map") => {
                (vec![p("path", &string)], fs_fails(Ty::Prim(Prim::Bytes)))
            }
            // A WebSocket handshake's pieces, for `std/http/websocket.kav`.
            // `http.file`'s file, sent by the system (spec 14.11): for the
            // standard library, whose `http.file` was allowed to read it.
            ("net", "send_file") => {
                let module = &self.res.modules[self.res.module_at(span.start)].name;
                if !module.starts_with("std.") {
                    self.error(
                        span,
                        "this is for the standard library: use `http.file` (spec 14.11)",
                    );
                }
                let conn = Ty::Builtin("Connection", Vec::new());
                let fails = Ty::Result(
                    Box::new(Ty::Prim(Prim::Bool)),
                    Box::new(self.builtin_ty("NetError")),
                );
                (
                    vec![p("conn", &conn), p("path", &string), p("length", &int)],
                    fails,
                )
            }
            ("net", "websocket_accept" | "websocket_key" | "websocket_mask") => {
                let module = &self.res.modules[self.res.module_at(span.start)].name;
                if !module.starts_with("std.") {
                    self.error(
                        span,
                        "this is for the standard library: use `http.websocket` (spec 14.11)",
                    );
                }
                match name {
                    "websocket_accept" => (vec![p("key", &string)], string.clone()),
                    "websocket_key" => (vec![], string.clone()),
                    _ => (vec![], Ty::Prim(Prim::Bytes)),
                }
            }
            (
                "net",
                "listen" | "listen_tls" | "connect" | "connect_tls" | "connect_quic" | "bind",
            ) => {
                let made = match name {
                    "listen" | "listen_tls" => "Listener",
                    "bind" => "UdpSocket",
                    _ => "Connection",
                };
                let fails = Ty::Result(
                    Box::new(Ty::Builtin(made, Vec::new())),
                    Box::new(self.builtin_ty("NetError")),
                );
                let mut params = vec![p("address", &string)];
                let list = Ty::List(Box::new(string.clone()));
                match name {
                    "connect_tls" => params.extend([
                        with_default("protocols", &list),
                        with_default("roots", &string),
                        with_default("cert", &string),
                        with_default("key", &string),
                    ]),
                    "listen_tls" => params.extend([
                        p("cert", &string),
                        p("key", &string),
                        with_default("protocols", &list),
                        with_default("clients", &string),
                    ]),
                    "connect_quic" => params.push(p("protocols", &list)),
                    _ => {}
                }
                (params, fails)
            }
            ("fs", "lines") => (
                vec![p("path", &string)],
                fs_fails(Ty::Builtin("Lines", Vec::new())),
            ),
            ("fs", "chunks") => (
                vec![p("path", &string), p("size", &int)],
                fs_fails(Ty::Builtin("Chunks", Vec::new())),
            ),
            ("fs", "write_bytes" | "write_durable") => (
                vec![p("path", &string), p("data", &Ty::Prim(Prim::Bytes))],
                fs_fails(Ty::Unit),
            ),
            ("fs", "list") => (
                vec![p("folder", &string)],
                fs_fails(Ty::List(Box::new(string.clone()))),
            ),
            ("fs", "remove" | "create_dir" | "remove_all") => {
                (vec![p("path", &string)], fs_fails(Ty::Unit))
            }
            // A file's details as numbers, for `fs.info` in `std/fs.kav`.
            ("fs", "stat") => {
                let module = &self.res.modules[self.res.module_at(span.start)].name;
                if !module.starts_with("std.") {
                    self.error(
                        span,
                        "`fs.stat` is for `std.fs`: use `fs.info(path)` (spec 14.3)",
                    );
                }
                (
                    vec![p("path", &string)],
                    fs_fails(Ty::List(Box::new(int.clone()))),
                )
            }
            ("fs", "set_read_only") => (
                vec![p("path", &string), p("read_only", &Ty::Prim(Prim::Bool))],
                fs_fails(Ty::Unit),
            ),
            ("fs", "set_permissions") => (
                vec![p("path", &string), p("mode", &int)],
                fs_fails(Ty::Unit),
            ),
            ("fs", "rename" | "copy") => (
                vec![p("from", &string), p("to", &string)],
                fs_fails(Ty::Unit),
            ),
            ("fs", "symlink") => (
                vec![p("target", &string), p("link", &string)],
                fs_fails(Ty::Unit),
            ),
            ("time", "now_ms" | "monotonic_ms" | "monotonic_ns") => (vec![], int.clone()),
            ("time", "sleep_ns") => (vec![p("ns", &int)], Ty::Unit),
            ("time", "sleep_ms" | "freeze") => (vec![p("ms", &int)], Ty::Unit),
            ("agents", "call_key") => (vec![], Ty::Optional(Box::new(string.clone()))),
            // Approvals answered in code (spec 14.16), behind `Approvals`:
            // `std/agents.kav` gives them their types.
            ("agents", "request_fields") => (vec![], Ty::List(Box::new(string.clone()))),
            ("agents", "answer_request") => {
                let e = self.builtin_ty("ApprovalError");
                (
                    vec![p("fields", &Ty::List(Box::new(string.clone())))],
                    Ty::Result(Box::new(Ty::Unit), Box::new(e)),
                )
            }
            // Serial ports (spec 14.18), behind `Serial`, by the port's number.
            ("devices", _) => self.devices_signature(name)?,
            // std.crypto (spec 14.19): no capability.
            ("crypto", _) => self.crypto_signature(name)?,
            // std.text's normal forms (spec 14.9); the rest is Kavach.
            ("text", "nfc" | "nfd" | "nfkc" | "nfkd") => (vec![p("s", &string)], string.clone()),
            // std.base64 (spec 14.20): no capability.
            ("base64", _) => self.base64_signature(name)?,
            // std.compress (spec 14.21): no capability.
            ("compress", _) => self.compress_signature(name)?,
            // Windows (spec 14.22), behind `Window`, and `Input`.
            ("window", _) => self.window_signature(name)?,
            // Gamepads (spec 14.26), behind `Input`.
            ("input", _) => self.input_signature(name)?,
            // Sound (spec 14.27), behind `Audio`.
            ("audio", _) => self.audio_signature(name)?,
            // Pictures (spec 14.29): data, no capability.
            ("image", _) => self.image_signature(name)?,
            // Fonts (spec 14.31): data, no capability.
            ("font", _) => self.font_signature(name)?,
            // Folders, crash reports and dialogs (spec 14.30).
            ("app", _) => self.app_signature(name)?,
            ("process", _) => self.process_signature(name)?,
            // What machine this is (spec 14.25): the basics free.
            ("system", _) => self.system_signature(name)?,
            // `std.tensor`'s product (spec 14.33), for the standard library.
            ("matrix", _) => self.matrix_signature(name, generics, span)?,
            ("gpu", _) => self.draw_signature(name)?,
            ("log", _) => self.log_signature(name)?,
            ("fs", n) if n.starts_with("writer_") || n.starts_with("watch_") => {
                self.writer_signature(name)?
            }
            ("clipboard", _) => self.clipboard_signature(name)?,
            ("time", "zone_offset") => (
                vec![p("name", &string), p("ms", &int)],
                Ty::Optional(Box::new(int.clone())),
            ),
            ("time", "sleep") => (vec![p("d", &self.builtin_ty("Duration"))], Ty::Unit),
            // `f` runs after `d`, in a task of the caller's (spec 14.4).
            ("time", "after") => {
                let t = self.fresh();
                let f = Ty::Fn(Vec::new(), Box::new(t.clone()), None);
                (
                    vec![p("d", &self.builtin_ty("Duration")), p("f", &f)],
                    Ty::Builtin("Task", vec![t]),
                )
            }
            ("time", "every") => (
                vec![p("d", &self.builtin_ty("Duration"))],
                Ty::Builtin("Ticker", Vec::new()),
            ),
            ("math", "sqrt" | "floor" | "ceil" | "round")
            | ("math", "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "exp" | "ln")
            | ("math", "log10" | "cbrt" | "erfc" | "lgamma") => (vec![p("x", &real)], real.clone()),
            ("math", "atan2") => (vec![p("y", &real), p("x", &real)], real.clone()),
            ("math", "pow" | "hypot" | "remainder") => {
                (vec![p("x", &real), p("y", &real)], real.clone())
            }
            ("io", "read_line") => (vec![], Ty::Optional(Box::new(string.clone()))),
            ("io", "read_hidden") => (vec![], Ty::Optional(Box::new(string.clone()))),
            ("io", "is_terminal") => (vec![], Ty::Prim(Prim::Bool)),
            ("io", "colors") => (vec![], Ty::Prim(Prim::Bool)),
            ("io", "write") => (vec![p("text", &string)], Ty::Unit),
            ("cli", "args") => (vec![], Ty::List(Box::new(string.clone()))),
            ("cli", "exit") => (vec![p("code", &int)], Ty::Never),
            // Holds on the program's stop, for `cli.on_stop` in
            // `std/cli.kav` (decision 0088).
            ("cli", "stop_hold" | "stop_asked" | "stop_done") => {
                let module = &self.res.modules[self.res.module_at(span.start)].name;
                if !module.starts_with("std.") {
                    self.error(
                        span,
                        "this is for the standard library: use `cli.on_stop(grace)` (spec 14.6)",
                    );
                }
                match name {
                    "stop_hold" => (vec![p("grace_ms", &int)], int.clone()),
                    "stop_asked" => (vec![], Ty::Prim(Prim::Bool)),
                    _ => (vec![p("id", &int)], Ty::Unit),
                }
            }
            ("time", "every_ms") => (vec![p("ms", &int)], Ty::Builtin("Ticker", Vec::new())),
            ("regex", "new" | "advanced") => (
                vec![p("pattern", &string)],
                fails(Ty::Builtin("Regex", Vec::new()), "RegexError"),
            ),
            ("cli", "parse") => return Some(self.cli_parse(generics, args, span)),
            ("json", "parse") => (
                vec![
                    p("text", &string),
                    with_default("max_depth", &int),
                    with_default("max_bytes", &int),
                    with_default("max_fields", &int),
                    with_default("comments", &Ty::Prim(crate::Prim::Bool)),
                ],
                Ty::Result(Box::new(json), Box::new(self.builtin_ty("JsonError"))),
            ),
            ("json", "stringify") => (
                vec![
                    p("value", &any),
                    with_default("indent", &int),
                    with_default("big_as_text", &Ty::Prim(crate::Prim::Bool)),
                ],
                string.clone(),
            ),
            ("json", "of") => (
                vec![
                    p("value", &any),
                    with_default("big_as_text", &Ty::Prim(crate::Prim::Bool)),
                ],
                json,
            ),
            ("json", "schema") => {
                let Some(t) = generics.first().map(|g| self.lower(g, None)) else {
                    self.error(span, "`json.schema` needs a type: `json.schema<Order>()`");
                    return Some(Ty::Error);
                };
                if let Some(why) = self.no_schema(&t) {
                    self.error(span, why);
                }
                (Vec::new(), json)
            }
            ("json", "describe") => return Some(self.describe_call(args, span)),
            ("json", "decode_texts" | "encode_texts") => {
                self.json_texts_signature(name, generics, span)?
            }
            ("json", "decode" | "decode_value") => {
                let Some(t) = generics.first().map(|g| self.lower(g, None)) else {
                    self.error(
                        span,
                        format!("`json.{name}` needs the type to read: `json.{name}<Order>(…)`"),
                    );
                    return Some(Ty::Error);
                };
                if let Some(why) = self.not_decodable(&t) {
                    self.error(span, why);
                }
                self.check_decode_access(&t, span, &format!("`json.{name}`"));
                self.json_reads.push((self.current_fn, t.clone(), span));
                let unknown = self.builtin_ty("JsonUnknown");
                let params = if name == "decode" {
                    vec![
                        p("text", &string),
                        with_default("max_depth", &int),
                        with_default("max_bytes", &int),
                        with_default("max_fields", &int),
                        with_default("unknown", &unknown),
                    ]
                } else {
                    vec![p("value", &json), with_default("unknown", &unknown)]
                };
                let error = self.builtin_ty("JsonError");
                (params, Ty::Result(Box::new(t), Box::new(error)))
            }
            _ => {
                // Renamed: `ask` is the AI's call (decision 0053).
                let renamed = match (module.as_str(), name) {
                    ("term", "ask" | "ask_int" | "ask_float" | "ask_secret") => {
                        Some(name.replacen("ask", "prompt", 1))
                    }
                    _ => None,
                };
                // Now list methods (decision 0135).
                let gone = crate::table_methods::now_a_method(&module, name);
                let message = match renamed {
                    Some(new) => format!(
                        "`term.{name}` is now `term.{new}`: `ask` is the AI's call (decision 0053)"
                    ),
                    None => gone.unwrap_or_else(|| {
                        format!("`std.{module}` has no function `{name}` (spec 14)")
                    }),
                };
                self.error(span, message);
                return Some(Ty::Error);
            }
        };
        self.std_capability(&module, name, span);
        // Keys and passwords are often secrets: std.crypto takes them, and
        // so does a TLS private `key` (8.7).
        let secret: Vec<bool> = params
            .iter()
            .map(|(n, ..)| {
                module == "crypto"
                    || (module == "net" && n == "key")
                    // A new user's password (spec 14.25).
                    || (module == "system" && n == "text")
            })
            .collect();
        self.args_with(
            args,
            &params,
            &secret,
            span,
            &format!("`{module}.{name}`"),
            "argument",
        );
        if module == "math" && self.inf.resolve(&real) == Ty::Prim(Prim::Decimal) {
            self.error(
                span,
                format!("`math.{name}` takes a `Float` or a `Float32`, not a `Decimal`: convert with `Float(x)`"),
            );
        }
        Some(ret)
    }

    /// A type that every program has from a standard library module
    /// written in Kavach, such as `Json` (spec 2.12).
    pub(crate) fn builtin_ty(&self, name: &str) -> Ty {
        match self.res.std_type(name) {
            Some(def) => Ty::Adt(def, Vec::new()),
            None => Ty::Error,
        }
    }
}
