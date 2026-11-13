//! Run-time values and how they print (spec 1.11).

use crate::map::MapValue;
use kavach_ast::{Expr, Span};
use kavach_checker::Prim;
use kavach_resolver::DefId;
use kavach_runtime_core::Decimal;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Value<'a> {
    Unit,
    /// Every integer type, with the type it has (for overflow checks).
    Int(i128, Prim),
    Float(f64),
    /// IEEE single precision: rounded to 32 bits at every step (spec 2.1).
    Float32(f32),
    Decimal(Decimal),
    /// A `Decimal` too big or too precise for the form above (decision
    /// 0042). A value that fits that form is always held in it.
    BigDecimal(Rc<kavach_runtime_core::BigDecimal>),
    /// A whole number of any size (spec 2.1).
    BigInt(Rc<kavach_runtime_core::BigInt>),
    Bool(bool),
    Char(char),
    Str(Rc<str>),
    /// Raw data (spec 2.13).
    Bytes(Rc<[u8]>),
    /// A file read as a `for` loop goes (spec 14.3).
    Reader(crate::files::Shared),
    /// A TCP listener or connection (spec 14.10).
    Socket(std::sync::Arc<kavach_runtime_net::net::Shared>),
    /// A connection to a generated test's fake server (spec 11.5).
    FakeConn(crate::fake_net::SharedConn),
    /// Shared until changed (copy-on-write, spec 6.1), so passing a list
    /// around never copies it.
    List(Rc<Vec<Value<'a>>>),
    /// Insertion-ordered (spec 2.2).
    /// Shared until changed, like lists.
    Map(Rc<MapValue<'a>>),
    /// A set: a map whose values are all `Unit` (spec 14.2).
    Set(Rc<MapValue<'a>>),
    /// A compiled regular expression (spec 14.8).
    Regex(Rc<kavach_runtime_core::regex::Pattern>),
    /// `time.every_ms(ms)`: rounds of a `for` loop (spec 14.4).
    Ticker(u64),
    /// A spawned task (spec 9.1).
    Task(std::sync::Arc<crate::tasks::TaskHandle<'a>>),
    /// An end of a channel (spec 9.4): its sender when `true`.
    /// A channel's end: `None` for the `Sender`, the receiver's share
    /// for the `Receiver` (spec 9.4).
    Channel(
        crate::channels::Channel<'a>,
        Option<Rc<crate::channels::Receiving<'a>>>,
    ),
    Tuple(Vec<Value<'a>>),
    /// The missing optional. A present optional is just its value.
    None,
    Record {
        def: DefId,
        name: Rc<str>,
        fields: Vec<Field<'a>>,
        /// For a type that implements `Drop`: the value's identity, which
        /// its copies share (`drops.rs`).
        life: Option<Rc<Life<'a>>>,
    },
    Variant {
        union: DefId,
        name: Rc<str>,
        fields: Vec<Field<'a>>,
        /// For a union that implements `Drop`: as a record's (`drops.rs`).
        /// Only held, so its `drop` runs when the last copy goes.
        #[allow(dead_code)]
        life: Option<Rc<Life<'a>>>,
    },
    Closure(Rc<Closure<'a>>),
    Func(DefId),
    /// A started agent (spec 8.8).
    Agent(Rc<crate::agents::Running<'a>>),
    Shared(Rc<RefCell<Value<'a>>>),
    /// Points to a `Shared` without keeping it alive (spec 6.2).
    Weak(std::rc::Weak<RefCell<Value<'a>>>),
    /// `start..end`, end excluded.
    Range(i128, i128, Prim),
    /// A built-in error, such as text `Int(text)` can't read.
    BuiltinError(Rc<str>),
    /// An error passed on with `context "…"` (spec 5.7).
    Context(Rc<str>, Box<Value<'a>>),
    /// The two sides of a call that can fail, inside `match` (spec 5.4).
    Ok(Box<Value<'a>>),
    Err(Box<Value<'a>>),
}

/// The identity of a value of a type that implements `Drop` (spec 6.1):
/// its copies share it, and when the last one goes, the value, with the
/// fields it has then, waits in `queue` for its `drop` to run.
#[derive(Debug)]
pub struct Life<'a> {
    pub def: DefId,
    /// The record's name, or the variant's, for a union.
    pub name: Rc<str>,
    /// Whether it is a union's value, `def` the union.
    pub variant: bool,
    pub fields: RefCell<Vec<Field<'a>>>,
    pub queue: Rc<RefCell<Vec<Value<'a>>>>,
}

impl Drop for Life<'_> {
    fn drop(&mut self) {
        let (name, fields) = (Rc::clone(&self.name), self.fields.take());
        let value = match self.variant {
            true => Value::Variant {
                union: self.def,
                name,
                fields,
                life: None,
            },
            false => Value::Record {
                def: self.def,
                name,
                fields,
                life: None,
            },
        };
        self.queue.borrow_mut().push(value);
    }
}

#[derive(Clone, Debug)]
pub struct Field<'a> {
    pub name: Rc<str>,
    pub value: Value<'a>,
    pub secret: bool,
}

#[derive(Debug)]
pub struct Closure<'a> {
    pub params: Vec<Option<DefId>>,
    pub body: &'a Expr,
    /// A dot lambda (`.status == Paid`): its single parameter is `it`.
    pub dot: bool,
    pub captured: HashMap<DefId, Value<'a>>,
    /// The type arguments of the generic function the lambda was made in:
    /// its body may name them (`json.schema<T>()`) wherever it is called.
    pub generics: HashMap<DefId, kavach_checker::Ty>,
    pub self_value: Option<Value<'a>>,
}

/// Why evaluation stopped early.
#[derive(Debug)]
pub enum Flow<'a> {
    Return(Value<'a>),
    /// `break`, with the label it names, if any (spec 4.6).
    Break(Option<Rc<str>>),
    Continue(Option<Rc<str>>),
    /// `fail`: an error value travelling to the nearest handler (spec 5).
    Fail(Value<'a>),
    /// A bug (spec 5.6): stops the program.
    Panic(String, Span),
    /// `cli.exit(code)` (spec 14.6): ends the program with this exit code.
    Exit(i32),
    /// The task is cancelled (spec 9.6): every function it is in returns
    /// at once. Not an error: `catch` doesn't see it.
    Cancelled,
}

pub type Eval<'a> = Result<Value<'a>, Flow<'a>>;

impl PartialEq for Value<'_> {
    fn eq(&self, other: &Self) -> bool {
        use Value::*;
        match (self, other) {
            (Unit, Unit) | (None, None) => true,
            (Int(a, _), Int(b, _)) => a == b,
            (Float(a), Float(b)) => a == b,
            (Float32(a), Float32(b)) => a == b,
            (Decimal(a), Decimal(b)) => a == b,
            (BigInt(a), BigInt(b)) => a == b,
            // One form per value, so a big decimal never equals a small one.
            (BigDecimal(_), BigDecimal(_)) => crate::bigint::any_decimal(self)
                .zip(crate::bigint::any_decimal(other))
                .is_some_and(|(x, y)| x.compare(&y).is_eq()),
            (Bool(a), Bool(b)) => a == b,
            (Char(a), Char(b)) => a == b,
            // A value shared by both (a copy not yet changed) is equal to
            // itself without looking inside (decision 0039).
            (Str(a), Str(b)) | (BuiltinError(a), BuiltinError(b)) => Rc::ptr_eq(a, b) || a == b,
            (List(a), List(b)) => Rc::ptr_eq(a, b) || a == b,
            (Tuple(a), Tuple(b)) => a == b,
            (Map(a), Map(b)) => Rc::ptr_eq(a, b) || a == b,
            (Regex(a), Regex(b)) => a == b,
            (Ticker(a), Ticker(b)) => a == b,
            (Bytes(a), Bytes(b)) => Rc::ptr_eq(a, b) || a == b,
            (Reader(a), Reader(b)) => Rc::ptr_eq(a, b),
            (Socket(a), Socket(b)) => std::sync::Arc::ptr_eq(a, b),
            (FakeConn(a), FakeConn(b)) => Rc::ptr_eq(a, b),
            (Task(a), Task(b)) => std::sync::Arc::ptr_eq(a, b),
            (Channel(a, x), Channel(b, y)) => Rc::ptr_eq(a, b) && x.is_some() == y.is_some(),
            // Sets are equal when they hold the same values, in any order.
            (Set(a), Set(b)) => {
                Rc::ptr_eq(a, b)
                    || a.len() == b.len() && a.entries.iter().all(|(k, _)| b.get(k).is_some())
            }
            (
                Record {
                    def: d1,
                    fields: f1,
                    ..
                },
                Record {
                    def: d2,
                    fields: f2,
                    ..
                },
            ) => d1 == d2 && fields_eq(f1, f2),
            (
                Variant {
                    union: u1,
                    name: n1,
                    fields: f1,
                    ..
                },
                Variant {
                    union: u2,
                    name: n2,
                    fields: f2,
                    ..
                },
            ) => u1 == u2 && n1 == n2 && fields_eq(f1, f2),
            (Shared(a), Shared(b)) => Rc::ptr_eq(a, b),
            (Weak(a), Weak(b)) => a.ptr_eq(b),
            (Range(a, b, _), Range(c, d, _)) => a == c && b == d,
            (Context(_, a), b) | (b, Context(_, a)) => **a == *b,
            (Ok(a), Ok(b)) | (Err(a), Err(b)) => a == b,
            (Func(a), Func(b)) => a == b,
            _ => false,
        }
    }
}

fn fields_eq<'a>(a: &[Field<'a>], b: &[Field<'a>]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.name == y.name && x.value == y.value)
}

impl Value<'_> {
    /// How `print` shows the value: text as is, everything else like code.
    pub fn display(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, true);
        out
    }

    /// The value written the way code writes it: text in quotes, even at
    /// the top level.
    pub(crate) fn display_code(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, false);
        out
    }

    /// The value as code, each variant written with its union's name, as
    /// a program writes it (decision 0070): for examples and found cases.
    /// `union` gives a union's name.
    pub(crate) fn source_code(&self, union: &dyn Fn(DefId) -> String) -> String {
        let mut out = String::new();
        self.write_as(&mut out, false, Some(union));
        out
    }

    fn write(&self, out: &mut String, top: bool) {
        self.write_as(out, top, None);
    }

    fn write_as(&self, out: &mut String, top: bool, union: Option<&dyn Fn(DefId) -> String>) {
        let list = |out: &mut String, items: &[Value<'_>]| {
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                v.write_as(out, false, union);
            }
        };
        let fields = |out: &mut String, name: &str, fields: &[Field<'_>]| {
            out.push_str(name);
            if fields.is_empty() {
                return;
            }
            out.push('(');
            for (i, f) in fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                let _ = write!(out, "{}: ", f.name);
                if f.secret {
                    out.push_str("<secret>");
                } else {
                    f.value.write_as(out, false, union);
                }
            }
            out.push(')');
        };
        match self {
            Value::Unit => out.push_str("()"),
            Value::Int(n, _) => {
                let _ = write!(out, "{n}");
            }
            Value::Float(f) => out.push_str(&kavach_runtime_core::format_float(*f)),
            Value::Float32(f) => out.push_str(&kavach_runtime_core::format_float32(*f)),
            Value::Decimal(d) => {
                let _ = write!(out, "{d}");
            }
            Value::BigInt(n) => {
                let _ = write!(out, "{n}");
            }
            Value::BigDecimal(d) => {
                let _ = write!(out, "{d}");
            }
            Value::Bool(b) => {
                let _ = write!(out, "{b}");
            }
            Value::Char(c) if top => out.push(*c),
            Value::Char(c) => {
                let _ = write!(out, "{c:?}");
            }
            Value::Str(s) if top => out.push_str(s),
            Value::Str(s) => {
                let _ = write!(out, "{s:?}");
            }
            Value::List(items) => {
                out.push('[');
                list(out, items);
                out.push(']');
            }
            Value::Regex(r) => {
                let _ = write!(out, "Regex({:?})", r.source());
            }
            Value::Ticker(ms) => {
                let _ = write!(out, "time.every_ms({ms})");
            }
            Value::Task(_) => out.push_str("Task"),
            Value::Bytes(b) => out.push_str(&crate::bytes::show(b)),
            Value::Reader(_) => out.push_str("Reader"),
            Value::Socket(_) | Value::FakeConn(_) => out.push_str("Socket"),
            Value::Channel(..) => out.push_str("Channel"),
            Value::Set(s) => {
                out.push_str("Set.of(");
                for (i, (k, _)) in s.entries.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    k.write_as(out, false, union);
                }
                out.push(')');
            }
            Value::Map(m) if m.is_empty() => out.push_str("[:]"),
            Value::Map(m) => {
                out.push('[');
                for (i, (k, v)) in m.entries.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    k.write_as(out, false, union);
                    out.push_str(": ");
                    v.write_as(out, false, union);
                }
                out.push(']');
            }
            Value::Tuple(items) => {
                out.push('(');
                list(out, items);
                out.push(')');
            }
            Value::None => out.push_str("none"),
            // A `Json` prints as compact JSON text (spec 2.12).
            Value::Variant { union, .. } if crate::json::is_json(*union) => {
                out.push_str(&kavach_runtime_core::json::write(
                    &crate::json::json_tree(self),
                    0,
                ));
            }
            Value::Record {
                name, fields: fs, ..
            } => fields(out, name, fs),
            Value::Variant {
                union: def,
                name,
                fields: fs,
                ..
            } => {
                if let Some(union) = union.map(|u| u(*def)).filter(|u| !u.is_empty()) {
                    out.push_str(&union);
                    out.push('.');
                }
                fields(out, name, fs)
            }
            Value::Closure(_) | Value::Func(_) => out.push_str("<function>"),
            Value::Agent(a) => out.push_str(&format!("<agent {}>", a.decl.name.name)),
            Value::Shared(v) => {
                out.push_str("Shared(");
                v.borrow().write_as(out, false, union);
                out.push(')');
            }
            Value::Weak(w) => {
                out.push_str("Weak(");
                match w.upgrade() {
                    Some(v) => v.borrow().write_as(out, false, union),
                    None => out.push_str("none"),
                }
                out.push(')');
            }
            Value::Range(a, b, _) => {
                let _ = write!(out, "{a}..{b}");
            }
            Value::BuiltinError(e) => out.push_str(e),
            Value::Context(text, inner) => {
                let _ = write!(out, "{text}: ");
                inner.write_as(out, true, union);
            }
            Value::Ok(v) | Value::Err(v) => v.write_as(out, top, union),
        }
    }
}
