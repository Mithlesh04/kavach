//! Expressions.

use crate::checker::Checker;
use crate::infer::VarKind;
use crate::ty::{Prim, Ty};
use kavach_ast::*;

const BOOL: Ty = Ty::Prim(Prim::Bool);
const STRING: Ty = Ty::Prim(Prim::String);

impl Checker<'_> {
    /// The type of `e`, which must not be an unhandled failure (spec 5).
    pub(crate) fn value(&mut self, e: &Expr, expected: Option<&Ty>) -> Ty {
        match self.infer(e, expected) {
            Ty::Result(ok, _) if self.in_example => *ok,
            Ty::Result(ok, _) => {
                let what = self.describe(e);
                self.error(
                    e.span,
                    format!("{what} can fail: handle the error with `try`, `catch` or `match`"),
                );
                *ok
            }
            t => t,
        }
    }

    /// The type of `e`; a call that can fail gives `Ty::Result`.
    pub(crate) fn infer(&mut self, e: &Expr, expected: Option<&Ty>) -> Ty {
        let t = self.infer_kind(e, expected);
        self.expr_types.push((e.span, t.clone()));
        t
    }

    fn infer_kind(&mut self, e: &Expr, expected: Option<&Ty>) -> Ty {
        // Where a `Json` is expected, a literal becomes one (spec 2.12).
        if let Some(json) = expected.filter(|t| self.is_json(t))
            && crate::json::json_literal_kind(e)
        {
            let json = self.inf.resolve(json);
            return self.json_literal(e, &json);
        }
        if let Some(t) = self.big_constant(e, expected) {
            return t;
        }
        self.note_exact_constant(e);
        match &e.kind {
            ExprKind::Tagged { tag, args } => self.tagged(tag, args),
            ExprKind::BigInt(_) => Ty::Prim(Prim::BigInt),
            ExprKind::Int(_) | ExprKind::Float(_) => {
                let kind = if matches!(e.kind, ExprKind::Int(_)) {
                    VarKind::IntLiteral
                } else {
                    VarKind::FloatLiteral
                };
                let t = self.inf.fresh(kind);
                self.literals.push((e.span.start, t.clone()));
                t
            }
            ExprKind::Char(_) => Ty::Prim(Prim::Char),
            ExprKind::Bool(_) => BOOL,
            ExprKind::Str(parts) => {
                for part in parts {
                    if let StrPart::Expr(x) = part {
                        self.value(x, None);
                        self.check_secret(x, "text");
                    }
                }
                STRING
            }
            ExprKind::None => match expected.map(|t| self.inf.shallow(t)) {
                Some(t @ Ty::Optional(_)) => t,
                _ => Ty::Optional(Box::new(self.fresh())),
            },
            ExprKind::Name(id) => self.name(id, expected),
            ExprKind::SelfValue => self
                .ctx
                .as_ref()
                .and_then(|c| c.self_ty.clone())
                .unwrap_or(Ty::Error),
            ExprKind::It => self.its.last().cloned().unwrap_or(Ty::Error),
            ExprKind::List(items) => {
                let item = match expected.map(|t| self.inf.shallow(t)) {
                    Some(Ty::List(t)) => *t,
                    _ => self.fresh(),
                };
                for x in items {
                    let actual = self.value(x, Some(&item));
                    if self.coerce(&actual, &item) {
                        continue;
                    }
                    // `[2, 3, [4, 5]]`: a list holds one type.
                    let lists = |t: &Ty| matches!(t, Ty::List(_));
                    let (want, got) = (self.inf.resolve(&item), self.inf.resolve(&actual));
                    if lists(&want) != lists(&got) {
                        let (want, got) = (self.show(&want), self.show(&got));
                        self.error(
                            x.span,
                            format!("expected `{want}`, found `{got}`: a list holds one type; to mix numbers and lists, use a union (2.6) or `Json` (2.12)"),
                        );
                    } else {
                        self.mismatch(x, &item, &actual);
                    }
                }
                Ty::List(Box::new(item))
            }
            ExprKind::Map(entries) => {
                let (k, v) = (self.fresh(), self.fresh());
                for (key, value) in entries {
                    self.check_expect(key, &k);
                    self.check_expect(value, &v);
                }
                Ty::Map(Box::new(k), Box::new(v))
            }
            ExprKind::Tuple(items) => {
                Ty::Tuple(items.iter().map(|x| self.value(x, None)).collect())
            }
            ExprKind::AnonRecord(fields) => {
                // Where a record of known fields is expected, each field is
                // checked against its type, so a `T` fills a `T?` (2.7).
                let wanted = match expected.map(|t| self.inf.resolve(t)) {
                    Some(Ty::Anon(fs)) => fs,
                    _ => Vec::new(),
                };
                let mut fs: Vec<(String, Ty)> = Vec::new();
                for (name, x) in fields {
                    let ty = match wanted.iter().find(|(n, _)| *n == name.name) {
                        Some((_, want)) => {
                            self.check_expect(x, want);
                            want.clone()
                        }
                        None => self.value(x, None),
                    };
                    if fs.iter().any(|(n, _)| *n == name.name) {
                        self.error(
                            name.span,
                            format!("the field `{}` is written twice", name.name),
                        );
                        continue;
                    }
                    fs.push((name.name.clone(), ty));
                }
                fs.sort_by(|a, b| a.0.cmp(&b.0));
                Ty::Anon(fs)
            }
            ExprKind::Unary(op, x) => self.unary(*op, x),
            ExprKind::Binary(op, l, r) => self.binary(*op, l, r),
            ExprKind::Compare(first, links) => self.chain(first, links),
            ExprKind::Call {
                callee,
                generics,
                args,
            } => self.call(e, callee, generics, args, expected),
            ExprKind::Field {
                base,
                name,
                optional,
            } => self.field(e, base, name, *optional),
            ExprKind::TupleField { base, index } => {
                let t = self.value(base, None);
                match self.inf.resolve(&t) {
                    Ty::Tuple(ts) if (*index as usize) < ts.len() => ts[*index as usize].clone(),
                    Ty::Error | Ty::Var(_) => Ty::Error,
                    other => {
                        let shown = self.show(&other);
                        self.error(e.span, format!("`{shown}` has no element {index}"));
                        Ty::Error
                    }
                }
            }
            ExprKind::Index { base, index } => self.index(base, index),
            ExprKind::With { base, fields } => self.with(base, fields),
            ExprKind::If { .. } | ExprKind::IfLet { .. } | ExprKind::Match { .. } => {
                self.branching(e, expected)
            }
            ExprKind::Block(b) => self.block_value(b, expected),
            ExprKind::Lambda { params, body } => self.lambda(params, body, expected),
            ExprKind::DotLambda(body) => {
                let (param, ret) = match expected.map(|t| self.inf.shallow(t)) {
                    Some(Ty::Fn(ps, r, _)) if ps.len() == 1 => (ps[0].clone(), Some(*r)),
                    _ => (self.fresh(), None),
                };
                // `.amount` can't fail, whatever the wanted type allows.
                let ret = ret.map(|r| match self.inf.shallow(&r) {
                    Ty::Result(ok, _) => *ok,
                    r => r,
                });
                self.its.push(param.clone());
                self.lambdas.push(body.span);
                let t = match &ret {
                    Some(r) => self.check_expect(body, r),
                    None => self.value(body, None),
                };
                self.lambdas.pop();
                self.its.pop();
                Ty::Fn(vec![param], Box::new(t), None)
            }
            ExprKind::Try { expr, context } => self.try_expr(expr, context.as_deref()),
            ExprKind::Catch {
                expr,
                binding,
                handler,
            } => self.catch_expr(expr, binding.as_ref(), handler, expected),
            ExprKind::Fail(x) => self.fail(e, x),
            ExprKind::Return(value) => self.return_expr(e, value.as_deref()),
            ExprKind::Break(label) => {
                self.check_jump(label, "break", e.span);
                Ty::Never
            }
            ExprKind::Continue(label) => {
                self.check_jump(label, "continue", e.span);
                Ty::Never
            }
            // The spawned code may fail: its error comes out of `.wait()`
            // (spec 9.1).
            ExprKind::Spawn(kind, x, within) => {
                if let Some(d) = within {
                    let duration = self.builtin_ty("Duration");
                    self.check_expect(d, &duration);
                }
                self.copies.push((x.span.start, "task"));
                let outer_loops = std::mem::take(&mut self.loops);
                let t = self.infer(x, None);
                self.loops = outer_loops;
                self.copies.pop();
                if *kind == kavach_ast::SpawnKind::Process {
                    self.check_process_call(x, &t);
                }
                let t = match t {
                    Ty::Result(ok, _) => *ok,
                    t => t,
                };
                Ty::Builtin("Task", vec![t])
            }
            ExprKind::Arena(b) => {
                self.arenas.push(b.span);
                let t = self.block_value(b, expected);
                self.arenas.pop();
                t
            }
            ExprKind::Unsafe(b) => {
                self.check_unsafe(e.span);
                self.in_unsafe += 1;
                let t = self.block_value(b, expected);
                self.in_unsafe -= 1;
                t
            }
            ExprKind::Error => Ty::Error,
        }
    }

    /// A variant written without its union's name: always an error, which
    /// says what to write (decision 0070).
    pub(crate) fn bare_variant(&mut self, id: &Ident, of: kavach_resolver::DefId) {
        let union = self.def_name(of).to_string();
        self.error(
            id.span,
            format!(
                "`{}` is a variant of `{union}`: write `{union}.{}` (decision 0070)",
                id.name, id.name
            ),
        );
    }

    fn unary(&mut self, op: UnaryOp, x: &Expr) -> Ty {
        match op {
            UnaryOp::Not => self.check_expect(x, &BOOL),
            UnaryOp::Deref => self.deref(x),
            UnaryOp::Neg | UnaryOp::BitNot => {
                let t = self.value(x, None);
                if self.require_number(x, &t, op == UnaryOp::BitNot) {
                    t
                } else {
                    Ty::Error
                }
            }
        }
    }

    pub(crate) fn require_number(&mut self, e: &Expr, t: &Ty, int_only: bool) -> bool {
        let ok = match self.inf.resolve(t) {
            Ty::Prim(p) => {
                if int_only {
                    p.is_int()
                } else {
                    p.is_numeric()
                }
            }
            Ty::Var(v) => !int_only || self.inf.kind(v) != VarKind::FloatLiteral,
            Ty::Error | Ty::Never => true,
            _ => false,
        };
        if !ok {
            let shown = self.show(t);
            let what = if int_only { "an integer" } else { "a number" };
            // A value that may be missing: say how to get the number out.
            let hint = match self.inf.resolve(t) {
                Ty::Optional(_) => {
                    ": it may be `none`, so give a default with `?? 0`, or check it with `if let` (spec 2.7)"
                }
                // `xs + ys`: lists are joined by pushing into a copy.
                Ty::List(_) => {
                    ": `+` doesn't join lists; copy one into a `let` (`let all = xs`) and `push` the other's items in a `for` loop (spec 2.2)"
                }
                _ => "",
            };
            self.error(e.span, format!("expected {what}, found `{shown}`{hint}"));
        }
        ok
    }

    fn binary(&mut self, op: BinaryOp, l: &Expr, r: &Expr) -> Ty {
        use BinaryOp::*;
        match op {
            And | Or => {
                self.check_expect(l, &BOOL);
                self.check_expect(r, &BOOL);
                BOOL
            }
            Eq | NotEq | Lt | Le | Gt | Ge => {
                let lt = self.value(l, None);
                self.check_expect(r, &lt);
                self.check_compare(op, l, r, &lt);
                if !matches!(op, Eq | NotEq) {
                    let what = format!("`{}`", op.as_str());
                    self.require_order(&lt, l.span, &what);
                }
                BOOL
            }
            Default => {
                let lt = self.value(l, None);
                match self.inf.shallow(&lt) {
                    Ty::Optional(inner) => {
                        self.check_expect(r, &inner);
                        *inner
                    }
                    Ty::Var(v) if self.inf.kind(v) == VarKind::Any => self.value(r, None),
                    Ty::Error => self.value(r, None),
                    other => {
                        let shown = self.show(&other);
                        self.error(l.span, format!("`??` needs an optional value on the left, but this is `{shown}`, which is never `none`"));
                        self.value(r, None)
                    }
                }
            }
            Range | RangeInclusive => {
                let lt = self.value(l, None);
                if !self.require_number(l, &lt, true) {
                    self.value(r, None);
                    return Ty::Error;
                }
                self.check_expect(r, &lt);
                Ty::Range(Box::new(lt))
            }
            _ => {
                let lt = self.value(l, None);
                if let Some(t) = self.ptr_offset(op, &lt, r, l.span) {
                    return t;
                }
                if op == Add && matches!(self.inf.resolve(&lt), Ty::Prim(Prim::Bytes)) {
                    let bytes = Ty::Prim(Prim::Bytes);
                    self.check_expect(r, &bytes);
                    return bytes;
                }
                if op == Add && matches!(self.inf.resolve(&lt), Ty::Prim(Prim::String)) {
                    self.check_expect(r, &STRING);
                    return STRING;
                }
                let int_only = matches!(op, BitAnd | BitOr | BitXor | Shl | Shr);
                let number = self.require_number(l, &lt, int_only);
                if !number || (op == Pow && !self.check_power(l, &lt)) {
                    self.value(r, None);
                    return Ty::Error;
                }
                self.check_expect(r, &lt);
                lt
            }
        }
    }
}
