//! `inout` parameters (spec 3.9, decision 0038): each argument for one is
//! a place the caller could assign to, and its variable appears nowhere
//! else in the call, so the function never sees two names for one value.

use crate::checker::{Checker, FnSig};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};

impl Checker<'_> {
    /// Checks the `inout` arguments of a call with signature `sig`.
    pub(crate) fn check_inout_args(&mut self, sig: &FnSig, args: &[Arg]) {
        if !sig.params.iter().any(|p| p.inout) {
            return;
        }
        // The parameter each argument goes to: by position, then by name.
        let index = |i: usize, a: &Arg| match &a.name {
            Some(n) => sig.params.iter().position(|p| p.name == n.name),
            None => Some(i),
        };
        let mut roots: Vec<(usize, DefId)> = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let Some(p) = index(i, a).and_then(|k| sig.params.get(k)) else {
                continue;
            };
            if !p.inout {
                continue;
            }
            match self.place_root(&a.value) {
                Ok(def) => roots.push((i, def)),
                Err(why) => self.error(
                    a.value.span,
                    format!("`{}` is `inout`, so {why} (spec 3.9)", p.name),
                ),
            }
        }
        for (i, root) in &roots {
            for (j, other) in args.iter().enumerate() {
                if j != *i && self.names(&other.value, *root) {
                    let name = self.res.defs[*root].name.clone();
                    self.error(
                        other.value.span,
                        format!("`{name}` is given as `inout` in this call, so it can't be another argument too (spec 3.9)"),
                    );
                }
            }
        }
    }

    /// The variable at the root of place `e` (`count`, `user.age`,
    /// `scores[0]`), or why `e` isn't a place an `inout` parameter can take.
    fn place_root(&self, e: &Expr) -> Result<DefId, String> {
        match &e.kind {
            ExprKind::Name(id) => {
                let Some(&def) = self.res.uses.get(&id.span.start) else {
                    return Err("its argument must be a variable".into());
                };
                match self.res.defs[def].kind {
                    DefKind::Local { mutable: true } => Ok(def),
                    DefKind::Local { mutable: false } => Err(format!(
                        "`{}` must be able to change: declare it with `let`, not `const`",
                        id.name
                    )),
                    _ => Err(format!(
                        "its argument must be a `let` variable, and `{}` is not one",
                        id.name
                    )),
                }
            }
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => self.place_root(base),
            _ => Err("its argument must be a `let` variable, or a field or item of one".into()),
        }
    }

    /// Whether `e` reads variable `def` anywhere.
    fn names(&self, e: &Expr, def: DefId) -> bool {
        let mut found = false;
        let mut look = |x: &Expr| {
            if let ExprKind::Name(id) = &x.kind
                && self.res.uses.get(&id.span.start) == Some(&def)
            {
                found = true;
            }
        };
        visit_exprs(e, &mut look);
        found
    }
}

/// Calls `f` on `e` and every expression inside it (on a copy: the
/// visitor the AST has changes what it visits).
fn visit_exprs(e: &Expr, f: &mut impl FnMut(&Expr)) {
    use kavach_ast::visit::{VisitMut, walk_expr};
    struct V<'f, F: FnMut(&Expr)>(&'f mut F);
    impl<F: FnMut(&Expr)> VisitMut for V<'_, F> {
        fn expr(&mut self, e: &mut Expr) {
            (self.0)(e);
            walk_expr(self, e);
        }
    }
    V(f).expr(&mut e.clone());
}
