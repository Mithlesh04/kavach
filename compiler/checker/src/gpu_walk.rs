//! The walk over a `gpu fn`'s body (spec 15.1): what it may use, with a
//! message naming anything else, and the `gpu fn`s it calls.

use crate::checker::Checker;
use crate::gpu::{WHY, gpu_item};
use crate::ty::Ty;
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};
use std::collections::HashMap;

/// The methods a `gpu fn` may call on its numbers.
const METHODS: [&str; 4] = ["abs", "min", "max", "clamp"];

/// The functions of `std.math` a `gpu fn` may call (spec 15.1).
const MATH: [&str; 5] = ["sqrt", "floor", "ceil", "round", "pow"];

/// The conversions a `gpu fn` may call.
const CONVERSIONS: [&str; 4] = ["Float32", "Int32", "UInt32", "Bool"];

/// Whether `e` names the module `std.math`.
fn is_math(c: &Checker<'_>, e: &Expr) -> bool {
    let ExprKind::Name(id) = &e.kind else {
        return false;
    };
    let def = c.res.uses.get(&id.span.start);
    def.and_then(|d| c.std_modules.get(d))
        .is_some_and(|m| m == "math")
}

/// Why a `gpu fn` can't touch the world outside it (spec 15.1).
const OUTSIDE: &str = "it reaches outside the program (files, the network, the terminal, time, AI), and on the GPU a `gpu fn` runs alone, item by item, with only its own values";

/// Built-ins that reach outside the program.
const PRINTS: [&str; 4] = ["print", "debug", "ask", "env"];

/// Whether `base` names a module or capability that reaches outside the
/// program (`fs`, `net`, `Payments`): all but `math`.
fn reaches_outside(c: &Checker<'_>, base: &Expr) -> bool {
    let ExprKind::Name(id) = &base.kind else {
        return false;
    };
    let Some(&def) = c.res.uses.get(&id.span.start) else {
        return false;
    };
    c.std_modules.get(&def).is_some_and(|m| m != "math")
        || c.res.defs[def].kind == DefKind::Capability
}

