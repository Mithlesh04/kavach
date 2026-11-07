//! `Int(text)` and friends in native code (spec 2.1): text read as a
//! number, `Bool` or `Char`. A literal was checked while compiling, so it
//! becomes its value here; other text is read when the program runs, a
//! call that can fail (`Parse`, only inside `LetResult`).

use crate::ir::{self, ExprKind as K, Shape, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast as ast;
use kavach_checker::{Prim, Ty};

/// The text of a string literal with no `{…}` in it.
fn literal_text(e: &ast::Expr) -> Option<String> {
    let ast::ExprKind::Str(parts) = &e.kind else {
        return None;
    };
    parts
        .iter()
        .map(|p| match p {
            ast::StrPart::Text(t) => Some(t.as_str()),
            ast::StrPart::Expr(_) => None,
        })
        .collect()
}

impl<'a> Lowerer<'a> {
    /// Whether `e` is text.
    pub(crate) fn is_text(&self, e: &ast::Expr) -> bool {
        self.type_at(e.span) == Some(Ty::Prim(Prim::String))
    }

    /// `T(text)` giving `ty`.
    pub(crate) fn convert_text(
        &mut self,
        text: &'a ast::Expr,
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let Some(literal) = literal_text(text) else {
            return Ok(ir::Expr {
                kind: K::Parse(Box::new(self.expr(text)?)),
                ty,
                line,
            });
        };
        let s = literal.trim();
        let bad = || self.unsupported("this conversion", span);
        let kind = match ty {
            Type::F64 => K::Float(s.parse().map_err(|_| bad())?),
            Type::F32 => K::Float(f64::from(s.parse::<f32>().map_err(|_| bad())?)),
            Type::Bool => K::Bool(s == "true"),
            Type::Char => K::Char(literal.chars().next().ok_or_else(bad)?),
            t if t.is_int() => K::Int(s.parse().map_err(|_| bad())?),
            _ if matches!(self.shapes.shape(ty), Some(Shape::Decimal | Shape::BigInt)) => {
                K::Decimal(s.to_string())
            }
            _ => return Err(bad()),
        };
        Ok(ir::Expr { kind, ty, line })
    }
}
