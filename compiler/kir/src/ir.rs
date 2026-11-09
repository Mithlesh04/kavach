//! The KIR data types.

pub use crate::rt_op::RtOp;

/// How a value is stored in machine code. Heap values (text, lists,
/// records, unions, optionals) are `Ref`: a counted pointer to a run-time
/// object whose layout is described by `Program::shapes[id]`.
/// Scalar types in the order their run-time codes count; `Ref(id)` is
/// `100 + id`. The runtime and interface boxes (spec 2.15) share it.
pub const TYPE_CODES: [Type; 13] = [
    Type::I8,
    Type::I16,
    Type::I32,
    Type::I64,
    Type::U8,
    Type::U16,
    Type::U32,
    Type::U64,
    Type::F64,
    Type::Bool,
    Type::Unit,
    Type::Char,
    Type::F32,
];

pub fn type_code(t: Type) -> i64 {
    match t {
        Type::Ref(id) => 100 + id as i64,
        t => TYPE_CODES.iter().position(|x| *x == t).unwrap_or(3) as i64,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F64,
    Bool,
    Char,
    Unit,
    Ref(TypeId),
    /// IEEE single precision (`Float32`, spec 2.1). In a heap slot, its
    /// 32 bits, zero-extended.
    F32,
}

impl Type {
    pub fn is_int(self) -> bool {
        self.is_signed() || matches!(self, Type::U8 | Type::U16 | Type::U32 | Type::U64)
    }

    pub fn is_signed(self) -> bool {
        matches!(self, Type::I8 | Type::I16 | Type::I32 | Type::I64)
    }

    /// `Float` or `Float32`.
    pub fn is_float(self) -> bool {
        matches!(self, Type::F64 | Type::F32)
    }

    pub fn is_ref(self) -> bool {
        matches!(self, Type::Ref(_))
    }

    pub fn bits(self) -> u32 {
        match self {
            Type::I8 | Type::U8 | Type::Bool => 8,
            Type::I16 | Type::U16 => 16,
            Type::I32 | Type::U32 | Type::Char | Type::F32 => 32,
            Type::Unit => 0,
            _ => 64,
        }
    }

    /// The Kavach name, for run-time messages ("doesn't fit in Int").
    pub fn kavach_name(self) -> &'static str {
        match self {
            Type::I8 => "Int8",
            Type::I16 => "Int16",
            Type::I32 => "Int32",
            Type::I64 => "Int",
            Type::U8 => "UInt8",
            Type::U16 => "UInt16",
            Type::U32 => "UInt32",
            Type::U64 => "UInt64",
            Type::F64 => "Float",
            Type::F32 => "Float32",
            Type::Bool => "Bool",
            Type::Char => "Char",
            Type::Unit => "nothing",
            Type::Ref(_) => "value",
        }
    }
}

pub type Local = u32;
pub type FuncId = u32;
pub type TypeId = u32;

/// The layout of a heap value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    Str,
    /// Raw data, one byte per byte (spec 2.13).
    Bytes,
    /// A TCP listener or connection (`Listener`, `Connection`, spec 14.10).
    Socket,
    /// A file read as a `for` loop goes, giving items of this type: its
    /// lines (text) or pieces (`Bytes`) (spec 14.3).
    Reader(Type),
    /// An exact base-10 number (spec 2.1).
    Decimal,
    /// A whole number of any size (spec 2.1). Made, negated, converted
    /// and computed with by the same runtime calls as `Decimal`.
    BigInt,
    /// An error of any type (`! Error`): the object knows its own type.
    Error,
    List(Type),
    /// Keys and values in insertion order (spec 2.2).
    Map(Type, Type),
    /// Values in insertion order, each once (spec 14.2). Stored like a map
    /// whose values are all `Unit`.
    Set(Type),
    /// One value that every copy shares (`Shared<T>`, spec 6.2).
    Shared(Type),
    /// A spawned task that gives a value of this type (`Task<T>`, spec
    /// 9.1). The object behind it is the runtime's.
    Task(Type),
    /// Either end of a channel of values of this type (`Sender<T>`,
    /// `Receiver<T>`, spec 9.4): both ends are the one channel object.
    Channel(Type),
    /// A compiled regular expression (spec 14.8).
    Regex,
    /// A function value: parameter types and return type (spec 3.4).
    /// The object behind it is a `Closure`.
    Func(Vec<Type>, Type),
    /// A closure object: the function's address, then the captured
    /// values of these types.
    Closure(Vec<Type>),
    /// An interface value (spec 2.15, decision 0029): the concrete value,
    /// its type (as a slot, for printing, equality and freeing), then one
    /// closure per method, in the interface's declared order.
    Iface {
        name: String,
        methods: Vec<(String, Type)>,
    },
    /// `none` is a null pointer. A present value is the inner pointer
    /// itself, or a box when the inner type is not a `Ref`.
    Optional(Type),
    Tuple(Vec<Type>),
    Record {
        name: String,
        fields: Vec<FieldShape>,
        /// The type implements `Drop` (spec 6.1): its values are never
        /// stored flat in a list, so each item can wait for its `drop`.
        drop: bool,
    },
    Union {
        name: String,
        variants: Vec<VariantShape>,
    },
}

