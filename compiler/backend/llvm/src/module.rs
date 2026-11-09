//! The parts of the LLVM module outside function bodies: declarations of
//! runtime functions and intrinsics, text constants, the runtime's
//! globals, and the C `main` that starts the program (as in decision
//! 0016).

use kavach_kir::{Program, Type, encode_shapes};
use std::collections::BTreeMap;
use std::fmt::Write;

/// The LLVM type of a KIR type (`void` for `Unit`). Heap values are
/// pointers kept as 64-bit integers, as the runtime passes them.
pub(crate) fn llvm_type(t: Type) -> &'static str {
    match t {
        Type::I8 | Type::U8 => "i8",
        Type::I16 | Type::U16 => "i16",
        Type::I32 | Type::U32 | Type::Char => "i32",
        Type::F64 => "double",
        Type::F32 => "float",
        Type::Bool => "i1",
        Type::Unit => "void",
        _ => "i64",
    }
}

/// The LLVM return type of a function returning `ret`: one that can fail
/// returns its error after its value (0 when it succeeded), as in the
/// Cranelift backend.
pub(crate) fn ret_type(ret: Type, fails: bool) -> String {
    match (fails, ret) {
        (false, t) => llvm_type(t).to_string(),
        (true, Type::Unit) => "i64".to_string(),
        (true, t) => format!("{{ {}, i64 }}", llvm_type(t)),
    }
}

#[derive(Default)]
pub(crate) struct Module {
    /// Declarations by name, written once each.
    decls: BTreeMap<String, String>,
    /// Text constants, in the order they were made.
    texts: Vec<Vec<u8>>,
    /// Whether each program function can fail.
    pub(crate) fails: Vec<bool>,
    /// The C name of each program function that is a C function (spec
    /// 10.7), `None` for the others.
    pub(crate) c_names: Vec<Option<String>>,
    /// Whether each program function is a closure's, which borrows its
    /// closure (`kavach_kir::borrows`).
    pub(crate) closures: Vec<bool>,
    /// How many loops are kept scalar (`scalar_loop`): each has its own
    /// loop metadata, `!100`, `!101`, ….
    scalar_loops: usize,
}

impl Module {
    /// Declares a function the code calls (runtime or intrinsic). Runtime
    /// functions that panic never return, which LLVM may rely on.
    pub(crate) fn declare(&mut self, name: &str, ret: &str, params: &[&str], never_returns: bool) {
        let attrs = if never_returns { " noreturn cold" } else { "" };
        self.decls
            .entry(name.to_string())
            .or_insert_with(|| format!("declare {ret} @{name}({}){attrs}", params.join(", ")));
    }

    /// A text constant: an operand giving its address as an `i64`, and its
    /// length in bytes. A zero byte follows it, so even "" has an address.
    pub(crate) fn text(&mut self, text: &str) -> (String, usize) {
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(0);
        let n = self.texts.len();
        self.texts.push(bytes);
        (format!("ptrtoint (ptr @.text{n} to i64)"), text.len())
    }

    /// The metadata for a loop LLVM must not vectorize: plain x86-64 has
    /// no vector division, and a vector `%` by a constant costs more than
    /// the scalar one it replaces (`benchmarks/everyday` stacks: 70 ms
    /// vectorized, 58 ms scalar, 2026-11-05).
    pub(crate) fn scalar_loop(&mut self) -> String {
        self.scalar_loops += 1;
        format!("!{}", 99 + self.scalar_loops)
    }

    /// The module text around the functions in `body`, with the C `main`
    /// that hands the entry function `entry` to the runtime's `kv_main`.
    pub(crate) fn finish(mut self, program: &Program, body: &str, entry: &str) -> String {
        let (path, path_len) = self.text(&program.path);
        let shapes_text = encode_shapes(&program.shapes);
        let (shapes, shapes_len) = self.text(&shapes_text);
        let fails = program.functions[program.entry as usize].fails as u8;
        self.declare(
            "kv_main",
            "i32",
            &["i64", "i8", "i64", "i64", "i64", "i64", "ptr"],
            false,
        );
        let main = format!(
            "define i32 @main(i32 %argc, ptr %argv) {{\n  %code = call i32 @kv_main(i64 ptrtoint (ptr @{entry} to i64), i8 {fails}, i64 {path}, i64 {path_len}, i64 {shapes}, i64 {shapes_len}, ptr @kv_limit_slot)\n  ret i32 %code\n}}\n"
        );
        self.surround(&format!("{body}{main}"), "thread_local(localexec)")
    }

    /// The module text around a library's functions and export shims
    /// (`library.rs`): everything `finish` writes but the C `main`. Its
    /// thread-local stack limit takes the general model: the fast one an
    /// executable uses assumes the executable's own thread-local block,
    /// and a library would write into its host's.
    pub(crate) fn finish_library(self, body: &str) -> String {
        self.surround(body, "thread_local")
    }

    fn surround(self, body: &str, tls: &str) -> String {
        let mut out = String::new();
        let triple = if cfg!(windows) {
            "x86_64-pc-windows-msvc"
        } else {
            "x86_64-unknown-linux-gnu"
        };
        let _ = writeln!(out, "target triple = \"{triple}\"\n");
        for global in ["kv_pool", "kv_live", "kv_free_at_exit"] {
            let _ = writeln!(out, "@{global} = external global i64");
        }
        // Each thread's stack limit (`func.rs`): tasks run on several
        // threads at once. The runtime sets it through `kv_limit_slot`.
        let _ = writeln!(out, "@kv_tls_limit = internal {tls} global i64 0\n");
        out.push_str("define internal ptr @kv_limit_slot() {\n  ret ptr @kv_tls_limit\n}\n");
        for (n, bytes) in self.texts.iter().enumerate() {
            let _ = writeln!(
                out,
                "@.text{n} = private unnamed_addr constant [{} x i8] c\"{}\"",
                bytes.len(),
                escape(bytes)
            );
        }
        out.push('\n');
        for decl in self.decls.values() {
            let _ = writeln!(out, "{decl}");
        }
        out.push('\n');
        out.push_str(body);
        // The stack pointer, for the stack-overflow check (`func.rs`).
        out.push_str("!0 = !{!\"rsp\"}\n");
        // Type-based alias tags (`heap.rs`): object headers and items
        // (list items, record fields) are never the same memory.
        out.push_str("!1 = !{!\"kavach\"}\n");
        out.push_str("!2 = !{!\"header\", !1, i64 0}\n!3 = !{!2, !2, i64 0}\n");
        out.push_str("!4 = !{!\"item\", !1, i64 0}\n!5 = !{!4, !4, i64 0}\n");
        if self.scalar_loops > 0 {
            out.push_str("!6 = !{!\"llvm.loop.vectorize.enable\", i1 false}\n");
            for n in 100..100 + self.scalar_loops {
                let _ = writeln!(out, "!{n} = distinct !{{!{n}, !6}}");
            }
        }
        out
    }
}

/// Bytes as the inside of an LLVM `c"…"` string.
fn escape(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        if b.is_ascii_graphic() && b != b'"' && b != b'\\' || b == b' ' {
            s.push(b as char);
        } else {
            let _ = write!(s, "\\{b:02X}");
        }
    }
    s
}
