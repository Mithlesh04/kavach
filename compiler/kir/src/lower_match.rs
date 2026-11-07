//! Lowering `match`: a chain of `if`s over a local holding the value,
//! with patterns turned into conditions and bindings.

use crate::ir::{self, Block, ExprKind as K, Shape, Stmt, Type, UnOp};
use crate::lower::{Lowered, Lowerer};
use crate::lower_expr::wrap;
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, PatternKind};
use kavach_checker::Ty;
use kavach_resolver::{DefId, DefKind};

/// One step from a matched value to a part of it, with the part's type.
#[derive(Clone, Copy)]
enum Step {
    Field(u32),
    Element(u32),
    /// The value of a present optional.
    Unwrap,
}

impl<'a> Lowerer<'a> {
    /// `match` becomes a chain of `if`s over a local holding the value.
    pub(crate) fn match_expr(
        &mut self,
        scrutinee: &'a ast::Expr,
        arms: &'a [ast::MatchArm],
        ty: Type,
        want_value: bool,
        line: u32,
    ) -> Lowered<ir::Expr> {
        // A call that can fail is matched with `ok(…)` and `err(…)` (spec 5.4).
        let (first, tmp, value_ty, outcome) = if self.can_fail(scrutinee) {
            let o = self.outcome(scrutinee)?;
            (o.stmt, o.value, o.ok, Some((o.error, o.err)))
        } else {
            let value = self.expr(scrutinee)?;
            let tmp = self.new_local(value.ty);
            let ty = value.ty;
            (Stmt::Let(tmp, value), tmp, ty, None)
        };
        let mut stmts = vec![first];
        let saved = self.error_views.len();
        let error_ty = self.error_type();
        if value_ty == error_ty {
            self.view_errors(arms, tmp, false, &mut stmts, line)?;
        }
        if let Some((error, err_ty)) = outcome
            && err_ty == error_ty
        {
            self.view_errors(arms, error, true, &mut stmts, line)?;
        }
        let arms = self.match_arms(arms, tmp, value_ty, ty, want_value, line, outcome);
        self.error_views.truncate(saved);
        let block = Block {
            stmts,
            value: Some(Box::new(arms?)),
        };
        Ok(ir::Expr {
            kind: K::Block(block),
            ty,
            line,
        })
    }

    /// The arms of a `match` as a chain of `if`s, last arm innermost.
    #[allow(clippy::too_many_arguments)]
    fn match_arms(
        &mut self,
        arms: &'a [ast::MatchArm],
        tmp: ir::Local,
        value_ty: Type,
        ty: Type,
        want_value: bool,
        line: u32,
        outcome: Option<(ir::Local, Type)>,
    ) -> Lowered<ir::Expr> {
        let mut rest = ir::Expr {
            kind: K::Panic("no `match` arm matched".into()),
            ty,
            line,
        };
        for arm in arms.iter().rev() {
            let (mut conds, binds) = (Vec::new(), &mut Vec::new());
            match (&arm.pattern.kind, outcome) {
                (PatternKind::Ok(inner), Some((error, _))) => {
                    conds.push(self.succeeded(error, line));
                    self.pattern(inner, tmp, &mut Vec::new(), value_ty, &mut conds, binds)?;
                }
                (PatternKind::Err(inner), Some((error, err_ty))) => {
                    conds.push(self.failed(error, line));
                    self.pattern(inner, error, &mut Vec::new(), err_ty, &mut conds, binds)?;
                }
                _ => self.pattern(
                    &arm.pattern,
                    tmp,
                    &mut Vec::new(),
                    value_ty,
                    &mut conds,
                    binds,
                )?,
            }
            let lets: Vec<Stmt> = binds.drain(..).map(|(l, e)| Stmt::Let(l, e)).collect();
            let mut body = Block::default();
            if let Some(g) = &arm.guard {
                // Bind first, then test the guard, only once the pattern matched.
                let guard = self.expr(g)?;
                conds.push(ir::Expr {
                    kind: K::Block(Block {
                        stmts: lets,
                        value: Some(Box::new(guard)),
                    }),
                    ty: Type::Bool,
                    line,
                });
            } else {
                body.stmts = lets;
            }
            if want_value {
                let v = self.expr(&arm.body)?;
                body.value = Some(Box::new(self.coerce(v, ty)?));
            } else {
                self.expr_stmt(&arm.body, &mut body.stmts)?;
            }
            let cond = conds.into_iter().reduce(|a, b| ir::Expr {
                kind: K::And(Box::new(a), Box::new(b)),
                ty: Type::Bool,
                line,
            });
            rest = match cond {
                None => ir::Expr {
                    kind: K::Block(body),
                    ty,
                    line,
                },
                Some(c) => ir::Expr {
                    kind: K::If(Box::new(c), body, wrap(rest, want_value)),
                    ty,
                    line,
                },
            };
        }
        Ok(rest)
    }

