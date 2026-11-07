//! Code that always breaks a contract (spec 7.7): a call's arguments are
//! known, and a `require` that compares a parameter with a number is false
//! for them; or a value a function returns is known, and an `ensure` that
//! compares `result` with a number is false for it.

use super::Known;
use kavach_ast::visit::{VisitMut, walk_expr};
use kavach_ast::*;
use num_bigint::BigInt;
use std::collections::HashMap;

/// A number the compiler knows: whole numbers exactly, floats as written.
#[derive(Clone, PartialEq, PartialOrd)]
enum Number {
    Whole(BigInt),
    Float(f64),
}

impl Number {
    fn float(&self) -> f64 {
        match self {
            Number::Whole(n) => n.to_string().parse().unwrap_or(f64::NAN),
            Number::Float(f) => *f,
        }
    }

    /// `a op b`, exact for two whole numbers.
    fn compare(&self, other: &Number) -> std::cmp::Ordering {
        match (self, other) {
            (Number::Whole(a), Number::Whole(b)) => a.cmp(b),
            _ => self
                .float()
                .partial_cmp(&other.float())
                .unwrap_or(std::cmp::Ordering::Equal),
        }
    }
}

impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Whole(n) => write!(f, "{n}"),
            Number::Float(x) => write!(f, "{x:?}"),
        }
    }
}

/// A float literal, or a minus and one.
fn float_literal(e: &Expr) -> Option<f64> {
    match &e.kind {
        ExprKind::Float(text) => text.replace('_', "").parse().ok(),
        ExprKind::Unary(UnaryOp::Neg, x) => float_literal(x).map(|f| -f),
        _ => None,
    }
}

/// What a function's calls are checked against: its name, its parameters'
/// names in order, and its `require`s.
pub(super) struct Contract {
    name: String,
    params: Vec<String>,
    requires: Vec<Expr>,
}

/// The contracts of the file's functions with a `require`, by definition.
pub(super) fn contracts(
    k: &Known<'_, '_>,
    file: &File,
) -> HashMap<kavach_resolver::DefId, Contract> {
    let mut out = HashMap::new();
    for item in &file.items {
        let ItemKind::Fn(f) = &item.kind else {
            continue;
        };
        if f.requires.is_empty() {
            continue;
        }
        if let Some(d) = k.checker.decl.get(&f.name.span.start) {
            out.insert(
                *d,
                Contract {
                    name: f.name.name.clone(),
                    params: f.params.iter().map(|p| p.name.name.clone()).collect(),
                    requires: f.requires.clone(),
                },
            );
        }
    }
    out
}

/// The file's functions with an `inout` parameter, by definition.
pub(super) fn inout_fns(
    k: &Known<'_, '_>,
    file: &File,
) -> std::collections::HashSet<kavach_resolver::DefId> {
    let fns = file.items.iter().filter_map(|item| match &item.kind {
        ItemKind::Fn(f) if f.params.iter().any(|p| p.is_inout) => {
            k.checker.decl.get(&f.name.span.start).copied()
        }
        _ => None,
    });
    fns.collect()
}

/// A known number: a whole number's value, or a float literal.
fn number(k: &Known<'_, '_>, e: &Expr) -> Option<Number> {
    k.value(e)
        .map(Number::Whole)
        .or_else(|| float_literal(e).map(Number::Float))
}

/// The values of `return`s in what it visits, not of lambdas.
struct Returns(Vec<Expr>);

impl VisitMut for Returns {
    fn expr(&mut self, e: &mut Expr) {
        match &e.kind {
            ExprKind::Lambda { .. } | ExprKind::DotLambda(_) => {}
            ExprKind::Return(Some(v)) => {
                self.0.push((**v).clone());
                walk_expr(self, e);
            }
            _ => walk_expr(self, e),
        }
    }
}

/// What `e`, at the end of a function, gives: each branch's last value.
fn last_values(e: &Expr, out: &mut Vec<Expr>) {
    match &e.kind {
        ExprKind::If {
            then, otherwise, ..
        }
        | ExprKind::IfLet {
            then, otherwise, ..
        } => {
            last_of_block(then, out);
            if let Some(o) = otherwise {
                last_values(o, out);
            }
        }
        ExprKind::Block(b) => last_of_block(b, out),
        ExprKind::Match { arms, .. } => arms.iter().for_each(|a| last_values(&a.body, out)),
        _ => out.push(e.clone()),
    }
}

fn last_of_block(b: &Block, out: &mut Vec<Expr>) {
    if let Some(Stmt {
        kind: StmtKind::Expr(e),
        ..
    }) = b.stmts.last()
    {
        last_values(e, out);
    }
}