/// The call reaching outside the program that `e` is, through `try` and
/// `catch`: `fs.read_text`, `print`.
fn outside_call(c: &Checker<'_>, e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Try { expr, .. } | ExprKind::Catch { expr, .. } => outside_call(c, expr),
        ExprKind::Call { callee, .. } => match &callee.kind {
            ExprKind::Name(id) if PRINTS.contains(&id.name.as_str()) => Some(id.name.clone()),
            ExprKind::Field { base, name, .. } if reaches_outside(c, base) => match &base.kind {
                ExprKind::Name(m) => Some(format!("{}.{}", m.name, name.name)),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

/// A walk over one `gpu fn`'s body, collecting what it may not use.
pub(crate) struct Walk<'t> {
    types: &'t HashMap<(u32, u32), Ty>,
    /// Whether each plain function would fit the GPU (`gpu_fit.rs`), for
    /// the message when one is called.
    fits: &'t crate::gpu_fit::Fits,
    pub(crate) calls: Vec<DefId>,
    pub(crate) errors: Vec<(Span, String)>,
}

impl<'t> Walk<'t> {
    pub(crate) fn new(types: &'t HashMap<(u32, u32), Ty>, fits: &'t crate::gpu_fit::Fits) -> Self {
        Walk {
            types,
            fits,
            calls: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn no(&mut self, span: Span, what: &str) {
        self.errors
            .push((span, format!("a `gpu fn` can't use {what} (spec 15.1)")));
    }

    pub(crate) fn block(&mut self, c: &Checker<'_>, b: &Block) {
        for s in &b.stmts {
            match &s.kind {
                StmtKind::Let { pattern, value, .. } => {
                    if !matches!(pattern.kind, PatternKind::Binding(_)) {
                        self.no(pattern.span, "patterns: name one value");
                    }
                    self.expr(c, value);
                }
                StmtKind::Assign { target, value, .. } => {
                    if !matches!(target.kind, ExprKind::Name(_)) {
                        self.no(target.span, "fields or items: assign to a variable");
                    }
                    self.expr(c, value);
                }
                StmtKind::For {
                    pattern,
                    iter,
                    body,
                    ..
                } => {
                    if !matches!(
                        pattern.kind,
                        PatternKind::Binding(_) | PatternKind::Wildcard
                    ) {
                        self.no(pattern.span, "patterns: name one value");
                    }
                    match &iter.kind {
                        ExprKind::Binary(BinaryOp::Range | BinaryOp::RangeInclusive, l, r) => {
                            self.expr(c, l);
                            self.expr(c, r);
                        }
                        _ => self.no(iter.span, "a loop over anything but a range (`0..n`)"),
                    }
                    self.block(c, body);
                }
                StmtKind::While { cond, body, .. } => {
                    self.expr(c, cond);
                    self.block(c, body);
                }
                StmtKind::Assert(e) => self.no(e.span, "`assert`: a GPU can't stop the program"),
                StmtKind::Expr(e) => self.expr(c, e),
            }
        }
    }

    fn expr(&mut self, c: &Checker<'_>, e: &Expr) {
        // `fs.read_text(p) catch _ => ""`: the call is the reason, not the
        // `String` it gives.
        if let Some(what) = outside_call(c, e) {
            self.no(e.span, &format!("`{what}`: {OUTSIDE}"));
            return;
        }
        if let Some(t) = self.types.get(&(e.span.start, e.span.end))
            && !gpu_item(c, t)
            && !matches!(t, Ty::Unit | Ty::Never | Ty::Error | Ty::Range(_))
        {
            let shown = c.show(t);
            self.errors.push((
                e.span,
                format!(
                    "this is a `{shown}`, and {WHY}: write the type, as `const k: Float32 = 0.5`"
                ),
            ));
            return;
        }
        match &e.kind {
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) => {}
            ExprKind::Break(_) | ExprKind::Continue(_) | ExprKind::Return(None) => {}
            ExprKind::Name(id) => {
                let kind = c.res.uses.get(&id.span.start).map(|d| c.res.defs[*d].kind);
                if !matches!(kind, Some(DefKind::Local { .. } | DefKind::Param)) {
                    self.no(
                        e.span,
                        &format!("`{}` here: only its own parameters and variables", id.name),
                    );
                }
            }
            ExprKind::Unary(UnaryOp::Neg | UnaryOp::Not | UnaryOp::BitNot, x)
            | ExprKind::Return(Some(x)) => self.expr(c, x),
            ExprKind::Binary(op, l, r) => {
                if matches!(
                    op,
                    BinaryOp::Default | BinaryOp::Range | BinaryOp::RangeInclusive
                ) {
                    self.no(e.span, &format!("`{}` here", op.as_str()));
                }
                self.expr(c, l);
                self.expr(c, r);
            }
            ExprKind::Compare(first, links) => {
                self.expr(c, first);
                links.iter().for_each(|(_, x)| self.expr(c, x));
            }
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.expr(c, cond);
                self.block(c, then);
                if let Some(o) = otherwise {
                    self.expr(c, o);
                }
            }
            ExprKind::Block(b) => self.block(c, b),
            // A record's or tuple's fields: read, or made into a new one.
            ExprKind::Field { base, .. } | ExprKind::TupleField { base, .. } => self.expr(c, base),
            ExprKind::Tuple(items) => items.iter().for_each(|x| self.expr(c, x)),
            ExprKind::Call { callee, args, .. } => self.call(c, e, callee, args),
            // A `gpu fn` can't fail: on the GPU there's no error to give
            // back, only a panic (spec 15.1).
            ExprKind::Try { .. } | ExprKind::Catch { .. } | ExprKind::Fail(_) => self.no(
                e.span,
                "`try`, `catch` or `fail`: it can't fail, since on the GPU there is no error to give back. Check the value first with `if`, and give a result for every item",
            ),
            _ => self.no(
                e.span,
                "this: only numbers, `Bool`, records and tuples of them, operators, `if`, loops and calls to other `gpu fn`s",
            ),
        }
    }

    fn call(&mut self, c: &Checker<'_>, e: &Expr, callee: &Expr, args: &[Arg]) {
        let before = self.errors.len();
        self.callee(c, e, callee);
        // A call that isn't allowed says so once, not again for its arguments.
        if self.errors.len() == before {
            for a in args {
                self.expr(c, &a.value);
            }
        }
    }

    fn callee(&mut self, c: &Checker<'_>, e: &Expr, callee: &Expr) {
        match &callee.kind {
            ExprKind::Name(id) => {
                let def = c.res.uses.get(&id.span.start).copied();
                match def {
                    Some(d) if c.gpu_fns.contains(&d) => self.calls.push(d),
                    Some(d)
                        if c.res.defs[d].kind == DefKind::Builtin
                            && CONVERSIONS.contains(&id.name.as_str()) => {}
                    // A record made of its fields (`Particle(x: x, y: y)`).
                    Some(d) if c.res.defs[d].kind == DefKind::Type => {}
                    // A plain function: whether it could be a `gpu fn`.
                    Some(d) if self.fits.contains_key(&d) => {
                        let name = &id.name;
                        let why = match &self.fits[&d] {
                            None => format!(
                                "`{name}`: it isn't a `gpu fn`, but it fits the GPU's rules. Mark it `gpu fn {name}`; it still runs on the CPU when called there"
                            ),
                            Some(reason) => format!(
                                "`{name}`: it isn't a `gpu fn`, and can't be one, since {reason}"
                            ),
                        };
                        self.no(e.span, &why);
                    }
                    _ => self.no(
                        e.span,
                        &format!(
                            "`{}`: it isn't a `gpu fn`, and a `gpu fn` calls only other `gpu fn`s",
                            id.name
                        ),
                    ),
                }
            }
            ExprKind::Field { base, name, .. }
                if is_math(c, base) && MATH.contains(&name.name.as_str()) => {}
            ExprKind::Field { base, name, .. } if METHODS.contains(&name.name.as_str()) => {
                self.expr(c, base)
            }
            _ => self.no(
                e.span,
                "this call: only other `gpu fn`s, conversions, and `abs`, `min`, `max`, `clamp`",
            ),
        }
    }
}
