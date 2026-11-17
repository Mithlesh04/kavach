//! `arena { … }` (spec 6.3): a block whose values are freed together when
//! it ends. A task spawned in it may use them, so the arena waits for the
//! tasks spawned in it before it ends: `mark; value = block; join(mark,
//! wait)`, the join that lets them finish rather than cancel them.

use crate::ir::{self, Block, ExprKind as K, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, ExprKind};

impl<'a> Lowerer<'a> {
    pub(crate) fn arena(
        &mut self,
        b: &'a ast::Block,
        want_value: bool,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let body = self.branch_block(b, want_value, ty)?;
        let at = |kind, ty| ir::Expr { kind, ty, line };
        if !spawns(b) {
            return Ok(at(K::Block(body), ty));
        }
        let mark = self.new_local(Type::I64);
        let mut stmts = vec![Stmt::Let(
            mark,
            at(K::Rt(RtOp::TaskMark, Vec::new()), Type::I64),
        )];
        let value = if want_value && ty != Type::Unit {
            let held = self.new_local(ty);
            stmts.push(Stmt::Let(held, at(K::Block(body), ty)));
            Some(Box::new(self.local_expr(held, line)))
        } else {
            stmts.push(Stmt::Expr(at(K::Block(body), ty)));
            None
        };
        let wait = at(K::Int(1), Type::I64);
        let join = K::Rt(RtOp::TaskJoin, vec![self.local_expr(mark, line), wait]);
        stmts.push(Stmt::Expr(at(join, Type::Unit)));
        Ok(at(K::Block(Block { stmts, value }), ty))
    }
}

/// Whether `b` has a `spawn` anywhere in it.
fn spawns(b: &ast::Block) -> bool {
    struct Find(bool);
    impl kavach_ast::visit::VisitMut for Find {
        fn expr(&mut self, e: &mut ast::Expr) {
            self.0 |= matches!(e.kind, ExprKind::Spawn(..));
            kavach_ast::visit::walk_expr(self, e);
        }
    }
    let mut find = Find(false);
    kavach_ast::visit::walk_block(&mut find, &mut b.clone());
    find.0
}
