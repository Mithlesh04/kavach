//! The Kavach syntax tree: what the parser produces and every later compiler
//! stage reads. It mirrors the source closely; names are not resolved and
//! types are not checked yet.

mod exprs;
mod items;
mod types;
pub mod visit;

pub use exprs::*;
pub use items::*;
pub use kavach_lexer::Span;
pub use types::*;

/// A name as written in the source.
#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// Dotted names such as `Payments.refund` or `billing.invoice`.
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub segments: Vec<Ident>,
    pub span: Span,
}
