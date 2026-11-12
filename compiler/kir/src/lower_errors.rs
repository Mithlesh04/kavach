//! Lowering errors as values (spec 5): calls that can fail, `try`,
//! `catch`, `fail` and `context`.
//!
//! A call that can fail is lowered to [`Stmt::LetResult`], which keeps its
//! value and its error in two locals. The error local is a pointer that
//! is 0 (`none`) when the call succeeded, so testing it is `IsSome`. An
//! error that is a plain number (`-> Int ! Int`) travels boxed, as an
//! optional `Int` does, and is unboxed where `catch` names it.

use crate::ir::{self, Block, ExprKind as K, Shape, Stmt, Type, UnOp};
use crate::lower::{Lowered, Lowerer};
use crate::lower_expr::wrap;
use kavach_ast as ast;
use kavach_checker::Ty;

/// A call that can fail, lowered into locals for its value and its error.
pub(crate) struct Outcome {
    pub(crate) stmt: Stmt,
    pub(crate) value: ir::Local,
    pub(crate) error: ir::Local,
    pub(crate) ok: Type,
    pub(crate) err: Type,
    /// The error's own type when it travels boxed (`held_error`).
    pub(crate) plain: Option<Type>,
}

impl<'a> Lowerer<'a> {
    /// Whether `e` is a call that can fail.
    pub(crate) fn can_fail(&self, e: &ast::Expr) -> bool {
        matches!(
            self.c.expr_types.get(&(e.span.start, e.span.end)),
            Some(Ty::Result(..))
        )
    }

    pub(crate) fn outcome(&mut self, e: &'a ast::Expr) -> Lowered<Outcome> {
        let Some(Ty::Result(_, err)) = self.c.expr_types.get(&(e.span.start, e.span.end)).cloned()
        else {
            return Err(self.unsupported("this call", e.span));
        };
        let own = self.kir_type(&err, e.span)?;
        let err = self.held_error(own);
        let plain = (err != own).then_some(own);
        if let Some(o) = self.cli_parse_outcome(e, err) {
            return o;
        }
        self.inout_pending = None;
        let outer = self.pending_error.take();
        let lowered = self.expr(e)?;
        // A built-in loop given a function that can fail: its own error.
        if let Some(error) = std::mem::replace(&mut self.pending_error, outer) {
            let value = self.new_local(lowered.ty);
            return Ok(Outcome {
                stmt: Stmt::Let(value, lowered),
                value,
                error,
                ok: self.locals[value as usize],
                err,
                plain,
            });
        }
        if let Some(name) = self.unhandled_failing_method(e) {
            return Err(
                self.unsupported(format!("`{name}` given a function that can fail"), e.span)
            );
        }
        let ir::Expr { kind, ty: ok, line } = lowered;
        if let Some(p) = self.inout_pending.take() {
            return self.inout_outcome(p, kind, ok, err, plain, line);
        }
        let (value, error) = (self.new_local(ok), self.new_local(err));
        // `f(try g())`: arguments with statements of their own (a `try`
        // that may leave) are worked out first, in order, into locals,
        // so the call gets plain values.
        let kind = match kind {
            K::Call(f, args) if args.iter().any(|a| matches!(a.kind, K::Block(_))) => {
                let mut stmts = Vec::new();
                let mut plain_args = Vec::new();
                for a in args {
                    let (ty, at) = (a.ty, a.line);
                    let held = self.new_local(ty);
                    stmts.push(Stmt::Let(held, a));
                    plain_args.push(self.local_expr(held, at));
                }
                let call = ir::Expr {
                    kind: K::Call(f, plain_args),
                    ty: ok,
                    line,
                };
                K::Block(Block {
                    stmts,
                    value: Some(Box::new(call)),
                })
            }
            kind => kind,
        };
        let stmt = match kind {
            // A block that ends in the call (a network wait's `.timeout(d)`,
            // spec 14.10): its statements, then the call.
            K::Block(Block {
                mut stmts,
                value: Some(call),
            }) => {
                stmts.push(Stmt::LetResult {
                    value,
                    error,
                    call: *call,
                });
                let all = Block { stmts, value: None };
                Stmt::Expr(ir::Expr {
                    kind: K::Block(all),
                    ty: Type::Unit,
                    line,
                })
            }
            kind => Stmt::LetResult {
                value,
                error,
                call: ir::Expr { kind, ty: ok, line },
            },
        };
        Ok(Outcome {
            stmt,
            value,
            error,
            ok,
            err,
            plain,
        })
    }

    /// Error type `err` as it travels: an object, so a plain number is
    /// boxed as an optional one.
    pub(crate) fn held_error(&mut self, err: Type) -> Type {
        if err.is_ref() {
            err
        } else {
            self.shapes_intern(Shape::Optional(err))
        }
    }

    /// Whether the error in local `error` is set.
    pub(crate) fn failed(&self, error: ir::Local, line: u32) -> ir::Expr {
        ir::Expr {
            kind: K::IsSome(Box::new(self.local_expr(error, line))),
            ty: Type::Bool,
            line,
        }
    }

    pub(crate) fn succeeded(&self, error: ir::Local, line: u32) -> ir::Expr {
        ir::Expr {
            kind: K::Unary(UnOp::Not, Box::new(self.failed(error, line))),
            ty: Type::Bool,
            line,
        }
    }

    /// The success value of an outcome, or nothing when its type is `Unit`.
    fn value_of(&self, o: &Outcome, line: u32) -> Option<Box<ir::Expr>> {
        (o.ok != Type::Unit).then(|| Box::new(self.local_expr(o.value, line)))
    }

