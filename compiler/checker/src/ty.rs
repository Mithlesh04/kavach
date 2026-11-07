//! Types as the checker sees them.

use kavach_resolver::DefId;

/// Built-in scalar types (spec 2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prim {
    Int,
    Int8,
    Int16,
    Int32,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float,
    Float32,
    Decimal,
    /// A whole number of any size (spec 2.1).
    BigInt,
    Bool,
    Char,
    String,
    Bytes,
}

const PRIMS: &[(&str, Prim)] = &[
    ("Int", Prim::Int),
    ("Int8", Prim::Int8),
    ("Int16", Prim::Int16),
    ("Int32", Prim::Int32),
    ("UInt8", Prim::UInt8),
    ("UInt16", Prim::UInt16),
    ("UInt32", Prim::UInt32),
    ("UInt64", Prim::UInt64),
    ("Float", Prim::Float),
    ("Float32", Prim::Float32),
    ("Decimal", Prim::Decimal),
    ("BigInt", Prim::BigInt),
    ("Bool", Prim::Bool),
    ("Char", Prim::Char),
    ("String", Prim::String),
    ("Bytes", Prim::Bytes),
];

impl Prim {
    pub fn from_name(name: &str) -> Option<Prim> {
        PRIMS.iter().find(|(n, _)| *n == name).map(|(_, p)| *p)
    }

    pub fn name(self) -> &'static str {
        PRIMS
            .iter()
            .find(|(_, p)| *p == self)
            .map(|(n, _)| *n)
            .unwrap_or("?")
    }

    pub fn is_int(self) -> bool {
        use Prim::*;
        matches!(
            self,
            Int | Int8 | Int16 | Int32 | UInt8 | UInt16 | UInt32 | UInt64
        )
    }

    pub fn is_numeric(self) -> bool {
        self.is_int()
            || matches!(
                self,
                Prim::Float | Prim::Float32 | Prim::Decimal | Prim::BigInt
            )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Prim(Prim),
    /// What a function without `->` returns (spec 3.1).
    Unit,
    /// The type of `return`, `fail`, `break` and `panic(…)`: they never
    /// produce a value, so they fit anywhere.
    Never,
    /// A type that could not be worked out because of an earlier error.
    /// It fits anywhere, so one mistake is reported once.
    Error,
    List(Box<Ty>),
    Map(Box<Ty>, Box<Ty>),
    Set(Box<Ty>),
    Optional(Box<Ty>),
    Tuple(Vec<Ty>),
    /// `(lo: Int, hi: Int)`: an anonymous record (spec 2.14). Fields are
    /// kept sorted by name, so equal shapes are equal types.
    Anon(Vec<(String, Ty)>),
    Range(Box<Ty>),
    Fn(Vec<Ty>, Box<Ty>),
    /// A record, union, interface, capability or agent, with type arguments.
    Adt(DefId, Vec<Ty>),
    /// Built-in generic and library types: `Shared<T>`, `Task<T>`, `Error`…
    Builtin(&'static str, Vec<Ty>),
    /// A generic parameter inside its own declaration.
    Param(DefId),
    /// The value of a call that can fail (spec 5): it must be handled with
    /// `try`, `catch` or `match` before the success value can be used.
    Result(Box<Ty>, Box<Ty>),
    /// An unknown type being inferred.
    Var(u32),
}

/// Names of built-in types that are not scalars, as `&'static str`.
pub(crate) fn builtin_name(name: &str) -> &'static str {
    const NAMES: &[&str] = &[
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
        "Ticker",
        "Lines",
        "Chunks",
        "Display",
        "Drop",
        "Comparable",
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
    NAMES
        .iter()
        .copied()
        .find(|n| *n == name)
        .unwrap_or("Unknown")
}
