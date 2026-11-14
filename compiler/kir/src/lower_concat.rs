//! `xs + ys` and `xs += ys` on lists (spec 2.2), as pushes onto a list,
//! so each backend's kinds of list (flat, text, packed) need nothing new;
//! and `xs.pop()` (spec 14.2), as its last item and a shorter list; and
//! a change to a list or map in a field or an item, made in place.

use crate::ir::{self, Block, ExprKind as K, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// Whether `ty` is a list type.
    pub(crate) fn is_list_type(&self, ty: Type) -> bool {
        matches!(self.shapes.shape(ty), Some(Shape::List(_)))
    }

    /// `xs + ys`: a copy of `xs`, with each item of `ys` pushed onto it.
    /// The copy is made by the first push, only when `xs` is still held.
    pub(crate) fn list_concat(&mut self, l: ir::Expr, r: ir::Expr, line: u32) -> Lowered<ir::Expr> {
        let ty = l.ty;
        let r = self.coerce(r, ty)?;
        let acc = self.new_local(ty);
        let mut stmts = vec![Stmt::Let(acc, l)];
        stmts.extend(self.push_all(acc, r, line));
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts,
                value: Some(Box::new(self.local_expr(acc, line))),
            }),
            ty,
            line,
        })
    }

    /// `xs += ys` with `xs` a variable: each item of `ys` pushed onto it,
    /// in place when nothing else holds it (spec 6.1).
    pub(crate) fn list_extend(
        &mut self,
        target: Local,
        r: ir::Expr,
        line: u32,
    ) -> Lowered<Vec<Stmt>> {
        let r = self.coerce(r, self.locals[target as usize])?;
        Ok(self.push_all(target, r, line))
    }

    /// `xs.pop()`: the last item (`none` for an empty list), and `xs` cut
    /// to the items before it, in place when nothing else holds it. `None`
    /// when `base` is not a list.
    pub(crate) fn list_pop(
        &mut self,
        base: &'a ast::Expr,
        ty: Type,
        line: u32,
    ) -> Lowered<Option<ir::Expr>> {
        let typed = self.c.expr_types.get(&(base.span.start, base.span.end));
        if !matches!(typed, Some(kavach_checker::Ty::List(_))) {
            return Ok(None);
        }
        let list = self.expr(base)?;
        let list_ty = list.ty;
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let (len, item) = (self.new_local(Type::I64), self.new_local(ty));
        let last = at(K::Rt(RtOp::ListLast, vec![self.expr(base)?]), ty);
        let one = at(K::Int(1), Type::I64);
        let keep = at(
            K::Binary(
                ir::BinOp::Sub,
                Box::new(self.local_expr(len, line)),
                Box::new(one),
            ),
            Type::I64,
        );
        let cut = at(
            K::Rt(RtOp::ListTruncate, vec![self.expr(base)?, keep]),
            list_ty,
        );
        let stmts = vec![
            Stmt::Let(len, at(K::Rt(RtOp::ListLen, vec![list]), Type::I64)),
            Stmt::Let(item, last),
            self.assign_path(base, cut)?,
        ];
        Ok(Some(at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(self.local_expr(item, line))),
            }),
            ty,
        )))
    }

    /// `b.items.push(x)`, `ls[0].push(x)`: `changed` is the op on what
    /// `base` holds. A field's or item's list is held by its place too, so
    /// changing it would copy it each time (Sakshi's S6: 40,000 pushes in 5
    /// s). Held in a local, with the place emptied while the op runs, it has
    /// one holder and changes in place (spec 6.1). Only for a path whose
    /// indexes are names or numbers, as it is written three times.
    pub(crate) fn change_in_place(
        &mut self,
        base: &'a ast::Expr,
        changed: ir::Expr,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        let simple = |mut e: &ast::Expr| loop {
            match &e.kind {
                ast::ExprKind::Name(_) => return true,
                ast::ExprKind::Field {
                    base,
                    optional: false,
                    ..
                } => e = base,
                ast::ExprKind::Index { base, index }
                    if matches!(index.kind, ast::ExprKind::Name(_) | ast::ExprKind::Int(_)) =>
                {
                    e = base
                }
                _ => return false,
            }
        };
        if !matches!(changed.kind, K::Rt(..)) {
            out.push(self.assign_path(base, changed)?);
            return Ok(());
        }
        let (ty, line) = (changed.ty, changed.line);
        let K::Rt(op, mut args) = changed.kind else {
            unreachable!("just checked");
        };
        if !op.changes_receiver()
            || matches!(base.kind, ast::ExprKind::Name(_))
            || !simple(base)
            || args.is_empty()
        {
            let changed = self.at(K::Rt(op, args), ty, line);
            out.push(self.assign_path(base, changed)?);
            return Ok(());
        }
        // The receiver, then the other operands, as they were worked out.
        let mut held = Vec::new();
        for a in args.drain(..) {
            let l = self.new_local(a.ty);
            out.push(Stmt::Let(l, a));
            held.push(l);
        }
        let empty = self.at(K::Int(0), self.locals[held[0] as usize], line);
        out.push(self.assign_path(base, empty)?);
        let operands = held
            .iter()
            .map(|&l| self.at(K::Move(l), self.locals[l as usize], line))
            .collect();
        let changed = self.at(K::Rt(op, operands), ty, line);
        out.push(self.assign_path(base, changed)?);
        Ok(())
    }

    /// `for item in r { acc = acc.push(item) }`.
    fn push_all(&mut self, acc: Local, r: ir::Expr, line: u32) -> Vec<Stmt> {
        let list_ty = self.locals[acc as usize];
        let item_ty = match self.shapes.shape(list_ty) {
            Some(Shape::List(t)) => *t,
            _ => unreachable!("a list"),
        };
        let src = self.new_local(r.ty);
        let (item, index) = (self.new_local(item_ty), self.new_local(Type::I64));
        let pushed = ir::Expr {
            kind: K::Rt(
                RtOp::ListPush,
                vec![self.local_expr(acc, line), self.local_expr(item, line)],
            ),
            ty: list_ty,
            line,
        };
        let set = ir::Expr {
            kind: K::Assign(acc, Box::new(pushed)),
            ty: Type::Unit,
            line,
        };
        vec![
            Stmt::Let(src, r),
            Stmt::ForEach {
                var: item,
                index,
                list: src,
                body: Block {
                    stmts: vec![Stmt::Expr(set)],
                    value: None,
                },
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{ExprKind, Type};

    /// Sakshi's S6: `b.items.push(x)` copied the list each time, as the
    /// field held it too. The field is emptied (a null) while it changes.
    #[test]
    fn a_field_lets_go_of_its_list_while_it_grows() {
        let src = "type Box {\n    items: [Int]\n}\n\nfn main() {\n    let b = Box(items: [])\n    b.items.push(1)\n    print(b.items)\n}\n";
        let c = kavach_driver::compile(src, "t.kav", Default::default());
        let program = crate::lower(&c).expect("lowers");
        let main = &program.functions[program.entry as usize];
        let mut emptied = false;
        crate::walk::any_in_block(&main.body, &mut |e| {
            if let ExprKind::SetField(_, _, v) = &e.kind {
                emptied |= matches!(v.kind, ExprKind::Int(0)) && matches!(v.ty, Type::Ref(_));
            }
            false
        });
        assert!(emptied);
    }
}
