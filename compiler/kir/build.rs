//! Makes `RtOp`, the built-in operations, and `RtOp::ALL`, every op by
//! the code native code passes to the runtime, from the op lists in
//! `src/rt_op/ops/`: one file per family, each op a line `Name,` after its
//! doc comment. An op's code is its place in the families below, in order.

use std::fmt::Write;

/// The op lists, in the order of their codes.
const FAMILIES: [&str; 8] = [
    "text",
    "collections",
    "json",
    "system",
    "net",
    "tasks",
    "agents",
    "gpu",
];

fn main() {
    let mut body = String::new();
    let mut names = Vec::new();
    for family in FAMILIES {
        let path = format!("src/rt_op/ops/{family}.ops");
        println!("cargo:rerun-if-changed={path}");
        let text = std::fs::read_to_string(&path).expect("an op list");
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with("//") && !t.starts_with("///") {
                continue;
            }
            if !t.is_empty() && !t.starts_with("///") {
                names.push(t.trim_end_matches(',').to_string());
            }
            let _ = writeln!(body, "    {line}");
        }
    }
    let all: String = names
        .iter()
        .map(|n| format!("        RtOp::{n},\n"))
        .collect();
    let out = format!(
        "/// Built-in methods and standard library functions. Each has up to three
/// operands; the result type says what it gives back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RtOp {{
{body}}}

impl RtOp {{
    /// Every op, by the code native code passes to the runtime (`kv_rt`).
    pub const ALL: [RtOp; {}] = [
{all}    ];
}}
",
        names.len()
    );
    let dir = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    std::fs::write(format!("{dir}/rt_op.rs"), out).expect("written");
}
