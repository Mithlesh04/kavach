// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
    /// `Int<bits: n>` or `UInt<bits: n>` (spec 2.1) of a width other than
    /// 8, 16, 32 and 64, which are `Int8` … `UInt64`: the width, and
    /// whether it has a sign. Kept in an `Int`'s 64 bits.
    Bits(u16, bool),
    /// `Int<range: lo..=hi>` (spec 2.1): a whole number always in the
    /// range. `Int<range: 1..10>` is `Int<range: 1..=9>`.
    Ranged(i64, i64),
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
        match self {
            Prim::Bits(..) | Prim::Ranged(..) => crate::int_numbers::name(self),
            _ => PRIMS
                .iter()
                .find(|(_, p)| *p == self)
                .map(|(n, _)| *n)
                .unwrap_or("?"),
        }
    }

    pub fn is_int(self) -> bool {
        use Prim::*;
        matches!(
            self,
            Int | Int8 | Int16 | Int32 | UInt8 | UInt16 | UInt32 | UInt64 | Bits(..) | Ranged(..)
        )
    }

    /// The smallest and largest value of a whole-number type; `None` for
    /// the others, and for `BigInt`, which has no limits.
    pub fn int_range(self) -> Option<(i128, i128)> {
        Some(match self {
            Prim::Int => (i64::MIN.into(), i64::MAX.into()),
            Prim::Int8 => (i8::MIN.into(), i8::MAX.into()),
            Prim::Int16 => (i16::MIN.into(), i16::MAX.into()),
            Prim::Int32 => (i32::MIN.into(), i32::MAX.into()),
            Prim::UInt8 => (0, u8::MAX.into()),
            Prim::UInt16 => (0, u16::MAX.into()),
            Prim::UInt32 => (0, u32::MAX.into()),
            Prim::UInt64 => (0, u64::MAX.into()),
            Prim::Bits(n, true) => (-(1i128 << (n - 1)), (1i128 << (n - 1)) - 1),
            Prim::Bits(n, false) => (0, (1i128 << n) - 1),
            Prim::Ranged(lo, hi) => (lo.into(), hi.into()),
            _ => return None,
        })
    }

    /// An `Int<bits: n>` or `Int<range: …>` other than `Int8` …
    /// `UInt64`: kept in an `Int`, with its own limits checked.
    pub fn is_narrow(self) -> bool {
        matches!(self, Prim::Bits(..) | Prim::Ranged(..))
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
    /// Parameters, result (a `Result` for one that can fail), and the
    /// most its functions may use (`uses`, spec 3.4): `None` sets no limit.
    Fn(Vec<Ty>, Box<Ty>, Option<Vec<String>>),
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
    /// A number in a type (spec 2.9): `3` in `Grid<Float, width: 3>`.
    Num(i64),
    /// A list, dict, set, ring or `Bytes` with a number in its type
    /// (spec 2.2, 2.13): `[Int]<max: 2>`, `Bytes<len: 32>`. The number is
    /// a `Num`, a number parameter's `Param`, or a `Var`. `Infer::shallow`
    /// sees through it, so code that doesn't care about the number sees
    /// the plain type.
    Sized(Box<Ty>, SizeKind, Box<Ty>),
    /// `Frozen<T>` and `Frozen<T, K>` (spec 2.20): a value whose fields
    /// and items never change; `true` when frozen all the way down, and
    /// the key's type when sealed with one. `Infer::shallow` sees through
    /// it, so reading works as on `T`.
    Frozen(Box<Ty>, bool, Option<Box<Ty>>),
}

/// What a collection's number says (spec 2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeKind {
    /// `max`: at most that many entries.
    Max,
    /// `len`: exactly that many.
    Len,
}

impl SizeKind {
    pub fn name(self) -> &'static str {
        match self {
            SizeKind::Max => "max",
            SizeKind::Len => "len",
        }
    }
}

/// Names of built-in types that are not scalars, as `&'static str`.
pub(crate) fn builtin_name(name: &str) -> &'static str {
    const NAMES: &[&str] = &[
        "Shared",
        "Weak",
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
        "Ticker",
        "Lines",
        "Chunks",
        "Display",
        "Drop",
        "Pipeline",
        "Add",
        "Sub",
        "Mul",
        "Div",
        "Rem",
        "Neg",
        "MatMul",
        "Pow",
        "Comparable",
        "Compare",
        "Number",
        "Floating",
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
    NAMES
        .iter()
        .copied()
        .find(|n| *n == name)
        .unwrap_or("Unknown")
}
