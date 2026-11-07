//! Expressions, statements and patterns.

use crate::{Ident, Path, Span, TypeExpr};

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Let {
        mutable: bool,
        pattern: Pattern,
        ty: Option<TypeExpr>,
        value: Expr,
    },
    Assign {
        target: Expr,
        op: AssignOp,
        value: Expr,
    },
    /// With `label`, `outer: for …` (spec 4.6).
    For {
        label: Option<Ident>,
        pattern: Pattern,
        iter: Expr,
        body: Block,
    },
    While {
        label: Option<Ident>,
        cond: Expr,
        body: Block,
    },
    Assert(Expr),
    Expr(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Set,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl AssignOp {
    /// The operator `x op= v` applies (`x = x op v`), or `None` for `=`.
    pub fn binary(self) -> Option<BinaryOp> {
        Some(match self {
            AssignOp::Set => return None,
            AssignOp::Add => BinaryOp::Add,
            AssignOp::Sub => BinaryOp::Sub,
            AssignOp::Mul => BinaryOp::Mul,
            AssignOp::Div => BinaryOp::Div,
            AssignOp::Rem => BinaryOp::Rem,
            AssignOp::Pow => BinaryOp::Pow,
            AssignOp::BitAnd => BinaryOp::BitAnd,
            AssignOp::BitOr => BinaryOp::BitOr,
            AssignOp::BitXor => BinaryOp::BitXor,
            AssignOp::Shl => BinaryOp::Shl,
            AssignOp::Shr => BinaryOp::Shr,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Int(u64),
    /// A whole-number literal too big for 64 bits, as its digits (spec 2.1).
    BigInt(String),
    /// Digits as written (without `_`); a `Float` or `Decimal` by context.
    Float(String),
    Char(char),
    Str(Vec<StrPart>),
    Bool(bool),
    None,
    Name(Ident),
    SelfValue,
    /// The implicit value of a dot lambda: `.status` in `filter(.status == Paid)`.
    It,
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Tuple(Vec<Expr>),
    /// `(on: f, retries: 3)`: an anonymous record (spec 2.14).
    AnonRecord(Vec<(Ident, Expr)>),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    /// `a < b <= c`: two or more comparisons in a row, each pair of
    /// neighbours compared, each operand worked out once (spec 1.8).
    Compare(Box<Expr>, Vec<(BinaryOp, Expr)>),
    Call {
        callee: Box<Expr>,
        /// `ask<RefundDecision>(…)`
        generics: Vec<TypeExpr>,
        args: Vec<Arg>,
    },
    /// `a.b`, or `a?.b` when `optional`.
    Field {
        base: Box<Expr>,
        name: Ident,
        optional: bool,
    },
    TupleField {
        base: Box<Expr>,
        index: u64,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    /// `user with { age: 32 }` (spec 2.4).
    With {
        base: Box<Expr>,
        fields: Vec<(Ident, Expr)>,
    },
    If {
        cond: Box<Expr>,
        then: Block,
        otherwise: Option<Box<Expr>>,
    },
    IfLet {
        name: Ident,
        value: Box<Expr>,
        then: Block,
        otherwise: Option<Box<Expr>>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Block(Block),
    Lambda {
        params: Vec<LambdaParam>,
        body: Box<Expr>,
    },
    /// `.status == Paid` as an argument: a lambda over [`ExprKind::It`].
    DotLambda(Box<Expr>),
    /// `try call()`, optionally with `context "text"` (spec 5.7).
    Try {
        expr: Box<Expr>,
        context: Option<Box<Expr>>,
    },
    /// `call() catch default` or `call() catch e => handler`.
    Catch {
        expr: Box<Expr>,
        binding: Option<Ident>,
        handler: Box<Expr>,
    },
    Fail(Box<Expr>),
    Return(Option<Box<Expr>>),
    /// `break`, or `break outer` for the loop labeled `outer` (spec 4.6).
    Break(Option<Ident>),
    /// `continue`, or `continue outer`.
    Continue(Option<Ident>),
    /// `spawn x`, `spawn thread x` or `spawn cores x` (spec 9.8), with
    /// a time limit when `within d` follows (9.6).
    Spawn(SpawnKind, Box<Expr>, Option<Box<Expr>>),
    Arena(Block),
    Unsafe(Block),
    /// Stands in for an expression that had a syntax error.
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StrPart {
    Text(String),
    Expr(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    pub name: Option<Ident>,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LambdaParam {
    pub name: Ident,
    pub ty: Option<TypeExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    /// `*p`: the value a `Ptr` points at (spec 6.4).
    Deref,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    /// `a ** b` (spec 1.8).
    Pow,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Range,
    RangeInclusive,
    /// `??`
    Default,
}

impl BinaryOp {
    pub fn as_str(self) -> &'static str {
        use BinaryOp::*;
        match self {
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Rem => "%",
            Pow => "**",
            Eq => "==",
            NotEq => "!=",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
            And => "and",
            Or => "or",
            BitAnd => "&",
            BitOr => "|",
            BitXor => "^",
            Shl => "<<",
            Shr => ">>",
            Range => "..",
            RangeInclusive => "..=",
            Default => "??",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

/// Patterns in `match`, `let` and `for` (spec 4.3).
#[derive(Clone, Debug, PartialEq)]
pub enum PatternKind {
    Wildcard,
    Binding(Ident),
    Literal(Expr),
    Range {
        lo: Expr,
        hi: Expr,
        inclusive: bool,
    },
    Tuple(Vec<Pattern>),
    None,
    /// `Paid`, `Circle(radius)`, `Rect(width: w, height: _)`, `Shape.Circle`.
    Variant {
        path: Path,
        fields: Option<Vec<FieldPattern>>,
    },
    /// `ok(value)` and `err(e)` for calls that can fail (spec 5.4).
    Ok(Box<Pattern>),
    Err(Box<Pattern>),
    /// `[first, second = 0, ...rest]` in `const` and `let` (spec 1.12).
    List {
        items: Vec<ListItem>,
        rest: Option<Ident>,
    },
    /// `{ name, age: years, city = "unknown" }` in `const` and `let`
    /// (spec 1.12).
    Record(Vec<RecordFieldPattern>),
}

/// An item of a list pattern: a name or `_`, maybe with a default. On a
/// map, `key` says which value it takes (`"asha": a`, `host: server`);
/// a bare name takes the text key spelled like it (spec 1.12).
#[derive(Clone, Debug, PartialEq)]
pub struct ListItem {
    pub key: Option<MapKey>,
    pub pattern: Pattern,
    pub default: Option<Expr>,
}

/// The key of a map pattern's item.
#[derive(Clone, Debug, PartialEq)]
pub enum MapKey {
    /// `host:`: the text key `"host"`.
    Name(Ident),
    /// `"asha":` or `7:`.
    Literal(Expr),
}

/// A field of a record pattern: `name`, `age: years`, `city = "?"`, or
/// `home: { city }`, a pattern nested in it.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordFieldPattern {
    pub field: Ident,
    /// What takes the field: a name (`field` itself unless renamed), or
    /// a nested pattern.
    pub pattern: Pattern,
    pub default: Option<Expr>,
}

/// `radius` (binds the field of that name) or `width: w`.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldPattern {
    pub name: Option<Ident>,
    pub pattern: Pattern,
}

/// How a spawned task runs (spec 9.8, decision 0069).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpawnKind {
    /// `spawn`: a coroutine on the spawner's thread.
    #[default]
    Coroutine,
    /// `spawn thread`: an operating-system thread of its own.
    Thread,
    /// `spawn cores`: on a worker thread per core.
    Cores,
    /// `spawn process`: an operating-system process of its own, running
    /// one call of the program's code.
    Process,
}

impl SpawnKind {
    /// The word after `spawn` that asks for this kind, if any.
    pub fn word(self) -> Option<&'static str> {
        match self {
            SpawnKind::Coroutine => None,
            SpawnKind::Thread => Some("thread"),
            SpawnKind::Cores => Some("cores"),
            SpawnKind::Process => Some("process"),
        }
    }
}