    /// `try call` (spec 5.2), optionally with `context "text"` (5.7).
    pub(crate) fn try_expr(
        &mut self,
        call: &'a ast::Expr,
        context: Option<&'a ast::Expr>,
        line: u32,
    ) -> Lowered<ir::Expr> {
        // `try` on what can't fail does nothing (spec 5.2).
        if !self.can_fail(call) {
            return self.expr(call);
        }
        let o = self.outcome(call)?;
        let mut error = self.local_expr(o.error, line);
        if let Some(text) = context {
            let text = self.expr(text)?;
            let ty = self.error_type();
            error = ir::Expr {
                kind: K::Context(Box::new(text), Box::new(error)),
                ty,
                line,
            };
        }
        let fail = ir::Expr {
            kind: K::Fail(Box::new(error)),
            ty: Type::Unit,
            line,
        };
        let check = ir::Expr {
            kind: K::If(
                Box::new(self.failed(o.error, line)),
                wrap(fail, false),
                Block::default(),
            ),
            ty: Type::Unit,
            line,
        };
        let value = self.value_of(&o, line);
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts: vec![o.stmt, Stmt::Expr(check)],
                value,
            }),
            ty: o.ok,
            line,
        })
    }

    /// `call catch handler` or `call catch e => handler` (spec 5.3).
    pub(crate) fn catch_expr(
        &mut self,
        call: &'a ast::Expr,
        binding: Option<&'a ast::Ident>,
        handler: &'a ast::Expr,
        line: u32,
    ) -> Lowered<ir::Expr> {
        if !self.can_fail(call) {
            return self.expr(call);
        }
        let o = self.outcome(call)?;
        let mut on_error = Block::default();
        if let Some(name) = binding.filter(|b| b.name != "_") {
            let bound = self.new_local(o.plain.unwrap_or(o.err));
            self.bind(name, bound);
            let mut error = self.local_expr(o.error, line);
            if let Some(own) = o.plain {
                error = ir::Expr {
                    kind: K::Unwrap(Box::new(error)),
                    ty: own,
                    line,
                };
            }
            on_error.stmts.push(Stmt::Let(bound, error));
        }
        // `catch e => return x` (or `break`, `continue`): the error branch
        // leaves instead of giving a value, as `x ?? return y` does.
        if o.ok == Type::Unit || crate::lower::leaves(handler) {
            self.expr_stmt(handler, &mut on_error.stmts)?;
        } else if matches!(handler.kind, ast::ExprKind::Fail(_)) {
            // `catch e => fail …` leaves the function too: no value.
            let failed = self.expr(handler)?;
            on_error.stmts.push(Stmt::Expr(failed));
        } else {
            let handled = self.expr(handler)?;
            on_error.value = Some(Box::new(self.coerce(handled, o.ok)?));
        }
        let on_success = Block {
            stmts: Vec::new(),
            value: self.value_of(&o, line),
        };
        let choice = ir::Expr {
            kind: K::If(Box::new(self.failed(o.error, line)), on_error, on_success),
            ty: o.ok,
            line,
        };
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts: vec![o.stmt],
                value: Some(Box::new(choice)),
            }),
            ty: o.ok,
            line,
        })
    }

    /// `fail value` (spec 5.1).
    pub(crate) fn fail_expr(&mut self, value: &'a ast::Expr, line: u32) -> Lowered<ir::Expr> {
        let mut error = self.expr(value)?;
        if !error.ty.is_ref() {
            let held = self.held_error(error.ty);
            error = ir::Expr {
                kind: K::Some(Box::new(error)),
                ty: held,
                line,
            };
        }
        Ok(ir::Expr {
            kind: K::Fail(Box::new(error)),
            ty: Type::Unit,
            line,
        })
    }

    /// The error type of `call`, the call in a loop's `try` or `catch`
    /// header (spec 14.3).
    pub(crate) fn read_error_type(&mut self, call: &'a ast::Expr) -> Lowered<Type> {
        let key = (call.span.start, call.span.end);
        let Some(Ty::Result(_, err)) = self.c.expr_types.get(&key).cloned() else {
            return Err(self.unsupported("this call", call.span));
        };
        self.kir_type(&err, call.span)
    }

    /// What a failed read does in a `for` loop whose header `iter` is
    /// `try` or `catch` around `fs.lines` or `fs.chunks` (spec 14.3): `try`
    /// fails with the error in local `error` (and its `context`); `catch`
    /// runs its handler with it, then the loop ends.
    pub(crate) fn header_failed(
        &mut self,
        iter: &'a ast::Expr,
        error: ir::Local,
        err: Type,
        line: u32,
    ) -> Lowered<Block> {
        let mut b = Block::default();
        match &iter.kind {
            ast::ExprKind::Try { context, .. } => {
                let mut e = self.local_expr(error, line);
                if let Some(text) = context {
                    let text = self.expr(text)?;
                    let ty = self.error_type();
                    e = ir::Expr {
                        kind: K::Context(Box::new(text), Box::new(e)),
                        ty,
                        line,
                    };
                }
                let fail = ir::Expr {
                    kind: K::Fail(Box::new(e)),
                    ty: Type::Unit,
                    line,
                };
                b.stmts.push(Stmt::Expr(fail));
            }
            ast::ExprKind::Catch {
                binding, handler, ..
            } => {
                if let Some(name) = binding.as_ref().filter(|b| b.name != "_") {
                    let bound = self.new_local(err);
                    self.bind(name, bound);
                    b.stmts.push(Stmt::Let(bound, self.local_expr(error, line)));
                }
                // `break` and `continue` there end this loop, as the
                // interpreter does.
                if !matches!(
                    handler.kind,
                    ast::ExprKind::Break(_) | ast::ExprKind::Continue(_)
                ) {
                    self.expr_stmt(handler, &mut b.stmts)?;
                }
                b.stmts.push(Stmt::Break);
            }
            _ => {}
        }
        Ok(b)
    }
}