/// The most fields an item of a flat list may have.
pub const FLAT_MAX: usize = 8;

/// How many words each item of list type `list` takes when the list is
/// flat, or 0. A list is flat when its items are records or tuples whose
/// fields are all scalars (numbers, booleans, characters): their fields
/// are stored in the list's buffer, not in an object per item
/// (`docs/architecture/flat-lists.md`).
pub fn flat_width(shapes: &[Shape], list: Type) -> usize {
    let Type::Ref(id) = list else { return 0 };
    let Some(Shape::List(Type::Ref(item))) = shapes.get(id as usize) else {
        return 0;
    };
    let fields: Vec<Type> = match &shapes[*item as usize] {
        Shape::Record { drop: true, .. } => return 0,
        Shape::Record { fields, .. } => fields.iter().map(|f| f.ty).collect(),
        Shape::Tuple(ts) => ts.clone(),
        _ => return 0,
    };
    let plain = !fields.is_empty() && fields.len() <= FLAT_MAX;
    if plain && fields.iter().all(|t| !t.is_ref()) {
        fields.len()
    } else {
        0
    }
}

/// The bytes a list packs each item of type `item` into (decisions 0030
/// and 0045), or 0 for one 64-bit slot. `Char` stays a slot: it is a
/// code point, not a number.
pub fn packed_size(item: Type) -> usize {
    match item {
        Type::U8 | Type::I8 | Type::Bool => 1,
        Type::U16 | Type::I16 => 2,
        Type::U32 | Type::I32 | Type::F32 => 4,
        _ => 0,
    }
}

/// The bytes per item of list type `list` when it packs them, or 0.
pub fn item_bytes(shapes: &[Shape], list: Type) -> usize {
    let Type::Ref(id) = list else { return 0 };
    match shapes.get(id as usize) {
        Some(Shape::List(item)) => packed_size(*item),
        _ => 0,
    }
}

