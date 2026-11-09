//! Destructuring in `const` and `let` (spec 1.12): lists by position,
//! maps by key, records by field, patterns nested in each other. A part
//! that may be missing is read as an optional; a default fills it. A
//! nested pattern on such a part gives each name inside its final local
//! first (`dest_finals`), then an `if` fills them: from the part when it
//! is there, from defaults or `none` when it isn't. Each name's type is
//! the checker's, so both branches agree.

use crate::ir::{self, Block, ExprKind as K, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, MapKey, PatternKind};

impl<'a> Lowerer<'a> {
    /// Binds the names of list or record pattern `p` to parts of local
    /// `whole` (of type `ty`); `known` is how many items a list literal
    /// showed. False when `p` is neither kind.
    pub(crate) fn destructure_parts(
        &mut self,
        p: &'a ast::Pattern,
        whole: Local,
        ty: Type,
        known: usize,
        out: &mut Vec<Stmt>,
    ) -> Lowered<bool> {
        if !nested(p) {
            return Ok(false);
        }
        self.take_apart(p, whole, ty, known, out)?;
        Ok(true)
    }

    /// Takes the value in local `whole` (there, of type `ty`) apart.
    fn take_apart(
        &mut self,
        p: &'a ast::Pattern,
        whole: Local,
        ty: Type,
        known: usize,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        let line = self.line(p.span);
        let it = |l: &Self| l.local_expr(whole, line);
        match (&p.kind, self.shapes.shape(ty).cloned()) {
            (PatternKind::List { items, .. }, Some(Shape::Map(key_ty, value_ty))) => {
                let opt = self.optional_of(value_ty);
                for item in items {
                    let Some(key) = self.key_of(item, key_ty, line)? else {
                        continue;
                    };
                    let part = at(K::Rt(RtOp::MapGet, vec![it(self), key]), opt, line);
                    self.take_part(&item.pattern, part, item.default.as_ref(), out)?;
                }
            }
            (PatternKind::List { items, rest }, Some(Shape::List(item_ty))) => {
                let len = at(K::Rt(RtOp::ListLen, vec![it(self)]), Type::I64, line);
                for (i, item) in items.iter().enumerate() {
                    let index = at(K::Int(i as i128), Type::I64, line);
                    let read = at(
                        K::Index(Box::new(it(self)), Box::new(index.clone())),
                        item_ty,
                        line,
                    );
                    let part = if i < known {
                        read
                    } else {
                        // Item `i` only when the list has it.
                        let has = at(
                            K::Binary(ir::BinOp::Lt, Box::new(index), Box::new(len.clone())),
                            Type::Bool,
                            line,
                        );
                        let opt = self.optional_of(item_ty);
                        let some = if opt == item_ty {
                            read
                        } else {
                            at(K::Some(Box::new(read)), opt, line)
                        };
                        choose(has, some, at(K::None, opt, line), opt, line)
                    };
                    self.take_part(&item.pattern, part, item.default.as_ref(), out)?;
                }
                if let Some(r) = rest {
                    let skip = at(K::Int(items.len() as i128), Type::I64, line);
                    let left = at(K::Rt(RtOp::ListSkip, vec![it(self), skip]), ty, line);
                    self.put(r, left, out)?;
                }
            }
            (PatternKind::Record(fields), _) => {
                for f in fields {
                    let (index, field_ty) = self.field_index(ty, &f.field.name, f.field.span)?;
                    let part = at(K::GetField(Box::new(it(self)), index), field_ty, line);
                    self.take_part(&f.pattern, part, f.default.as_ref(), out)?;
                }
            }
            _ => return Err(self.unsupported("this pattern", p.span)),
        }
        Ok(())
    }

