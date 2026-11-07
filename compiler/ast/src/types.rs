//! Types as written in source: `Int`, `[Order]`, `[String: Int]`, `User?`.

use crate::{Ident, Path, Span};

#[derive(Clone, Debug, PartialEq)]
pub struct TypeExpr {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    /// `Int`, `Pair<A, B>`, `Shape.Circle`.
    Named { path: Path, args: Vec<TypeExpr> },
    /// `[T]`
    List(Box<TypeExpr>),
    /// `[K: V]`
    Map(Box<TypeExpr>, Box<TypeExpr>),
    /// `T?`
    Optional(Box<TypeExpr>),
    /// `(A, B)`
    Tuple(Vec<TypeExpr>),
    /// `(lo: Int, hi: Int)`: an anonymous record type (spec 2.14).
    AnonRecord(Vec<(Ident, TypeExpr)>),
    /// `fn(Int, Int) -> Int`
    Fn {
        params: Vec<TypeExpr>,
        ret: Option<Box<TypeExpr>>,
    },
    /// Stands in for a type that had a syntax error.
    Error,
}
