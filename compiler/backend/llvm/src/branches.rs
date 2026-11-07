//! Expressions that branch: `if`/`else`, `fallback` and the
//! short-circuit `and`/`or`. Each merges its value through a stack slot,
//! which LLVM turns into a register. Split from `expr.rs`.

use crate::func::FnEmit;
use crate::module::llvm_type;
use kavach_kir::{Block, Expr, Type};

impl FnEmit<'_> {
    pub(crate) fn if_else(
        &mut self,
        cond: &Expr,
        then: &Block,
        otherwise: &Block,
        ty: Type,
    ) -> Option<String> {
        let c = self.expr(cond)?;
        let result = (ty != Type::Unit).then(|| self.slot(ty));
        let (yes, no, merge) = (self.label(), self.label(), self.label());
        self.cond_br(&c, &yes, &no);
        for (label, body) in [(&yes, then), (&no, otherwise)] {
            self.start(label);
            let v = self.block(body);
            if let Some(slot) = &result
                && let Some(v) = v.or_else(|| Self::zero(ty))
            {
                self.ins(format!("store {} {v}, ptr {slot}", llvm_type(ty)));
            }
            self.br(&merge);
        }
        self.start(&merge);
        result.map(|slot| self.val(format!("load {}, ptr {slot}", llvm_type(ty))))
    }

    /// `fast`, whose failed checks jump to `slow` (see `call_rt`).
    pub(crate) fn fallback(&mut self, fast: &Block, slow: &Block, ty: Type) -> Option<String> {
        let result = (ty != Type::Unit).then(|| self.slot(ty));
        let (slow_label, merge) = (self.label(), self.label());
        for (i, body) in [fast, slow].into_iter().enumerate() {
            if i == 0 {
                self.fallbacks.push(slow_label.clone());
            } else {
                self.start(&slow_label);
            }
            let v = self.block(body);
            if let Some(slot) = &result
                && let Some(v) = v.or_else(|| Self::zero(ty))
            {
                self.ins(format!("store {} {v}, ptr {slot}", llvm_type(ty)));
            }
            if i == 0 {
                self.fallbacks.pop();
            }
            self.br(&merge);
        }
        self.start(&merge);
        result.map(|slot| self.val(format!("load {}, ptr {slot}", llvm_type(ty))))
    }

    /// `and` stops at false, `or` at true.
    pub(crate) fn short_circuit(&mut self, is_and: bool, l: &Expr, r: &Expr) -> String {
        let slot = self.slot(Type::Bool);
        let a = self.expr(l).expect("a Bool");
        self.ins(format!("store i1 {a}, ptr {slot}"));
        let (rhs, merge) = (self.label(), self.label());
        if is_and {
            self.cond_br(&a, &rhs, &merge);
        } else {
            self.cond_br(&a, &merge, &rhs);
        }
        self.start(&rhs);
        let b = self.expr(r).expect("a Bool");
        self.ins(format!("store i1 {b}, ptr {slot}"));
        self.br(&merge);
        self.start(&merge);
        self.val(format!("load i1, ptr {slot}"))
    }
}