/// Whether list type `list` holds text. Such a list keeps each item in
/// two words: text up to 15 bytes inside them, longer text as a pointer
/// to a text object (`docs/architecture/flat-lists.md`).
pub fn text_list(shapes: &[Shape], list: Type) -> bool {
    let Type::Ref(id) = list else { return false };
    matches!(shapes.get(id as usize), Some(Shape::List(Type::Ref(item)))
        if shapes.get(*item as usize) == Some(&Shape::Str))
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FieldShape {
    pub name: String,
    pub ty: Type,
    pub secret: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VariantShape {
    pub name: String,
    pub fields: Vec<FieldShape>,
}

pub struct Program {
    pub path: String,
    pub functions: Vec<Function>,
    /// `main`, or the top-level statements (spec 1.3).
    pub entry: FuncId,
    /// A library's exported functions, with their names (decision 0026).
    pub exports: Vec<(FuncId, String)>,
    /// The layout of every heap type, by `TypeId`.
    pub shapes: Vec<Shape>,
    /// Functions from this index on are `quiet` loop chunks (decision
    /// 0031): their checks return 1 instead of panicking.
    pub quiet_from: FuncId,
    /// Lines of loops that wait for the GPU each round, and why, for
    /// `kavach check` (decision 0067).
    pub notes: Vec<(u32, String)>,
}

pub struct Function {
    pub name: String,
    pub params: Vec<Local>,
    pub ret: Type,
    /// The type of every local, by index.
    pub locals: Vec<Type>,
    pub body: Block,
    pub line: u32,
    /// The function can fail (`! E`, spec 5): it also returns an error
    /// pointer, which is 0 on success.
    pub fails: bool,
    /// A C function (`extern c`, spec 10.7): its library. It has no body
    /// here; native code calls the C symbol `name`.
    pub external: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// The block's value, if it has one.
    pub value: Option<Box<Expr>>,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let(Local, Expr),
    Expr(Expr),
    While {
        cond: Expr,
        body: Block,
    },
    /// `for var in start..end` (or `..=` when `inclusive`).
    ForRange {
        var: Local,
        start: Expr,
        end: Expr,
        inclusive: bool,
        body: Block,
    },
    /// `for var in list`, where `list` is a local holding the list and
    /// `index` counts from 0 (for `enumerate`).
    ForEach {
        var: Local,
        index: Local,
        list: Local,
        body: Block,
    },
    /// Calls something that can fail, keeping its value and its error
    /// (0 when it succeeded) in two locals.
    LetResult {
        value: Local,
        error: Local,
        call: Expr,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    Print(Vec<PrintPart>),
    /// Panics with `message` when `cond` is false: contracts and `assert`.
    Check {
        cond: Expr,
        message: String,
        line: u32,
    },
}

#[derive(Clone, Debug)]
pub enum PrintPart {
    Text(String),
    Value(Expr),
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Type,
    /// For run-time messages ("at file.kav:12").
    pub line: u32,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(i128),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(String),
    /// A `Decimal` or `BigInt` literal, as written: the type tells which.
    Decimal(String),
    /// Text made from the printed form of each part (string interpolation).
    Format(Vec<Expr>),
    Local(Local),
    /// The last read of a heap variable: its value is handed over and the
    /// variable emptied, so it needn't be shared (`moves.rs`).
    Move(Local),
    /// Assignment, as an expression of type `Unit`.
    Assign(Local, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// Integer arithmetic that the range pass (`ranges.rs`) proved can
    /// never fail, so it needs no check.
    Fast(FastOp, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Call(FuncId, Vec<Expr>),
    /// A function value: function `func` (whose first parameter is the
    /// closure), with `captures` stored in a closure object of type `env`.
    /// With an `env` of `Unit` and no captures, it is the bare address of
    /// `func`, an `I64` that C calls back (spec 10.7).
    Closure {
        func: FuncId,
        env: Type,
        captures: Vec<Expr>,
    },
    /// Calls a function value with these arguments.
    CallValue(Box<Expr>, Vec<Expr>),
    If(Box<Expr>, Block, Block),
    Block(Block),
    /// Runs the first block; if a check in it fails (a panic), runs the
    /// second instead. The second is the code as written, which then
    /// fails where the program says; the first does the same work faster
    /// and sets only its own locals (`fuse.rs`).
    Fallback(Block, Block),
    /// Converts a number to `ty` (spec 2.1), panicking if it doesn't fit.
    /// `false` means the value is proven to fit (`ranges.rs`), so no
    /// check is emitted.
    Convert(Box<Expr>, bool),
    Panic(String),
    /// The missing optional.
    None,
    /// Wraps a value as a present optional of type `ty`.
    Some(Box<Expr>),
    IsSome(Box<Expr>),
    /// The value inside an optional; panics on `none`.
    Unwrap(Box<Expr>),
    /// A record (`variant` 0) or a union variant, with every field in order.
    Record {
        variant: u32,
        fields: Vec<Expr>,
    },
    GetField(Box<Expr>, u32),
    /// A copy of a record with one field changed (copy-on-write).
    SetField(Box<Expr>, u32, Box<Expr>),
    /// Whether a union value is the variant at this index.
    HasVariant(Box<Expr>, u32),
    List(Vec<Expr>),
    /// A map literal: keys and values in order.
    Map(Vec<(Expr, Expr)>),
    Tuple(Vec<Expr>),
    /// `list[i]`, or element `i` of a tuple; panics when out of range.
    Index(Box<Expr>, Box<Expr>),
    /// `list[i]` where a check before proved `i` in range, so it needs no
    /// check of its own (made by `jam.rs`).
    InRange(Box<Expr>, Box<Expr>),
    /// A built-in method of text, lists, maps, `Shared` or integers, with
    /// up to three operands.
    Rt(RtOp, Vec<Expr>),
    /// `Int(text)` and friends: can fail, so only inside `LetResult`.
    Parse(Box<Expr>),
    /// Returns from the function with this error (spec 5.1).
    Fail(Box<Expr>),
    /// An error with text added in front (`try … context "…"`, spec 5.7).
    Context(Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FastOp {
    Add,
    Sub,
    Mul,
    /// `a % b` where `0 <= a < 2 * b`: one compare and one subtract.
    RemOnce,
    /// `a / b` and `a % b` where `a >= 0` and `b > 0`: unsigned division,
    /// which is shorter than signed division by a constant.
    DivPositive,
    RemPositive,
    /// The same when both are also below 2³²: 32-bit division, which is
    /// faster, and which vector units have.
    DivSmall,
    RemSmall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl BinOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
        }
    }
}
