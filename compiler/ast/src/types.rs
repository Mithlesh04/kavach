// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Types as written in source: `Int`, `[Order]`, `{String: Int}`, `User?`,
//! `[Float]<len: 3>`.

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
    /// `fn(Int, Int) -> Int ! E uses (Net, Db)` (spec 3.4)
    Fn {
        params: Vec<TypeExpr>,
        ret: Option<Box<TypeExpr>>,
        /// `! E`: what its functions can fail with.
        errors: Option<Box<TypeExpr>>,
        /// `uses Net`: the most its functions may use; `None` sets no
        /// limit, `Some([])` is never written.
        uses: Option<Vec<crate::Path>>,
    },
    /// `max: 2` in `<…>`: a number argument (spec 2.9). A number written
    /// without its name has `name: None`, a compile error that shows it.
    Num {
        name: Option<Ident>,
        value: Box<crate::Expr>,
    },
    /// `[Int]<max: 2>`, `{K: V}<max: 100>`: numbers after a list's or a
    /// dict's type (spec 2.2); each is a `Num`.
    WithNums(Box<TypeExpr>, Vec<TypeExpr>),
    /// Stands in for a type that had a syntax error.
    Error,
}