    /// For a `match` on an `Error` (spec 5.5): a local for each error type
    /// its arms name (`FsError.NotFound`), holding the error as that type,
    /// or `none` (`RtOp::ErrorAs`). With `in_err`, the arms are `err(…)`.
    fn view_errors(
        &mut self,
        arms: &'a [ast::MatchArm],
        source: ir::Local,
        in_err: bool,
        stmts: &mut Vec<Stmt>,
        line: u32,
    ) -> Lowered<()> {
        for arm in arms {
            let p = match (&arm.pattern.kind, in_err) {
                (PatternKind::Err(inner), true) => &**inner,
                (PatternKind::Ok(_) | PatternKind::Err(_), _) | (_, true) => continue,
                _ => &arm.pattern,
            };
            let Some(union) = self.union_of(p) else {
                continue;
            };
            if self
                .error_views
                .iter()
                .any(|v| v.0 == source && v.1 == union)
            {
                continue;
            }
            let union_ty = self.kir_type(&Ty::Adt(union, Vec::new()), p.span)?;
            let opt = self.shapes.intern(Shape::Optional(union_ty));
            let view = self.new_local(opt);
            let value = ir::Expr {
                kind: K::Rt(RtOp::ErrorAs, vec![self.local_expr(source, line)]),
                ty: opt,
                line,
            };
            stmts.push(Stmt::Let(view, value));
            self.error_views.push((source, union, view, union_ty));
        }
        Ok(())
    }

    /// The union whose variant pattern `p` names, if it is one.
    fn union_of(&self, p: &ast::Pattern) -> Option<DefId> {
        let PatternKind::Variant { path, .. } = &p.kind else {
            return None;
        };
        let res = &self.c.resolution;
        let last = path.segments.last()?;
        match res.defs[*res.uses.get(&last.span.start)?].kind {
            DefKind::Variant { of } => Some(of),
            _ => None,
        }
    }

    /// The value at `path` inside local `tmp`.
    fn part(&self, tmp: ir::Local, path: &[(Step, Type)], line: u32) -> ir::Expr {
        let mut e = self.local_expr(tmp, line);
        for (step, ty) in path {
            let kind = match step {
                Step::Unwrap => K::Unwrap(Box::new(e)),
                Step::Field(i) => K::GetField(Box::new(e), *i),
                Step::Element(i) => K::Index(
                    Box::new(e),
                    Box::new(ir::Expr {
                        kind: K::Int(*i as i128),
                        ty: Type::I64,
                        line,
                    }),
                ),
            };
            e = ir::Expr {
                kind,
                ty: *ty,
                line,
            };
        }
        e
    }

