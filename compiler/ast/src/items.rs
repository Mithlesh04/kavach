//! Declarations: functions, types, interfaces, agents, `use` and tests.

use crate::{Block, Expr, Ident, Path, Span, Stmt, TypeExpr};

/// One `.kav` file.
#[derive(Clone, Debug, PartialEq)]
pub struct File {
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub kind: ItemKind,
    /// `///` doc comments written just before the item.
    pub docs: Vec<String>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemKind {
    Fn(FnDecl),
    Type(TypeDecl),
    Interface(InterfaceDecl),
    Extend(ExtendDecl),
    Capability(CapabilityDecl),
    Agent(AgentDecl),
    Use(UseDecl),
    Test(TestDecl),
    /// A top-level statement in an entry file (spec 1.3).
    Stmt(Stmt),
}

/// A function, method or agent handler, with its full signature (spec 3.5).
#[derive(Clone, Debug, PartialEq)]
pub struct FnDecl {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    /// `-> Type`; `None` means the function returns nothing (spec 3.1).
    pub ret: Option<TypeExpr>,
    /// `-> secret T`: its result is a secret (spec 8.7).
    pub ret_secret: bool,
    /// `! ErrorType` (spec 5.1).
    pub errors: Option<TypeExpr>,
    /// `uses Net, Payments.refund` (spec 8.1).
    pub uses: Vec<Path>,
    /// The folder of `uses Fs("./data")`: file access is kept inside it.
    pub fs_scope: Option<String>,
    pub requires: Vec<Expr>,
    pub ensures: Vec<Expr>,
    pub examples: Vec<Example>,
    /// `None` for signatures in interfaces, capabilities and `extern`.
    pub body: Option<Block>,
    /// The C library of a function declared in `extern c "name" { … }`
    /// (spec 10.7); the parser gives each its own item.
    pub c_library: Option<String>,
    /// `gpu fn`: may run on the GPU, so uses only what a GPU can (spec 15).
    pub is_gpu: bool,
    /// `deprecated "…" fix …` on the line before (spec 12.6).
    pub deprecated: Option<Box<Deprecation>>,
}

/// `deprecated "use refund_order instead" fix refund_order(order)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Deprecation {
    pub message: String,
    /// What a call becomes; the function's parameters stand for its
    /// arguments.
    pub fix: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenericParam {
    pub name: Ident,
    /// `T: Comparable` bounds.
    pub bounds: Vec<TypeExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: Ident,
    /// Always written, except for `self`.
    pub ty: Option<TypeExpr>,
    pub default: Option<Expr>,
    /// `secret card: String`: it may receive a secret (spec 8.7).
    pub is_secret: bool,
    /// `inout n: Int`: the caller's variable comes in, and its last value
    /// goes back (spec 3.9).
    pub is_inout: bool,
}

/// `example call == value` or `example call fails Error` (spec 7.6).
#[derive(Clone, Debug, PartialEq)]
pub struct Example {
    pub expr: Expr,
    pub fails: Option<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDecl {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    /// Interfaces or capabilities after `:`.
    pub implements: Vec<TypeExpr>,
    pub body: TypeBody,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeBody {
    Record(RecordBody),
    Union(Vec<Variant>),
    /// `type Handler = (on: fn(Int) -> Int)`: an alias of an anonymous
    /// record type (spec 2.14).
    Alias(TypeExpr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordBody {
    pub fields: Vec<Field>,
    pub methods: Vec<Item>,
    pub invariants: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: Ident,
    pub ty: TypeExpr,
    pub default: Option<Expr>,
    pub is_pub: bool,
    pub is_secret: bool,
    pub docs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
    pub name: Ident,
    pub fields: Vec<Field>,
    /// An enum variant's number: `NotFound = 404` (spec 2.6).
    pub value: Option<i128>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceDecl {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub methods: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExtendDecl {
    /// `extend<S> App<S>`: names for the type's own parameters (spec 2.5).
    pub generics: Vec<GenericParam>,
    pub target: TypeExpr,
    pub implements: Vec<TypeExpr>,
    pub methods: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityDecl {
    pub name: Ident,
    pub methods: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentDecl {
    pub name: Ident,
    pub members: Vec<AgentMember>,
}

/// The parts of an agent (spec 8.3).
#[derive(Clone, Debug, PartialEq)]
pub enum AgentMember {
    Can(Vec<Path>),
    Limit {
        target: Path,
        rules: Vec<LimitRule>,
    },
    Approve {
        target: Path,
        when: Expr,
        by: Expr,
        /// How long a request may wait (`within 2.days`): a `Duration`;
        /// without it, 7 days (decision 0072).
        within: Option<Expr>,
    },
    On(FnDecl),
    /// `model careful`: the `Model` its `ask`s use (spec 8.3, decision
    /// 0056).
    Model(Expr),
    /// `settings standard()`: its `AgentSettings` (spec 8.5, decision 0060).
    Settings(Expr),
}

/// `sum(amount) <= 50_000 per day`
#[derive(Clone, Debug, PartialEq)]
pub struct LimitRule {
    pub expr: Expr,
    pub per: Option<Ident>,
}

/// `use billing.invoice.{Invoice, create}` or `use std.collections.Set as HashSet`.
#[derive(Clone, Debug, PartialEq)]
pub struct UseDecl {
    pub path: Path,
    pub names: Vec<Ident>,
    pub alias: Option<Ident>,
}

/// `test "name" uses Net with Payments = FakePayments() { … }` (spec 11.8).
#[derive(Clone, Debug, PartialEq)]
pub struct TestDecl {
    pub name: Expr,
    pub uses: Vec<Path>,
    pub with: Vec<(Ident, Expr)>,
    pub body: Block,
}
