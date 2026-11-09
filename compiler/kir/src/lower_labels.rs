//! Labeled loops (spec 4.6, decision 0054): `break outer` and
//! `continue outer` from inside an inner loop. KIR's `Break` and
//! `Continue` leave only the innermost loop, which every pass and backend
//! relies on, so a jump to an outer loop is lowered with a flag: it sets
//! the outer loop's flag and leaves its own loop; after each loop it
//! crossed, a check leaves the next one out too, until the loop it named
//! breaks, or clears the flag and goes on with its next round.

use crate::ir::{self, Block, ExprKind as K, Local, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast as ast;

/// A source loop being lowered, innermost last.
#[derive(Default)]
pub(crate) struct LoopFrame {
    label: Option<String>,
    /// Set by a jump from an inner loop: 1 to break this loop, 2 to go on
    /// with its next round. Made at the first such jump.
    flag: Option<Local>,
    /// The outer loops a jump from inside this one goes to: after this
    /// loop, each gets its check.
    crossing: Vec<usize>,
}

const BREAK: i128 = 1;
const CONTINUE: i128 = 2;

impl<'a> Lowerer<'a> {
    /// Lowers loop statement `lower` (a `for` or `while`) as the loop
    /// labeled `label`, followed by the checks of the jumps that cross it.
    pub(crate) fn labeled_loop(
        &mut self,
        label: &Option<ast::Ident>,
        out: &mut Vec<Stmt>,
        lower: impl FnOnce(&mut Self, &mut Vec<Stmt>) -> Lowered<()>,
    ) -> Lowered<()> {
        let at = out.len();
        self.loop_frames.push(LoopFrame {
            label: label.as_ref().map(|l| l.name.clone()),
            ..LoopFrame::default()
        });
        let lowered = lower(self, out);
        let frame = self.loop_frames.pop().expect("the loop pushed above");
        lowered?;
        if let Some(flag) = frame.flag {
            // Cleared each time the loop starts, before its first round.
            out.insert(at, Stmt::Let(flag, self.int(0)));
        }
        let parent = self.loop_frames.len().checked_sub(1);
        for target in frame.crossing {
            let flag = self.loop_frames[target].flag.expect("made by the jump");
            if Some(target) == parent {
                // Next to the loop the jump named: break it, or clear the
                // flag and go on with its next round.
                out.push(self.when(flag, BREAK, vec![Stmt::Break]));
                let clear =
                    Stmt::Expr(self.bare(K::Assign(flag, Box::new(self.int(0))), Type::Unit));
                out.push(self.when(flag, CONTINUE, vec![clear, Stmt::Continue]));
            } else if let Some(p) = parent {
                // Still further out: leave this loop too.
                out.push(self.when_set(flag, vec![Stmt::Break]));
                if !self.loop_frames[p].crossing.contains(&target) {
                    self.loop_frames[p].crossing.push(target);
                }
            }
        }
        Ok(())
    }

    /// `break` or `continue` (`again`), to the innermost loop or the one
    /// labeled `label` (the checker made sure it is around).
    pub(crate) fn jump(&mut self, label: &Option<ast::Ident>, again: bool, out: &mut Vec<Stmt>) {
        let plain = if again { Stmt::Continue } else { Stmt::Break };
        let top = self.loop_frames.len().checked_sub(1);
        let target = match label {
            None => top,
            Some(l) => self
                .loop_frames
                .iter()
                .rposition(|f| f.label.as_deref() == Some(l.name.as_str())),
        };
        let (Some(target), Some(top)) = (target, top) else {
            out.push(plain);
            return;
        };
        if target == top {
            out.push(plain);
            return;
        }
        let flag = match self.loop_frames[target].flag {
            Some(f) => f,
            None => {
                let f = self.new_local(Type::I64);
                self.loop_frames[target].flag = Some(f);
                f
            }
        };
        let kind = if again { CONTINUE } else { BREAK };
        out.push(Stmt::Expr(
            self.bare(K::Assign(flag, Box::new(self.int(kind))), Type::Unit),
        ));
        if !self.loop_frames[top].crossing.contains(&target) {
            self.loop_frames[top].crossing.push(target);
        }
        out.push(Stmt::Break);
    }

    fn int(&self, n: i128) -> ir::Expr {
        self.bare(K::Int(n), Type::I64)
    }

    /// An expression the lowering made, with no line of its own.
    fn bare(&self, kind: K, ty: Type) -> ir::Expr {
        ir::Expr { kind, ty, line: 0 }
    }

    /// `if flag == value { then }`.
    fn when(&self, flag: Local, value: i128, then: Vec<Stmt>) -> Stmt {
        let cond = self.bare(
            K::Binary(
                ir::BinOp::Eq,
                Box::new(self.local_expr(flag, 0)),
                Box::new(self.int(value)),
            ),
            Type::Bool,
        );
        self.if_stmt(cond, then)
    }

    /// `if flag != 0 { then }`.
    fn when_set(&self, flag: Local, then: Vec<Stmt>) -> Stmt {
        let cond = self.bare(
            K::Binary(
                ir::BinOp::Ne,
                Box::new(self.local_expr(flag, 0)),
                Box::new(self.int(0)),
            ),
            Type::Bool,
        );
        self.if_stmt(cond, then)
    }

    fn if_stmt(&self, cond: ir::Expr, then: Vec<Stmt>) -> Stmt {
        let then = Block {
            stmts: then,
            value: None,
        };
        Stmt::Expr(self.bare(K::If(Box::new(cond), then, Block::default()), Type::Unit))
    }
}