    /// Adds the conditions under which `p` matches, and the names it binds.
    fn pattern(
        &mut self,
        p: &'a ast::Pattern,
        tmp: ir::Local,
        path: &mut Vec<(Step, Type)>,
        ty: Type,
        conds: &mut Vec<ir::Expr>,
        binds: &mut Vec<(ir::Local, ir::Expr)>,
    ) -> Lowered<()> {
        let line = self.line(p.span);
        let compare = |l: &mut Self, op: ir::BinOp, x: ir::Expr, path: &[(Step, Type)]| {
            let part = l.part(tmp, path, line);
            ir::Expr {
                kind: K::Binary(op, Box::new(part), Box::new(x)),
                ty: Type::Bool,
                line,
            }
        };
        match &p.kind {
            PatternKind::Wildcard => {}
            PatternKind::Binding(id) => {
                let local = self.new_local(ty);
                self.bind(id, local);
                binds.push((local, self.part(tmp, path, line)));
            }
            PatternKind::Literal(e) => {
                let v = self.expr(e)?;
                let v = self.coerce(v, ty)?;
                conds.push(compare(self, ir::BinOp::Eq, v, path));
            }
            PatternKind::Range { lo, hi, inclusive } => {
                let (lo, hi) = (self.expr(lo)?, self.expr(hi)?);
                conds.push(compare(self, ir::BinOp::Ge, lo, path));
                conds.push(compare(
                    self,
                    if *inclusive {
                        ir::BinOp::Le
                    } else {
                        ir::BinOp::Lt
                    },
                    hi,
                    path,
                ));
            }
            PatternKind::None => {
                let present = ir::Expr {
                    kind: K::IsSome(Box::new(self.part(tmp, path, line))),
                    ty: Type::Bool,
                    line,
                };
                conds.push(ir::Expr {
                    kind: K::Unary(UnOp::Not, Box::new(present)),
                    ty: Type::Bool,
                    line,
                });
            }
            PatternKind::Tuple(items) => {
                let Some(Shape::Tuple(types)) = self.shapes.shape(ty).cloned() else {
                    return Err(self.unsupported("this pattern", p.span));
                };
                for (i, (item, t)) in items.iter().zip(types).enumerate() {
                    path.push((Step::Element(i as u32), t));
                    self.pattern(item, tmp, path, t, conds, binds)?;
                    path.pop();
                }
            }
            // On an `Error`: the error as the arm's type, if it is one.
            PatternKind::Variant { .. } if matches!(self.shapes.shape(ty), Some(Shape::Error)) => {
                let union = self.union_of(p);
                let view = self
                    .error_views
                    .iter()
                    .find(|v| v.0 == tmp && path.is_empty() && Some(v.1) == union)
                    .map(|v| (v.2, v.3));
                let Some((view, union_ty)) = view else {
                    return Err(self.unsupported("this pattern on an `Error`", p.span));
                };
                conds.push(ir::Expr {
                    kind: K::IsSome(Box::new(self.local_expr(view, line))),
                    ty: Type::Bool,
                    line,
                });
                let mut inside = vec![(Step::Unwrap, union_ty)];
                self.pattern(p, view, &mut inside, union_ty, conds, binds)?;
            }
            PatternKind::Variant { path: name, fields } => {
                let variant_name = &name.segments.last().expect("a name").name;
                let Some(Shape::Union { variants, .. }) = self.shapes.shape(ty).cloned() else {
                    return Err(self.unsupported("this pattern", p.span));
                };
                let Some(index) = variants.iter().position(|v| v.name == *variant_name) else {
                    return Err(self.unsupported("this variant", p.span));
                };
                let has = ir::Expr {
                    kind: K::HasVariant(Box::new(self.part(tmp, path, line)), index as u32),
                    ty: Type::Bool,
                    line,
                };
                conds.push(has);
                let shape_fields = &variants[index].fields;
                for (i, fp) in fields.iter().flatten().enumerate() {
                    let field_name = match (&fp.name, &fp.pattern.kind) {
                        (Some(n), _) => Some(n.name.as_str()),
                        (None, PatternKind::Binding(b))
                            if shape_fields.iter().any(|f| f.name == b.name) =>
                        {
                            Some(b.name.as_str())
                        }
                        _ => None,
                    };
                    let at = field_name
                        .map_or(Some(i), |n| shape_fields.iter().position(|f| f.name == n));
                    let Some(at) = at.filter(|a| *a < shape_fields.len()) else {
                        return Err(self.unsupported("this pattern", p.span));
                    };
                    path.push((Step::Field(at as u32), shape_fields[at].ty));
                    self.pattern(&fp.pattern, tmp, path, shape_fields[at].ty, conds, binds)?;
                    path.pop();
                }
            }
            PatternKind::Ok(_) | PatternKind::Err(_) => {
                return Err(self.unsupported("matching on errors", p.span));
            }
            // The checker allows these only in `const` and `let` (1.12).
            PatternKind::List { .. } | PatternKind::Record(_) => {
                return Err(self.unsupported("list and record patterns in `match`", p.span));
            }
        }
        Ok(())
    }
}
