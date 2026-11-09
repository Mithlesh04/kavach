//! Lambdas (spec 3.3): their parameter types from where they are used,
//! and those whose types are settled after the rest of the function.

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::*;
use kavach_resolver::DefId;

impl Checker<'_> {
    /// `let f = x => …` with untyped parameters: the lambda gets a type
    /// with unknown parts now, and its body is checked later (see
    /// `deferred`), so `x.len()` inside it knows what `x` is.
    pub(crate) fn defer_lambda(&mut self, pattern: &Pattern, value: &Expr) -> bool {
        let (PatternKind::Binding(id), ExprKind::Lambda { params, .. }) =
            (&pattern.kind, &value.kind)
        else {
            return false;
        };
        let Some(&def) = self.decl.get(&id.span.start) else {
            return false;
        };
        if params.iter().all(|p| p.ty.is_some()) {
            return false;
        }
        let types = params
            .iter()
            .map(|p| match &p.ty {
                Some(t) => self.lower(t, None),
                None => self.fresh(),
            })
            .collect();
        let ty = Ty::Fn(types, Box::new(self.fresh()));
        self.locals.insert(def, ty.clone());
        self.deferred.insert(def, (value.clone(), ty));
        true
    }

    /// Checks the body of deferred lambda `def`, or of every one left
    /// when `def` is `None`.
    pub(crate) fn check_deferred(&mut self, def: Option<DefId>) {
        let waiting: Vec<DefId> = match def {
            Some(d) => vec![d],
            None => {
                let mut all: Vec<DefId> = self.deferred.keys().copied().collect();
                all.sort_unstable();
                all
            }
        };
        for d in waiting {
            if let Some((lambda, ty)) = self.deferred.remove(&d) {
                self.infer(&lambda, Some(&ty));
            }
        }
    }

    pub(crate) fn lambda(
        &mut self,
        params: &[LambdaParam],
        body: &Expr,
        expected: Option<&Ty>,
    ) -> Ty {
        let (want_params, want_ret) = match expected.map(|t| self.inf.shallow(t)) {
            Some(Ty::Fn(ps, r)) if ps.len() == params.len() => (Some(ps), Some(*r)),
            _ => (None, None),
        };
        let mut types = Vec::new();
        for (i, p) in params.iter().enumerate() {
            let t = match (&p.ty, &want_params) {
                (Some(ty), _) => self.lower(ty, None),
                (None, Some(ps)) => ps[i].clone(),
                (None, None) => self.fresh(),
            };
            if let Some(&def) = self.decl.get(&p.name.span.start) {
                self.locals.insert(def, t.clone());
            }
            types.push(t);
        }
        self.copies.push((body.span.start, "lambda"));
        let outer_loops = std::mem::take(&mut self.loops);
        // A body checked against the wanted result gives that type: a
        // `Node` body is a `Node?` lambda where one is wanted (spec 2.7).
        let ret = match want_ret {
            Some(r) => {
                self.check_expect(body, &r);
                r
            }
            None => self.value(body, None),
        };
        self.loops = outer_loops;
        self.copies.pop();
        Ty::Fn(types, Box::new(ret))
    }
}
