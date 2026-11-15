//! Operator bounds (spec 2.16, 2.9, decision 0115): `T: Add` lets generic
//! code use `a + b` on two `T`s, which calls `add`: a program's own
//! type's method, or a number's, as every number type meets `Add`, `Sub`,
//! `Mul`, `Div`, `Rem`, `Neg` and `Comparable`. Text meets `Comparable`.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;

/// Each operator interface a bound can name, and its method.
const BOUNDS: [(&str, &str); 7] = [
    ("Add", "add"),
    ("Sub", "sub"),
    ("Mul", "mul"),
    ("Div", "div"),
    ("Rem", "rem"),
    ("Neg", "neg"),
    ("Comparable", "compare"),
];

impl Checker<'_> {
    /// Notes `def`'s operator bounds from `g`, with no type arguments.
    pub(crate) fn note_op_bounds(&mut self, g: &GenericParam, def: DefId) {
        for b in &g.bounds {
            let TypeKind::Named { path, args } = &b.kind else {
                continue;
            };
            let name = path.segments.last().map(|s| s.name.as_str());
            if let Some((_, m)) = BOUNDS.iter().find(|(i, _)| Some(*i) == name)
                && args.is_empty()
                && path.segments.len() == 1
            {
                self.param_ops.entry(def).or_default().push(m);
            }
        }
    }

    /// `a op b` with `a` a `T` bounded by the operator's interface: noted
    /// as the method call it is. `None` when `T` has no such bound.
    pub(crate) fn param_binary(
        &mut self,
        method: &'static str,
        l: &Expr,
        r: &Expr,
        lt: &Ty,
    ) -> Option<Ty> {
        let Ty::Param(p) = self.inf.resolve(lt) else {
            return None;
        };
        if !self
            .param_ops
            .get(&p)
            .is_some_and(|ms| ms.contains(&method))
        {
            return None;
        }
        self.check_expect(r, lt);
        self.user_ops.insert((l.span.start, r.span.end), method);
        Some(if method == "compare" {
            Ty::Prim(Prim::Bool)
        } else {
            Ty::Param(p)
        })
    }

    /// `-a` with `a` a `T: Neg`.
    pub(crate) fn param_neg(&mut self, x: &Expr, t: &Ty) -> Option<Ty> {
        let Ty::Param(p) = self.inf.resolve(t) else {
            return None;
        };
        if !self.param_ops.get(&p).is_some_and(|ms| ms.contains(&"neg")) {
            return None;
        }
        self.user_negs.insert((x.span.start, x.span.end));
        Some(Ty::Param(p))
    }

    /// Whether type `t` has operator method `m`: a number has them all,
    /// text `compare`, a program's own type the ones it
    /// implements, and a type parameter those its bounds give.
    pub(crate) fn has_operator(&self, t: &Ty, m: &str) -> Option<bool> {
        Some(match self.inf.resolve(t) {
            Ty::Prim(p) if p.is_numeric() => true,
            Ty::Prim(Prim::String) => m == "compare",
            Ty::Adt(def, _) => self.operators.contains_key(&(def, m)),
            // A number type has every operator, so a `Number` `T` does too.
            Ty::Param(q) => {
                self.param_ops.get(&q).is_some_and(|ms| ms.contains(&m))
                    || self.inf.numbers.contains_key(&q)
            }
            Ty::Var(_) | Ty::Error | Ty::Never => return None,
            _ => false,
        })
    }

    /// Checks that `t`, given for type parameter `g`, has the operators
    /// `g`'s bounds ask for.
    pub(crate) fn op_bounds_now(&mut self, g: DefId, t: &Ty, span: Span) {
        let Some(methods) = self.param_ops.get(&g).cloned() else {
            return;
        };
        for m in methods {
            if self.has_operator(t, m) == Some(false) {
                let (shown, name) = (self.show(t), &self.res.defs[g].name);
                let iface = BOUNDS.iter().find(|(_, x)| *x == m).map_or("", |(i, _)| i);
                self.error(span, format!("`{shown}` doesn't implement `{iface}`, which `{name}` needs here (spec 2.16)"));
            }
        }
    }
}
