//! `Drop` in native code (spec 6.1). A value that is, or holds, a value
//! of a type that implements `Drop` lives until the end of the block that
//! holds it, not only to its last use. This pass ends such variables
//! there, the last made first, by moving each out where nothing takes
//! it (a value nobody uses is released at once): also before a
//! `return`, `break` or `continue` leaves their blocks, and a function's
//! parameters before it returns. It runs the waiting `drop`s (`DropRun`)
//! at every statement boundary, as the interpreter does
//! (`compiler/interp/src/drops.rs`), so both print in the same order.

use crate::ir::*;
use crate::rt_op::RtOp;
use crate::walk::{Part, parts};
use std::collections::HashSet;

/// The types whose values are, or hold, a value of one of the `records`
/// (the record types that implement `Drop`).
///
/// A function value's type doesn't say what it captured, nor an interface
/// value's which type it holds: so when any closure holds a `Drop` value,
/// every function value counts, and when the program has `Drop` types,
/// every interface value does.
pub fn holders(shapes: &[Shape], records: &[u32]) -> HashSet<Type> {
    let ids = 0..shapes.len() as u32;
    let mut roots: Vec<u32> = records.to_vec();
    if !records.is_empty() {
        roots.extend(
            ids.clone()
                .filter(|i| matches!(shapes[*i as usize], Shape::Iface { .. })),
        );
    }
    let funcs: Vec<u32> = (ids.clone())
        .filter(|i| matches!(shapes[*i as usize], Shape::Func(..)))
        .collect();
    loop {
        let found: HashSet<Type> = (ids.clone())
            .map(Type::Ref)
            .filter(|t| holds(shapes, &roots, *t, &mut HashSet::new()))
            .collect();
        let closures = (ids.clone()).any(|i| {
            matches!(shapes[i as usize], Shape::Closure(_)) && found.contains(&Type::Ref(i))
        });
        if !closures || funcs.iter().all(|f| roots.contains(f)) {
            return found;
        }
        roots.extend(&funcs);
    }
}

fn holds(shapes: &[Shape], records: &[u32], t: Type, seen: &mut HashSet<u32>) -> bool {
    let Type::Ref(id) = t else { return false };
    if records.contains(&id) {
        return true;
    }
    if !seen.insert(id) {
        return false;
    }
    let inner: Vec<Type> = match &shapes[id as usize] {
        Shape::Record { fields, .. } => fields.iter().map(|f| f.ty).collect(),
        Shape::Union { variants, .. } => (variants.iter())
            .flat_map(|v| v.fields.iter().map(|f| f.ty))
            .collect(),
        Shape::Tuple(ts) | Shape::Closure(ts) => ts.clone(),
        Shape::List(t) | Shape::Set(t) | Shape::Optional(t) | Shape::Shared(t) => vec![*t],
        Shape::Map(k, v) => vec![*k, *v],
        _ => Vec::new(),
    };
    inner.into_iter().any(|t| holds(shapes, records, t, seen))
}

/// Ends `f`'s variables that hold `Drop` values at the end of their
/// blocks, and runs waiting `drop`s at each statement boundary when
/// `runs`: when the program has `Drop` types, so `drop`s can be waiting.
pub fn scope_drops(f: &mut Function, holders: &HashSet<Type>, runs: bool) {
    let params: Vec<Local> = (f.params.iter().copied())
        .filter(|p| holders.contains(&f.locals[*p as usize]))
        .collect();
    let mut pass = Pass {
        holders,
        locals: std::mem::take(&mut f.locals),
        scopes: vec![params],
        loops: Vec::new(),
        runs,
    };
    let body = std::mem::take(&mut f.body);
    // The body's block ends the parameters too: they are its outer scope.
    f.body = pass.block(body, Vec::new(), true);
    f.locals = pass.locals;
}

struct Pass<'h> {
    holders: &'h HashSet<Type>,
    locals: Vec<Type>,
    /// The variables holding `Drop` values of each enclosing block,
    /// outermost first; the first is the function's parameters.
    scopes: Vec<Vec<Local>>,
    /// For each enclosing loop: how many scopes are outside its body.
    loops: Vec<usize>,
    /// Whether `drop`s may be waiting, to run at statement boundaries.
    runs: bool,
}

/// Whether `s` leaves its block: `return`, `break`, `continue` or `fail`.
fn leaves(s: &Stmt) -> bool {
    matches!(s, Stmt::Return(_) | Stmt::Break | Stmt::Continue)
        || matches!(
            s,
            Stmt::Expr(Expr {
                kind: ExprKind::Fail(_),
                ..
            })
        )
}

fn run() -> Stmt {
    Stmt::Expr(Expr {
        kind: ExprKind::Rt(RtOp::DropRun, Vec::new()),
        ty: Type::Unit,
        line: 0,
    })
}

