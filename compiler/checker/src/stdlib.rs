//! The standard library's types (spec 14): `Set`, and the modules
//! brought in with `use std.<module>` (more methods of built-in types are
//! in `more_methods.rs`).

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, File, ItemKind, Span, TypeExpr};
use kavach_resolver::DefId;

/// The modules of spec 14, and the capability each one needs.
const MODULES: &[(&str, Option<&str>)] = &[
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
    ("process", Some("Process")),
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
            ("fs", "read_bytes") => (vec![p("path", &string)], fs_fails(Ty::Prim(Prim::Bytes))),
            // A WebSocket handshake's pieces, for `std/http/websocket.kav`.
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
            ("time", "now_ms" | "monotonic_ms") => (vec![], int.clone()),
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
            ("process", _) => self.process_signature(name)?,
            ("gpu", _) => self.draw_signature(name)?,
            ("log", _) => self.log_signature(name)?,
            ("fs", n) if n.starts_with("writer_") => self.writer_signature(name)?,
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
            ("math", "sqrt" | "floor" | "ceil" | "round") => (vec![p("x", &real)], real.clone()),
            ("math", "pow") => (vec![p("x", &real), p("y", &real)], real.clone()),
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
                vec![p("value", &any), with_default("indent", &int)],
                string.clone(),
            ),
            ("json", "of") => (vec![p("value", &any)], json),
            ("json", "schema") => {
                let Some(t) = generics.first().map(|g| self.lower(g, None)) else {
                    self.error(span, "`json.schema` needs a type: `json.schema<Order>()`");
                    return Some(Ty::Error);
                };
                let text_keys = |k: &Ty| *k == Ty::Prim(crate::Prim::String);
                if matches!(&t, Ty::Map(k, _) if !text_keys(k))
                    || matches!(t, Ty::Tuple(_) | Ty::Fn(..))
                {
                    self.error(
                        span,
                        "maps without `String` keys, tuples and functions have no JSON Schema (spec 2.12)",
                    );
                }
                (Vec::new(), json)
            }
            ("json", "describe") => return Some(self.describe_call(args, span)),
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
                let message = match renamed {
                    Some(new) => format!(
                        "`term.{name}` is now `term.{new}`: `ask` is the AI's call (decision 0053)"
                    ),
                    None => format!("`std.{module}` has no function `{name}` (spec 14)"),
                };
                self.error(span, message);
                return Some(Ty::Error);
            }
        };
        // Reading the clock needs no capability (spec 8.1); a contract still
        // may not read it, as its answer would change from run to run.
        let reads_clock = module == "time" && matches!(name, "now_ms" | "monotonic_ms");
        // Nor does reading a zone's rules, or freezing a test's clock (spec
        // 14.4), which only a `test` block may do.
        let reads_zone = module == "time" && matches!(name, "zone_offset" | "freeze");
        let in_test = self.ctx.as_ref().is_some_and(|c| c.name == "test");
        if module == "time" && name == "freeze" && !in_test {
            self.error(
                span,
                "`time.freeze` is only for tests: call it directly in a `test` block (spec 14.4)",
            );
        }
        if reads_clock && self.in_contract {
            self.error(span, format!("a contract can't call `time.{name}`: it reads the clock, and contracts must give the same answer every time (spec 7.5)"));
        }
        // `std.agents` needs nothing, except to answer approvals (14.16).
        let answers = module == "agents" && matches!(name, "request_fields" | "answer_request");
        // A hold on the program's stop needs nothing (decision 0088), nor
        // does working out a WebSocket handshake (0092).
        let holds = (module == "cli" && name.starts_with("stop_"))
            || (module == "net" && name.starts_with("websocket_"))
            || (module == "process" && name == "peak_bytes_id");
        if let Some(cap) = (MODULES.iter())
            .find(|(m, _)| *m == module && !reads_clock && !reads_zone && !holds)
            .and_then(|(_, c)| if answers { Some("Approvals") } else { *c })
        {
            // Reading and changing files are granted apart (spec 8.1).
            let cap = match (module.as_str(), name) {
                // Listening to the keyboard and mouse (spec 14.22).
                ("window", "listen_id" | "lock_cursor_id") => "Input",
                (
                    "fs",
                    "read_text" | "read_bytes" | "lines" | "chunks" | "exists" | "list" | "stat",
                ) => "Fs.read",
                ("fs", _) => "Fs.write",
                _ => cap,
            };
            self.note_cap(cap, span);
            if self.in_contract {
                self.error(span, format!("a contract can't call `{module}.{name}`: it uses {cap}, and contracts must be pure (spec 7.5)"));
            }
        }
        // Keys and passwords are often secrets: std.crypto takes them, and
        // so does a TLS private `key` (8.7).
        let secret: Vec<bool> = params
            .iter()
            .map(|(n, ..)| module == "crypto" || (module == "net" && n == "key"))
            .collect();
        self.args_with(
            args,
            &params,
            &secret,
            span,
            &format!("`{module}.{name}`"),
            "argument",
        );
        if module == "json"
            && matches!(name, "of" | "stringify")
            && let Some(why) = self.not_encodable(&any)
        {
            self.error(span, why);
        }
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

    /// `Float.from_bits(n)` (spec 14.2): the `Float` with IEEE 754 bits `n`.
    pub(crate) fn float_from_bits(&mut self, args: &[Arg], span: Span) -> Ty {
        let params = [("bits".to_string(), Ty::Prim(Prim::UInt64), false)];
        self.args(args, &params, span, "`from_bits`", "argument");
        Ty::Prim(Prim::Float)
    }
}