impl Known<'_, '_> {
    /// Each known value a function returns, against its `ensure`s. The
    /// `const`s it reads are known by now; a `let` isn't.
    pub(super) fn results_against_ensures(&mut self, file: &File) {
        for item in &file.items {
            let ItemKind::Fn(f) = &item.kind else {
                continue;
            };
            let (false, Some(body)) = (f.ensures.is_empty(), &f.body) else {
                continue;
            };
            let mut parts = Vec::new();
            for e in &f.ensures {
                conjuncts(e, &mut parts);
            }
            let mut returns = Returns(Vec::new());
            for s in &mut body.clone().stmts {
                returns.stmt(s);
            }
            let mut results = returns.0;
            last_of_block(body, &mut results);
            for r in &results {
                let Some(v) = number(self, r) else { continue };
                let known = HashMap::from([("result", v.clone())]);
                if let Some(part) = parts
                    .iter()
                    .find(|p| matches!(holds(p, &known), Some((false, _))))
                {
                    self.errors.push((
                        r.span,
                        format!(
                            "this always breaks `ensure {}` of `{}`: `result` is {v} (spec 7.7)",
                            show(part),
                            f.name.name
                        ),
                    ));
                }
            }
        }
    }

    pub(super) fn call(&mut self, e: &Expr, callee: &Expr, args: &[Arg]) {
        let ExprKind::Name(f) = &callee.kind else {
            return;
        };
        let Some(c) = self
            .checker
            .res
            .uses
            .get(&f.span.start)
            .and_then(|d| self.contracts.get(d))
        else {
            return;
        };
        // Each parameter's known value: positional arguments in order,
        // then named ones by name.
        let mut known: HashMap<&str, Number> = HashMap::new();
        let positional = args.iter().filter(|a| a.name.is_none());
        for (name, a) in c.params.iter().zip(positional) {
            if let Some(v) = number(self, &a.value) {
                known.insert(name, v);
            }
        }
        for a in args {
            if let (Some(n), Some(v)) = (&a.name, number(self, &a.value)) {
                known.insert(n.name.as_str(), v);
            }
        }
        let mut parts = Vec::new();
        for r in &c.requires {
            conjuncts(r, &mut parts);
        }
        for part in parts {
            if let Some((false, names)) = holds(part, &known) {
                let shown: Vec<String> = names
                    .iter()
                    .map(|n| format!("`{n}` is {}", known[n.as_str()]))
                    .collect();
                let message = format!(
                    "this call always breaks `require {}` of `{}`: {} (spec 7.7)",
                    show(part),
                    c.name,
                    shown.join(", ")
                );
                self.errors.push((e.span, message));
                return;
            }
        }
    }
}

/// A comparison of names and numbers, as written.
fn show(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Name(id) => id.name.clone(),
        ExprKind::Int(n) => n.to_string(),
        ExprKind::Float(text) => text.clone(),
        ExprKind::Unary(UnaryOp::Neg, x) => format!("-{}", show(x)),
        ExprKind::Binary(op, l, r) => {
            let op = match op {
                BinaryOp::Lt => "<",
                BinaryOp::Le => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::Ge => ">=",
                BinaryOp::Eq => "==",
                BinaryOp::NotEq => "!=",
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                _ => "?",
            };
            format!("{} {op} {}", show(l), show(r))
        }
        _ => "…".to_string(),
    }
}

/// The parts of `e` joined with `and`.
fn conjuncts<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
    match &e.kind {
        ExprKind::Binary(BinaryOp::And, l, r) => {
            conjuncts(l, out);
            conjuncts(r, out);
        }
        _ => out.push(e),
    }
}

/// Whether comparison `e` holds for the known parameters, with the ones it
/// read; `None` when it isn't a comparison of known numbers.
fn holds(e: &Expr, known: &HashMap<&str, Number>) -> Option<(bool, Vec<String>)> {
    let ExprKind::Binary(op, l, r) = &e.kind else {
        return None;
    };
    let mut names = Vec::new();
    let mut side = |x: &Expr| -> Option<Number> {
        match &x.kind {
            ExprKind::Name(id) => {
                let v = known.get(id.name.as_str())?.clone();
                names.push(id.name.clone());
                Some(v)
            }
            _ => crate::constants::const_int(x)
                .map(Number::Whole)
                .or_else(|| float_literal(x).map(Number::Float)),
        }
    };
    let (a, b) = (side(l)?, side(r)?);
    if names.is_empty() {
        return None;
    }
    use std::cmp::Ordering::*;
    let order = a.compare(&b);
    let ok = match op {
        BinaryOp::Lt => order == Less,
        BinaryOp::Le => order != Greater,
        BinaryOp::Gt => order == Greater,
        BinaryOp::Ge => order != Less,
        BinaryOp::Eq => order == Equal,
        BinaryOp::NotEq => order != Equal,
        _ => return None,
    };
    Some((ok, names))
}
