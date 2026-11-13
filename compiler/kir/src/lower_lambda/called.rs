//! Lambdas and functions called, not inlined: a named function given
//! where a lambda goes, and a lambda with a `return` of its own, which
//! must end only its own call (spec 3.3), not the function it is in.

use crate::ir::{self, ExprKind as K};
use crate::lower::{Lowered, Lowerer};
use kavach_ast::{self as ast, ExprKind};
use kavach_resolver::DefKind;

/// Whether `body` has a `return` of its own: one outside any lambda
/// written inside it.
pub(super) fn own_return(body: &ast::Expr) -> bool {
    struct Find(bool);
    impl kavach_ast::visit::VisitMut for Find {
        fn expr(&mut self, e: &mut ast::Expr) {
            match &e.kind {
                ExprKind::Return(_) => self.0 = true,
                ExprKind::Lambda { .. } | ExprKind::DotLambda(_) => {}
                _ => kavach_ast::visit::walk_expr(self, e),
            }
        }
    }
    let mut find = Find(false);
    kavach_ast::visit::VisitMut::expr(&mut find, &mut body.clone());
    find.0
}

impl<'a> Lowerer<'a> {
    /// The function `f` names, when calling it with `n` values needs
    /// nothing a plain call doesn't: not generic, not from C, no errors,
    /// no `inout` and no defaults left out.
    pub(super) fn plain_fn(&self, f: &ast::Expr, n: usize) -> Option<ir::FuncId> {
        let ExprKind::Name(id) = &f.kind else {
            return None;
        };
        let res = &self.c.resolution;
        let def = *res.uses.get(&id.span.start)?;
        if !matches!(res.defs[def].kind, DefKind::Fn) || self.generic_fns.contains_key(&def) {
            return None;
        }
        let fid = *self.fn_ids.get(&def)?;
        let decl = self.decl_of(fid);
        let plain = decl.c_library.is_none()
            && decl.errors.is_none()
            && decl.params.len() == n
            && decl.params.iter().all(|p| !p.is_inout);
        plain.then_some(fid)
    }

    /// The lambda `f` made into a closure and called with `args`.
    pub(super) fn called_lambda(
        &mut self,
        f: &'a ast::Expr,
        args: Vec<ir::Expr>,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let closure = self.expr(f)?;
        let (params, ret) = self.fn_parts(closure.ty, f.span)?;
        let args = (args.into_iter().zip(params))
            .map(|(a, t)| self.coerce(a, t))
            .collect::<Lowered<Vec<_>>>()?;
        Ok(ir::Expr {
            kind: K::CallValue(Box::new(closure), args),
            ty: ret,
            line,
        })
    }
}
