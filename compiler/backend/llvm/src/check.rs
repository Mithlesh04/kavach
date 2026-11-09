//! What the LLVM backend supports so far. A program that uses anything
//! else is built with Cranelift instead (decision 0018), so this list only
//! ever grows.
//!
//! Every type is supported, and every expression and statement but a
//! call that can fail outside `try` (lowering never makes one) and a
//! function value whose function can fail.

use kavach_kir::{Block, Expr, ExprKind, Function, PrintPart, Program, Stmt, Type};

/// The first thing in `program` the LLVM backend can't compile yet.
pub(crate) fn unsupported(program: &Program) -> Option<String> {
    let c = Check {
        fails: program.functions.iter().map(|f| f.fails).collect(),
    };
    program.functions.iter().find_map(|f| c.function(f))
}

struct Check {
    /// Whether each function can fail.
    fails: Vec<bool>,
}

impl Check {
    fn function(&self, f: &Function) -> Option<String> {
        self.block(&f.body)
    }

    fn block(&self, b: &Block) -> Option<String> {
        b.stmts
            .iter()
            .find_map(|s| self.stmt(s))
            .or_else(|| b.value.as_deref().and_then(|v| self.expr(v)))
    }

    fn stmt(&self, s: &Stmt) -> Option<String> {
        match s {
            Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => self.expr(e),
            Stmt::Return(None) | Stmt::Break | Stmt::Continue => None,
            Stmt::While { cond, body } => self.expr(cond).or_else(|| self.block(body)),
            Stmt::ForRange {
                start, end, body, ..
            } => self
                .expr(start)
                .or_else(|| self.expr(end))
                .or_else(|| self.block(body)),
            Stmt::Print(parts) => parts.iter().find_map(|p| match p {
                PrintPart::Text(_) => None,
                PrintPart::Value(e) => self.expr(e),
            }),
            Stmt::Check { cond, .. } => self.expr(cond),
            Stmt::ForEach { body, .. } => self.block(body),
            Stmt::LetResult { call, .. } => match &call.kind {
                ExprKind::Call(_, args) => args.iter().find_map(|a| self.expr(a)),
                ExprKind::CallValue(callee, args) => self
                    .expr(callee)
                    .or_else(|| args.iter().find_map(|a| self.expr(a))),
                ExprKind::Parse(x) => self.expr(x),
                ExprKind::Rt(op, args) if op.can_fail() => args.iter().find_map(|a| self.expr(a)),
                _ => self.expr(call),
            },
        }
    }

    fn expr(&self, e: &Expr) -> Option<String> {
        let all = |xs: &[Expr]| xs.iter().find_map(|x| self.expr(x));
        match &e.kind {
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::Char(_)
            | ExprKind::Local(_)
            | ExprKind::Move(_)
            | ExprKind::Panic(_)
            | ExprKind::None => None,
            // A call that can fail goes through `LetResult` (above).
            ExprKind::Call(f, _) if self.fails[*f as usize] => {
                Some("a call that can fail, outside `try`".into())
            }
            ExprKind::Some(x)
            | ExprKind::IsSome(x)
            | ExprKind::Unwrap(x)
            | ExprKind::HasVariant(x, _)
            | ExprKind::Fail(x) => self.expr(x),
            ExprKind::Context(a, b) => self.expr(a).or_else(|| self.expr(b)),
            ExprKind::Closure { captures, .. } => all(captures),
            ExprKind::CallValue(callee, args) => self.expr(callee).or_else(|| all(args)),
            ExprKind::Assign(_, x) => self.expr(x),
            ExprKind::Unary(_, x) | ExprKind::Convert(x, _) if x.ty == Type::Unit => {
                Some("operators on nothing".into())
            }
            ExprKind::Unary(_, x) | ExprKind::Convert(x, _) => self.expr(x),
            ExprKind::Str(_) => None,
            ExprKind::Format(parts) => all(parts),
            ExprKind::Binary(_, a, b)
            | ExprKind::Fast(_, a, b)
            | ExprKind::And(a, b)
            | ExprKind::Or(a, b) => self.expr(a).or_else(|| self.expr(b)),
            ExprKind::Call(_, args)
            | ExprKind::List(args)
            | ExprKind::Tuple(args)
            | ExprKind::Record { fields: args, .. } => all(args),
            ExprKind::GetField(x, _) => self.expr(x),
            ExprKind::SetField(x, _, v) => self.expr(x).or_else(|| self.expr(v)),
            ExprKind::Index(x, i) | ExprKind::InRange(x, i) if x.ty.is_ref() => {
                self.expr(x).or_else(|| self.expr(i))
            }
            ExprKind::Rt(op, args) if !op.can_fail() => all(args),
            ExprKind::Decimal(_) => None,
            ExprKind::Map(pairs) => pairs
                .iter()
                .find_map(|(k, v)| self.expr(k).or_else(|| self.expr(v))),
            ExprKind::If(c, t, o) => self
                .expr(c)
                .or_else(|| self.block(t))
                .or_else(|| self.block(o)),
            ExprKind::Block(b) => self.block(b),
            ExprKind::Fallback(fast, slow) => self.block(fast).or_else(|| self.block(slow)),
            _ => Some("this expression".into()),
        }
    }
}