    /// One part, `part`, for sub-pattern `pat`, with its default if it has
    /// one. An optional part with a default or a nested pattern may be
    /// missing.
    fn take_part(
        &mut self,
        pat: &'a ast::Pattern,
        part: ir::Expr,
        default: Option<&'a ast::Expr>,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        let line = part.line;
        let inner = match self.shapes.shape(part.ty) {
            Some(Shape::Optional(i)) if default.is_some() || nested(pat) => Some(*i),
            _ => None,
        };
        let (part, maybe_missing) = match (inner, default) {
            // A default fills a missing part.
            (Some(inner), Some(d)) => {
                let held = self.new_local(part.ty);
                out.push(Stmt::Let(held, part));
                let d = self.expr(d)?;
                let d = self.coerce(d, inner)?;
                let is = at(
                    K::IsSome(Box::new(self.local_expr(held, line))),
                    Type::Bool,
                    line,
                );
                let inside = at(
                    K::Unwrap(Box::new(self.local_expr(held, line))),
                    inner,
                    line,
                );
                (choose(is, inside, d, inner, line), false)
            }
            (Some(_), None) => (part, true),
            (None, _) => (part, false),
        };
        match &pat.kind {
            PatternKind::Wildcard => Ok(()),
            PatternKind::Binding(id) => self.put(id, part, out),
            _ if maybe_missing => self.take_maybe(pat, part, out),
            _ => {
                let held = self.new_local(part.ty);
                let ty = part.ty;
                out.push(Stmt::Let(held, part));
                self.take_apart(pat, held, ty, 0, out)
            }
        }
    }

    /// Nested pattern `pat` on optional `part`, which may be missing.
    fn take_maybe(
        &mut self,
        pat: &'a ast::Pattern,
        part: ir::Expr,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        let line = part.line;
        let Some(Shape::Optional(inner)) = self.shapes.shape(part.ty).cloned() else {
            return Err(self.unsupported("this pattern", pat.span));
        };
        let held = self.new_local(part.ty);
        out.push(Stmt::Let(held, part));
        // Each name's final local, holding a placeholder until the `if`.
        let mut names = Vec::new();
        names_in(pat, &mut names);
        let mut defs = Vec::new();
        for id in names {
            let Some(&def) = self.decl.get(&id.span.start) else {
                continue;
            };
            let ty = self.final_type(id)?;
            let local = self.new_local(ty);
            let start = self.placeholder(ty, id.span)?;
            out.push(Stmt::Let(local, start));
            self.bind(id, local);
            self.dest_finals.insert(def, local);
            defs.push(def);
        }
        let mut there = Vec::new();
        let value = self.new_local(inner);
        there.push(Stmt::Let(
            value,
            at(
                K::Unwrap(Box::new(self.local_expr(held, line))),
                inner,
                line,
            ),
        ));
        self.take_apart(pat, value, inner, 0, &mut there)?;
        let mut gone = Vec::new();
        self.take_missing(pat, inner, &mut gone)?;
        for def in defs {
            self.dest_finals.remove(&def);
        }
        let is = at(
            K::IsSome(Box::new(self.local_expr(held, line))),
            Type::Bool,
            line,
        );
        let block = |stmts| Block { stmts, value: None };
        out.push(Stmt::Expr(at(
            K::If(Box::new(is), block(there), block(gone)),
            Type::Unit,
            line,
        )));
        Ok(())
    }

    /// Every name in `pat`, for a part of type `ty` that is missing: its
    /// default, or `none`.
    fn take_missing(
        &mut self,
        pat: &'a ast::Pattern,
        ty: Type,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        let line = self.line(pat.span);
        let none = |l: &mut Self, t: Type| {
            let opt = l.optional_of(t);
            at(K::None, opt, line)
        };
        match (&pat.kind, self.shapes.shape(ty).cloned()) {
            (PatternKind::List { items, .. }, Some(Shape::Map(_, v))) => {
                for it in items {
                    let part = none(self, v);
                    self.take_part(&it.pattern, part, it.default.as_ref(), out)?;
                }
            }
            (PatternKind::List { items, rest }, Some(Shape::List(item_ty))) => {
                for it in items {
                    let part = none(self, item_ty);
                    self.take_part(&it.pattern, part, it.default.as_ref(), out)?;
                }
                if let Some(r) = rest {
                    self.put(r, at(K::List(Vec::new()), ty, line), out)?;
                }
            }
            (PatternKind::Record(fields), _) => {
                for f in fields {
                    let (_, field_ty) = self.field_index(ty, &f.field.name, f.field.span)?;
                    let part = none(self, field_ty);
                    self.take_part(&f.pattern, part, f.default.as_ref(), out)?;
                }
            }
            _ => return Err(self.unsupported("this pattern", pat.span)),
        }
        Ok(())
    }

