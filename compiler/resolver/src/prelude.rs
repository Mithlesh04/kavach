//! Names that every module can use without `use` (spec 10.2, 8.1, 6.4).

/// Built-in types (spec 2.1, 2.2, 6, 9) and the interfaces the language
/// itself relies on (`Display` 1.11, `Drop` 6.1, `Comparable` 2.9).
pub(crate) const TYPES: &[&str] = &[
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
    "Map",
    "Set",
    "Shared",
    "Weak",
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
    "CliError",
    "Regex",
    "RegexError",
    "Self",
    "Display",
    "Drop",
    "Comparable",
];

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
