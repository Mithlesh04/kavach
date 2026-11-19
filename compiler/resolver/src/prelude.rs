// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Names that every module can use without `use` (spec 10.2, 8.1, 6.4).

/// Built-in types (spec 2.1, 2.2, 6, 9) and the interfaces the language
/// itself relies on (`Display` 1.11, `Drop` 6.1, `Comparable` 2.9).
pub const TYPES: &[&str] = &[
    "Int",
    "Int8",
    "Int16",
    "Int32",
    "UInt8",
    "UInt16",
    "UInt32",
    "UInt64",
    "Float",
    "Float32",
    "Decimal",
    "BigInt",
    "Bool",
    "Char",
    "String",
    "Bytes",
    "List",
    "Dict",
    "Set",
    "Shared",
    "Weak",
    "Frozen",
    "Ring",
    "StrView",
    "TextBuilder",
    "Task",
    "Channel",
    "Sender",
    "Receiver",
    "Listener",
    "Connection",
    "UdpSocket",
    "Ptr",
    "Error",
    "ParseError",
    "InvariantError",
    "SizeError",
    "CliError",
    "LibraryError",
    "Regex",
    "RegexError",
    "Lines",
    "Chunks",
    "Self",
    "Display",
    "Drop",
    "Pipeline",
    "Add",
    "Sub",
    "Mul",
    "Div",
    "Rem",
    "MatMul",
    "Neg",
    "Pow",
    "Comparable",
    "Compare",
    "Index",
    "IndexSet",
    "Slice",
    "Number",
    "Floating",
];

/// Parts of a built-in capability that the whole doesn't cover: each is
/// written by its own name, and covers its own parts (spec 8.1, 14.25,
/// decision 0112): `System` doesn't cover `System.critical.read`.
pub const APART: &[&str] = &["System.critical"];

/// Whether capability `granted` covers `cap`: itself or one of its parts
/// (`Fs` covers `Fs.read`), but never across a part kept apart.
pub fn cap_covers(granted: &str, cap: &str) -> bool {
    let mut c = cap;
    loop {
        if c == granted {
            return true;
        }
        if APART.contains(&c) {
            return false;
        }
        match c.rsplit_once('.') {
            Some((whole, _)) => c = whole,
            None => return false,
        }
    }
}

/// Built-in capabilities (spec 8.1).
pub const CAPABILITIES: &[&str] = &[
    "Console",
    "Fs",
    "Net",
    "Env",
    "Clock",
    "Random",
    "Ai",
    "Foreign",
    "Clipboard",
    "Serial",
    "Approvals",
    "Window",
    "Input",
    "Process",
    "System",
    "Audio",
    "Microphone",
    "Gpu",
];

/// Prelude functions (spec 10.2), `reveal` (8.7), the `unsafe` memory
/// functions (6.4), and `log_write`, `test_random` and `random_bits`,
/// which only the standard library's `std.log` (14.12), `std.test` (14.7)
/// and `std.random` (14.15) may call.
pub(crate) const FUNCTIONS: &[&str] = &[
    "print",
    "debug",
    "ask",
    "panic",
    "env",
    "reveal",
    "alloc",
    "free",
    "log_write",
    "test_random",
    "random_bits",
];

/// Functions available only inside an agent's `limit` rules (spec 8.3).
pub(crate) const LIMIT_FUNCTIONS: &[&str] = &["sum", "count", "tokens"];

#[cfg(test)]
mod tests {
    use super::cap_covers;

    #[test]
    fn a_capability_covers_its_parts_but_not_the_critical_ones() {
        for (granted, cap, covers) in [
            ("Fs", "Fs.read", true),
            ("Fs.read", "Fs", false),
            ("System", "System", true),
            ("System", "System.critical.read", false),
            ("System", "System.critical", false),
            ("System.critical", "System.critical.write", true),
            ("System.critical.read", "System.critical.read", true),
            ("System.critical.read", "System.critical.write", false),
        ] {
            assert_eq!(cap_covers(granted, cap), covers, "{granted} covers {cap}");
        }
    }
}