    /// Gives name `id` `value`: into its final local when a nested pattern
    /// made one (`take_maybe`), else into a new one.
    fn put(&mut self, id: &'a ast::Ident, value: ir::Expr, out: &mut Vec<Stmt>) -> Lowered<()> {
        let line = value.line;
        let def = self.decl.get(&id.span.start).copied();
        if let Some(&local) = def.and_then(|d| self.dest_finals.get(&d)) {
            let ty = self.locals[local as usize];
            let value = self.coerce(value, ty)?;
            out.push(Stmt::Expr(at(
                K::Assign(local, Box::new(value)),
                Type::Unit,
                line,
            )));
            return Ok(());
        }
        let ty = self.final_type(id)?;
        let value = self.coerce(value, ty)?;
        let local = self.new_local(ty);
        self.bind(id, local);
        out.push(Stmt::Let(local, value));
        Ok(())
    }

    /// The type the checker gave name `id`.
    fn final_type(&mut self, id: &ast::Ident) -> Lowered<Type> {
        let ty = self
            .decl
            .get(&id.span.start)
            .and_then(|d| self.c.local_types.get(d))
            .cloned();
        match ty {
            Some(t) => self.kir_type(&t, id.span),
            None => Err(self.unsupported("this name", id.span)),
        }
    }

    /// A value of type `ty` a name holds until the `if` of `take_maybe`
    /// gives it its own.
    fn placeholder(&mut self, ty: Type, span: ast::Span) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let kind = match (self.shapes.shape(ty), ty) {
            (Some(Shape::Optional(_)), _) => K::None,
            (Some(Shape::List(_)), _) => K::List(Vec::new()),
            (Some(Shape::Str), _) => K::Str(String::new()),
            (None, Type::F64 | Type::F32) => K::Float(0.0),
            (None, Type::Bool) => K::Bool(false),
            (None, t) if t != Type::Unit && !t.is_ref() => K::Int(0),
            _ => {
                return Err(
                    self.unsupported("a name of this type in a part that may be missing", span)
                );
            }
        };
        Ok(at(kind, ty, line))
    }

    /// The key a map pattern's item reads, of type `key_ty`.
    fn key_of(
        &mut self,
        it: &'a ast::ListItem,
        key_ty: Type,
        line: u32,
    ) -> Lowered<Option<ir::Expr>> {
        Ok(Some(match &it.key {
            Some(MapKey::Literal(e)) => {
                let k = self.expr(e)?;
                self.coerce(k, key_ty)?
            }
            Some(MapKey::Name(n)) => at(K::Str(n.name.clone()), key_ty, line),
            None => match &it.pattern.kind {
                PatternKind::Binding(id) => at(K::Str(id.name.clone()), key_ty, line),
                _ => return Ok(None),
            },
        }))
    }

    /// `t?`, or `t` when it already is optional.
    fn optional_of(&mut self, t: Type) -> Type {
        match self.shapes.shape(t) {
            Some(Shape::Optional(_)) => t,
            _ => self.shapes_intern(Shape::Optional(t)),
        }
    }
}

fn nested(p: &ast::Pattern) -> bool {
    matches!(p.kind, PatternKind::List { .. } | PatternKind::Record(_))
}

/// The names pattern `p` binds, at any depth.
fn names_in<'p>(p: &'p ast::Pattern, out: &mut Vec<&'p ast::Ident>) {
    match &p.kind {
        PatternKind::Binding(id) => out.push(id),
        PatternKind::List { items, rest } => {
            items.iter().for_each(|it| names_in(&it.pattern, out));
            out.extend(rest.iter());
        }
        PatternKind::Record(fields) => fields.iter().for_each(|f| names_in(&f.pattern, out)),
        _ => {}
    }
}

fn at(kind: K, ty: Type, line: u32) -> ir::Expr {
    ir::Expr { kind, ty, line }
}

/// `if cond { yes } else { no }`, giving `ty`.
fn choose(cond: ir::Expr, yes: ir::Expr, no: ir::Expr, ty: Type, line: u32) -> ir::Expr {
    let block = |e: ir::Expr| Block {
        stmts: Vec::new(),
        value: Some(Box::new(e)),
    };
    at(K::If(Box::new(cond), block(yes), block(no)), ty, line)
}
