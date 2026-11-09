//! An `else if` that can't be true (spec 4.7): its condition compares a
//! variable with a literal, and an earlier condition of the same chain is
//! true whenever it is (`x > 20` after `x > 10`), so that branch ran.

use super::{NeverRuns, literal};
use kavach_ast::*;
use kavach_resolver::DefId;

/// `name op value`: a variable compared with a literal.
struct Fact {
    var: DefId,
    name: String,
    op: BinaryOp,
    value: Value,
}

#[derive(PartialEq)]
enum Value {
    Int(i128),
    /// Text, a character or `true`/`false`, as written: only `==` and
    /// `!=` say anything about them.
    Other(String),
}

impl NeverRuns<'_> {
    /// The `if` at `e` and its `else if`s.
    pub(super) fn chain(&mut self, e: &Expr) {
        let mut facts: Vec<Fact> = Vec::new();
        let mut at = e;
        while let ExprKind::If {
            cond, otherwise, ..
        } = &at.kind
        {
            if let Some(f) = self.fact(cond) {
                let earlier = facts.iter().find(|g| implies(&f, g));
                if let (Some(g), true) = (earlier, self.seen.insert(cond.span.start)) {
                    let (f, g) = (show(&f), show(g));
                    let message = match f == g {
                        true => format!(
                            "this condition is never true here: the same condition, `{f}`, is taken above. Remove this branch, or fix one of them (spec 4.7)"
                        ),
                        false => format!(
                            "this condition is never true here: `{f}` means `{g}`, and that branch is taken above. Check the order of the conditions (spec 4.7)"
                        ),
                    };
                    self.found.push((cond.span, message));
                }
                facts.push(f);
            }
            match otherwise {
                Some(next) => at = next,
                None => break,
            }
        }
    }

    fn fact(&self, cond: &Expr) -> Option<Fact> {
        let ExprKind::Binary(op, a, b) = &cond.kind else {
            return None;
        };
        let (x, lit, op) = match (&a.kind, &b.kind) {
            (ExprKind::Name(x), _) => (x, &**b, *op),
            (_, ExprKind::Name(x)) => (x, &**a, flip(*op)),
            _ => return None,
        };
        let var = *self.res.uses.get(&x.span.start)?;
        let value = match &lit.kind {
            ExprKind::Int(v) => Value::Int(i128::from(*v)),
            ExprKind::Unary(UnaryOp::Neg, inner) => match inner.kind {
                ExprKind::Int(v) => Value::Int(-i128::from(v)),
                _ => return None,
            },
            _ => Value::Other(literal(lit)?),
        };
        let ordered = matches!(
            op,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        );
        if !(ordered && matches!(value, Value::Int(_))
            || matches!(op, BinaryOp::Eq | BinaryOp::NotEq))
        {
            return None;
        }
        Some(Fact {
            var,
            name: x.name.clone(),
            op,
            value,
        })
    }
}

/// `a op x` as `x op' a`.
fn flip(op: BinaryOp) -> BinaryOp {
    match op {
        BinaryOp::Lt => BinaryOp::Gt,
        BinaryOp::Le => BinaryOp::Ge,
        BinaryOp::Gt => BinaryOp::Lt,
        BinaryOp::Ge => BinaryOp::Le,
        other => other,
    }
}

/// Whether `g` holds whenever `f` does, for the same variable.
fn implies(f: &Fact, g: &Fact) -> bool {
    if f.var != g.var {
        return false;
    }
    use BinaryOp::*;
    match (&f.value, &g.value) {
        (Value::Int(a), Value::Int(b)) => {
            let (a, b) = (*a, *b);
            match (f.op, g.op) {
                // `x == a`: `g` holds for `a` itself.
                (Eq, _) => holds(g.op, a, b),
                (Gt, Gt) | (Ge, Ge) | (Lt, Lt) | (Le, Le) => match f.op {
                    Gt | Ge => a >= b,
                    _ => a <= b,
                },
                (Gt, Ge) => a + 1 >= b,
                (Ge, Gt) => a > b,
                (Lt, Le) => a - 1 <= b,
                (Le, Lt) => a < b,
                (Gt, NotEq) => b <= a,
                (Ge, NotEq) => b < a,
                (Lt, NotEq) => b >= a,
                (Le, NotEq) => b > a,
                (NotEq, NotEq) => a == b,
                _ => false,
            }
        }
        (Value::Other(a), Value::Other(b)) => match (f.op, g.op) {
            (Eq, Eq) | (NotEq, NotEq) => a == b,
            (Eq, NotEq) => a != b,
            _ => false,
        },
        _ => false,
    }
}

/// Whether `a op b` is true.
fn holds(op: BinaryOp, a: i128, b: i128) -> bool {
    match op {
        BinaryOp::Eq => a == b,
        BinaryOp::NotEq => a != b,
        BinaryOp::Lt => a < b,
        BinaryOp::Le => a <= b,
        BinaryOp::Gt => a > b,
        BinaryOp::Ge => a >= b,
        _ => false,
    }
}

fn show(f: &Fact) -> String {
    let op = match f.op {
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
        _ => "?",
    };
    let value = match &f.value {
        Value::Int(v) => v.to_string(),
        Value::Other(s) => s.clone(),
    };
    format!("{} {op} {value}", f.name)
}