impl Pass<'_> {
    fn holds(&self, l: Local) -> bool {
        self.holders.contains(&self.locals[l as usize])
    }

    /// Moves out, so ends, the variables of scopes `from..` (innermost
    /// first, each last made first), then runs their `drop`s.
    fn end_scopes(&self, from: usize, out: &mut Vec<Stmt>) {
        for scope in self.scopes[from..].iter().rev() {
            for &l in scope.iter().rev() {
                let ty = self.locals[l as usize];
                let gone = Expr {
                    kind: ExprKind::Move(l),
                    ty,
                    line: 0,
                };
                out.push(Stmt::Expr(gone));
            }
        }
        if self.runs {
            out.push(run());
        }
    }

    /// `b` with its variables (and `made` ones, a loop's own) ended at its
    /// end. With `outer`, the scope below it (the parameters) ends too.
    fn block(&mut self, b: Block, made: Vec<Local>, outer: bool) -> Block {
        self.scopes.push(made);
        let mut stmts = if self.runs { vec![run()] } else { Vec::new() };
        for s in b.stmts {
            self.stmt(s, &mut stmts);
            // Nothing after a statement that leaves: code after it is
            // never reached, and would give its block a value it can't have.
            if self.runs && !stmts.last().is_some_and(leaves) {
                stmts.push(run());
            }
        }
        // A block that ends in `fail`: it leaves, as a statement would.
        let b_value = match b.value {
            Some(v) if matches!(v.kind, ExprKind::Fail(_)) => {
                self.stmt(Stmt::Expr(*v), &mut stmts);
                None
            }
            other => other,
        };
        let mut value = b_value.map(|mut v| {
            self.expr(&mut v);
            v
        });
        let from = if outer { 0 } else { self.scopes.len() - 1 };
        let ending = self.scopes[from..].iter().any(|s| !s.is_empty());
        if ending {
            // The block's value first: it may read what ends. A value of
            // no type (an `if let` statement's) runs as a statement.
            if let Some(v) = value.take_if(|v| v.ty == Type::Unit) {
                stmts.push(Stmt::Expr(*v));
            }
            if let Some(v) = value.take() {
                let (ty, line) = (v.ty, v.line);
                let keep = self.locals.len() as Local;
                self.locals.push(ty);
                stmts.push(Stmt::Let(keep, *v));
                value = Some(Box::new(Expr {
                    kind: ExprKind::Move(keep),
                    ty,
                    line,
                }));
            }
            self.end_scopes(from, &mut stmts);
        }
        self.scopes.pop();
        if outer {
            self.scopes.clear();
        }
        Block { stmts, value }
    }

    fn stmt(&mut self, s: Stmt, out: &mut Vec<Stmt>) {
        match s {
            Stmt::Let(l, mut e) => {
                self.expr(&mut e);
                out.push(Stmt::Let(l, e));
                if self.holds(l) {
                    self.scopes.last_mut().expect("a block").push(l);
                }
            }
            Stmt::Return(e) => {
                let e = e.map(|mut e| {
                    self.expr(&mut e);
                    e
                });
                let e = e.map(|e| {
                    let (ty, line) = (e.ty, e.line);
                    let keep = self.locals.len() as Local;
                    self.locals.push(ty);
                    out.push(Stmt::Let(keep, e));
                    Expr {
                        kind: ExprKind::Move(keep),
                        ty,
                        line,
                    }
                });
                self.end_scopes(0, out);
                out.push(Stmt::Return(e));
            }
            // `fail` leaves every block too, so the function's `Drop`
            // values end first, the last made first, as before `return`.
            Stmt::Expr(Expr {
                kind: ExprKind::Fail(error),
                ty,
                line,
            }) => {
                let mut error = *error;
                self.expr(&mut error);
                let keep = self.locals.len() as Local;
                self.locals.push(error.ty);
                let error_ty = error.ty;
                out.push(Stmt::Let(keep, error));
                self.end_scopes(0, out);
                let error = Expr {
                    kind: ExprKind::Move(keep),
                    ty: error_ty,
                    line,
                };
                out.push(Stmt::Expr(Expr {
                    kind: ExprKind::Fail(Box::new(error)),
                    ty,
                    line,
                }));
            }
            // A call that can fail: its value ends with the innermost loop's
            // round (or the function), not when the next round overwrites
            // it. Not with its own block, which its value may outlive (a
            // `.timeout` wraps the call in one).
            Stmt::LetResult {
                value,
                error,
                mut call,
            } => {
                self.expr(&mut call);
                out.push(Stmt::LetResult { value, error, call });
                if self.holds(value) {
                    let at = self.loops.last().copied().unwrap_or(1);
                    let at = at.min(self.scopes.len() - 1);
                    self.scopes[at].push(value);
                }
            }
            Stmt::Break | Stmt::Continue => {
                if let Some(&depth) = self.loops.last() {
                    self.end_scopes(depth, out);
                }
                out.push(s);
            }
            Stmt::While { mut cond, body } => {
                self.expr(&mut cond);
                let body = self.loop_body(body, Vec::new());
                out.push(Stmt::While { cond, body });
            }
            Stmt::ForRange {
                var,
                mut start,
                mut end,
                inclusive,
                body,
            } => {
                self.expr(&mut start);
                self.expr(&mut end);
                let body = self.loop_body(body, Vec::new());
                out.push(Stmt::ForRange {
                    var,
                    start,
                    end,
                    inclusive,
                    body,
                });
            }
            Stmt::ForEach {
                var,
                index,
                list,
                body,
            } => {
                // An item that holds a `Drop` value ends with its round.
                let made = if self.holds(var) {
                    vec![var]
                } else {
                    Vec::new()
                };
                let body = self.loop_body(body, made);
                out.push(Stmt::ForEach {
                    var,
                    index,
                    list,
                    body,
                });
            }
            mut other => {
                crate::walk::stmt_parts(&mut other, &mut |p| match p {
                    Part::Expr(x) => self.expr(x),
                    Part::Block(b) => *b = self.block(std::mem::take(b), Vec::new(), false),
                });
                out.push(other);
            }
        }
    }

    fn loop_body(&mut self, body: Block, made: Vec<Local>) -> Block {
        self.loops.push(self.scopes.len());
        let body = self.block(body, made, false);
        self.loops.pop();
        body
    }

    fn expr(&mut self, e: &mut Expr) {
        parts(e, &mut |p| match p {
            Part::Expr(x) => self.expr(x),
            Part::Block(b) => *b = self.block(std::mem::take(b), Vec::new(), false),
        });
    }
}
