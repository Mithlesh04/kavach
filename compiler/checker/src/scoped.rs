//! Blocks given to a call (spec 4.8): `log.span(name) { … }` and
//! `random.seeded(seed) { … }`. The call is checked here; the block is a
//! plain block, whose value is the expression's.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::{Block, Expr, ExprKind, Span};

/// A parameter: its name, type, and whether it has a default.
type Param = (&'static str, Prim, bool);

/// The standard library functions that take a block: module, name and
/// parameters.
const TAKES_A_BLOCK: &[(&str, &str, &[Param])] = &[
    (
        "log",
        "span",
        &[("name", Prim::String, false), ("from", Prim::String, true)],
    ),
    ("random", "seeded", &[("seed", Prim::Int, false)]),
];

impl Checker<'_> {
    /// `call { body }`: the call must be one of `TAKES_A_BLOCK`.
    pub(crate) fn scoped(&mut self, call: &Expr, body: &Block, expected: Option<&Ty>) -> Ty {
        self.check_scoped_call(call);
        self.block_value(body, expected)
    }

    /// The call a block is given to, and its argument.
    pub(crate) fn check_scoped_call(&mut self, call: &Expr) {
        match self.scoped_call(call) {
            Some((module, name, given)) => {
                if let ExprKind::Call { args, .. } = &call.kind {
                    let params: Vec<_> = (given.iter())
                        .map(|(n, p, default)| (n.to_string(), Ty::Prim(*p), *default))
                        .collect();
                    let owner = format!("`{module}.{name}`");
                    self.args(args, &params, call.span, &owner, "argument");
                }
            }
            None => self.error(
                call.span,
                "only `log.span(name)` and `random.seeded(seed)` take a block after them (spec 4.8)",
            ),
        }
    }

    /// Which standard library function `call` calls, if it takes a block.
    fn scoped_call(&self, call: &Expr) -> Option<(&'static str, &'static str, &'static [Param])> {
        let ExprKind::Call { callee, .. } = &call.kind else {
            return None;
        };
        let ExprKind::Field { base, name, .. } = &callee.kind else {
            return None;
        };
        let ExprKind::Name(id) = &base.kind else {
            return None;
        };
        let def = *self.res.uses.get(&id.span.start)?;
        let module = self.std_modules.get(&def)?;
        TAKES_A_BLOCK
            .iter()
            .find(|(m, n, ..)| m == module && *n == name.name)
            .copied()
    }

    /// `log.span(name)` with no block after it: says how to give one.
    pub(crate) fn needs_a_block(&mut self, module: &str, name: &str, span: Span) -> bool {
        let Some((m, n, params)) = TAKES_A_BLOCK
            .iter()
            .find(|(m, n, ..)| *m == module && *n == name)
        else {
            return false;
        };
        let param = params[0].0;
        self.error(
            span,
            format!("`{m}.{n}` runs a block: write it after the call, `{m}.{n}({param}) {{ … }}` (spec 4.8)"),
        );
        true
    }
}
