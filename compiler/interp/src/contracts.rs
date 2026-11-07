//! Contracts while the program runs (spec 7): `require`, `ensure` and
//! record `invariant`s. Split from `interp.rs`.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{Expr, Span, TypeBody};

impl<'a> Interp<'a> {
    /// A `require`, `ensure` or `invariant`: false is a panic that shows the
    /// contract (spec 7.4).
    pub(crate) fn contract(&mut self, kind: &str, e: &'a Expr) -> Eval<'a> {
        match self.eval(e)? {
            Value::Bool(true) => Ok(Value::Unit),
            _ => Err(self.panic(
                format!("contract failed: {kind} {}", self.text(e.span)),
                e.span,
            )),
        }
    }

    /// Checks a record's invariants (spec 7.3).
    pub(crate) fn check_invariants(
        &mut self,
        value: &Value<'a>,
        span: Span,
    ) -> Result<(), Flow<'a>> {
        let Value::Record { def, fields, .. } = value else {
            return Ok(());
        };
        let Some(TypeBody::Record(r)) = self.records.get(def).map(|t| &t.body) else {
            return Ok(());
        };
        if r.invariants.is_empty() {
            return Ok(());
        }
        self.invariant_fields.push(fields.clone());
        let result = r
            .invariants
            .iter()
            .try_for_each(|inv| match self.eval(inv) {
                Ok(Value::Bool(true)) => Ok(()),
                Ok(_) => Err(self.panic(
                    format!("contract failed: invariant {}", self.text(inv.span)),
                    span,
                )),
                Err(flow) => Err(flow),
            });
        self.invariant_fields.pop();
        result
    }
}
