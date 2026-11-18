//! A function with `inout` parameters that can't fail gives back a tuple
//! of its value and the parameters' last values (`lower_inout.rs`). When a
//! call of it isn't inlined, that tuple is an object made and freed at
//! every call: about 100 ns (Sakshi's S11, the VCD reader's
//! `header_bytes(inout r, …) -> Bool`). Here such a function returns its
//! value alone and leaves each `inout` value in a slot of its thread
//! (`RtOp::InoutStash`); each call takes them straight back
//! (`RtOp::InoutTake`) in a block whose value is the tuple, which
//! `tuple_lets.rs` then splits into locals, so no object is made.
//!
//! Only for a function called directly: one whose address is taken (a
//! function value, an interface method) keeps its tuple, as do the
//! program's roots and functions with float items, which the runtime's
//! slots don't carry. Between the stash and the take only waiting `drop`s
//! can run (`drop_scopes.rs`), so a function a `drop` method could reach
//! keeps its tuple too.

use crate::ir::*;
use crate::rt_op::RtOp;
use crate::walk::{any_in_block, blocks_mut, exprs_mut};
use std::collections::{HashMap, HashSet};

/// The most `inout` values a function leaves in slots.
const SLOTS: usize = 8;

/// `inout` holds, for each function lowered from a declaration with
/// `inout` parameters that can't fail, how many it has.
pub fn stash_inout_tuples(
    functions: &mut [Function],
    shapes: &[Shape],
    roots: &[FuncId],
    inout: &HashMap<FuncId, usize>,
) {
    let taken = |id: FuncId, fs: &[Function]| {
        fs.iter().any(|f| {
            any_in_block(
                &f.body,
                &mut |e| matches!(e.kind, ExprKind::Closure { func, .. } if func == id),
            )
        })
    };
    let from_drops = reached_by_drops(functions);
    // Each function changed: its tuple's items and how many lead with
    // the function's own value (0 or 1).
    let mut changed: HashMap<FuncId, (Vec<Type>, usize)> = HashMap::new();
    for (&id, &n) in inout {
        let Some(f) = functions.get(id as usize) else {
            continue;
        };
        let Type::Ref(t) = f.ret else { continue };
        let Some(Shape::Tuple(items)) = shapes.get(t as usize) else {
            continue;
        };
        let floats = items.iter().any(|t| matches!(t, Type::F64 | Type::F32));
        let lead = items.len().wrapping_sub(n);
        // A `drop` waiting to run between the stash and the take could
        // call it again and overwrite the slots.
        let dropped = from_drops.as_ref().is_none_or(|r| r.contains(&id));
        if f.fails
            || floats
            || n > SLOTS
            || lead > 1
            || roots.contains(&id)
            || taken(id, functions)
            || dropped
        {
            continue;
        }
        changed.insert(id, (items.clone(), lead));
    }
    for (&id, (items, lead)) in &changed {
        give_value_alone(&mut functions[id as usize], items, *lead);
    }
    for f in functions.iter_mut() {
        take_back(f, &changed);
    }
}

/// The functions a `drop` method (registered with `DropRegister`) may
/// call, itself included; `None` when one calls a function value, which
/// could be any function.
fn reached_by_drops(functions: &[Function]) -> Option<HashSet<FuncId>> {
    let mut todo: Vec<FuncId> = Vec::new();
    for f in functions {
        any_in_block(&f.body, &mut |e| {
            if let ExprKind::Rt(RtOp::DropRegister, args) = &e.kind
                && let Some(ExprKind::Closure { func, .. }) = args.get(1).map(|a| &a.kind)
            {
                todo.push(*func);
            }
            false
        });
    }
    let mut seen = HashSet::new();
    while let Some(id) = todo.pop() {
        if !seen.insert(id) {
            continue;
        }
        let Some(f) = functions.get(id as usize) else {
            continue;
        };
        let mut indirect = false;
        any_in_block(&f.body, &mut |e| {
            match &e.kind {
                ExprKind::Call(c, _) | ExprKind::Closure { func: c, .. } => todo.push(*c),
                ExprKind::CallValue(..) => indirect = true,
                _ => {}
            }
            false
        });
        if indirect {
            return None;
        }
    }
    Some(seen)
}

/// Function `f` returns its value alone, after putting each `inout` value
/// of its tuple in its slot.
fn give_value_alone(f: &mut Function, items: &[Type], lead: usize) {
    let mut g = Give {
        tuple: f.ret,
        ret: if lead == 1 { items[0] } else { Type::Unit },
        items,
        lead,
        line: f.line,
        locals: &mut f.locals,
    };
    blocks_mut(&mut f.body, &mut |b| {
        for s in &mut b.stmts {
            if let Stmt::Return(Some(e)) = s {
                let e = std::mem::replace(
                    e,
                    Expr {
                        kind: ExprKind::None,
                        ty: Type::Unit,
                        line: g.line,
                    },
                );
                *s = Stmt::Return(Some(g.leave(e)));
            }
        }
    });
    if let Some(v) = f.body.value.take() {
        f.body.value = Some(Box::new(g.leave(*v)));
    }
    f.ret = g.ret;
}

struct Give<'f> {
    tuple: Type,
    ret: Type,
    items: &'f [Type],
    lead: usize,
    line: u32,
    locals: &'f mut Vec<Type>,
}

impl Give<'_> {
    fn new_local(&mut self, t: Type) -> Local {
        self.locals.push(t);
        (self.locals.len() - 1) as Local
    }

    fn stash(&self, slot: usize, value: Expr) -> Stmt {
        let at = Expr {
            kind: ExprKind::Int(slot as i128),
            ty: Type::I64,
            line: self.line,
        };
        Stmt::Expr(Expr {
            kind: ExprKind::Rt(RtOp::InoutStash, vec![at, value]),
            ty: Type::Unit,
            line: self.line,
        })
    }

    /// `e`, a tuple-giving expression, as one giving the value alone.
    /// The stash goes into each branch that builds the tuple, so no tuple
    /// is built (after `inline.rs` lifted returns, the value is an `if`).
    fn leave(&mut self, mut e: Expr) -> Expr {
        let line = e.line;
        match e.kind {
            ExprKind::Tuple(items) => {
                let mut stmts = Vec::new();
                let mut items = items.into_iter();
                let value = if self.lead == 1 {
                    // The value first, as the tuple would: it may change
                    // an `inout` local.
                    let v = self.new_local(self.ret);
                    stmts.push(Stmt::Let(v, items.next().expect("the value")));
                    Some(Box::new(Expr {
                        kind: ExprKind::Local(v),
                        ty: self.ret,
                        line,
                    }))
                } else {
                    None
                };
                for (slot, item) in items.enumerate() {
                    stmts.push(self.stash(slot, item));
                }
                Expr {
                    kind: ExprKind::Block(Block { stmts, value }),
                    ty: self.ret,
                    line,
                }
            }
            ExprKind::Block(ref mut b) if b.value.is_some() => {
                let v = b.value.take().expect("a value");
                b.value = Some(Box::new(self.leave(*v)));
                e.ty = self.ret;
                e
            }
            ExprKind::If(_, ref mut then, ref mut otherwise) => {
                for b in [then, otherwise] {
                    if let Some(v) = b.value.take() {
                        b.value = Some(Box::new(self.leave(*v)));
                    }
                }
                e.ty = self.ret;
                e
            }
            _ => self.through_local(e),
        }
    }

    /// `e` held in a local, its items read from there.
    fn through_local(&mut self, e: Expr) -> Expr {
        let (tuple, line) = (self.tuple, self.line);
        let t = self.new_local(tuple);
        let item = |i: usize, ty: Type| Expr {
            kind: ExprKind::Index(
                Box::new(Expr {
                    kind: ExprKind::Local(t),
                    ty: tuple,
                    line,
                }),
                Box::new(Expr {
                    kind: ExprKind::Int(i as i128),
                    ty: Type::I64,
                    line,
                }),
            ),
            ty,
            line,
        };
        let mut stmts = vec![Stmt::Let(t, e)];
        for (slot, i) in (self.lead..self.items.len()).enumerate() {
            stmts.push(self.stash(slot, item(i, self.items[i])));
        }
        let value = (self.lead == 1).then(|| Box::new(item(0, self.ret)));
        Expr {
            kind: ExprKind::Block(Block { stmts, value }),
            ty: self.ret,
            line,
        }
    }
}

/// Each call in `f` of a function in `changed` becomes a block: the call,
/// then its `inout` values taken from their slots, with the tuple the
/// call gave before as its value.
fn take_back(f: &mut Function, changed: &HashMap<FuncId, (Vec<Type>, usize)>) {
    let locals = &mut f.locals;
    exprs_mut(&mut f.body, &mut |e| {
        let ExprKind::Call(id, _) = &e.kind else {
            return;
        };
        let Some((items, lead)) = changed.get(id) else {
            return;
        };
        let (tuple, line) = (e.ty, e.line);
        let ret = if *lead == 1 { items[0] } else { Type::Unit };
        let call = Expr {
            kind: std::mem::replace(&mut e.kind, ExprKind::None),
            ty: ret,
            line,
        };
        let mut stmts = Vec::new();
        let mut values = Vec::new();
        if *lead == 1 {
            let v = locals.len() as Local;
            locals.push(ret);
            stmts.push(Stmt::Let(v, call));
            values.push(Expr {
                kind: ExprKind::Local(v),
                ty: ret,
                line,
            });
        } else {
            stmts.push(Stmt::Expr(call));
        }
        for (slot, i) in (*lead..items.len()).enumerate() {
            let at = Expr {
                kind: ExprKind::Int(slot as i128),
                ty: Type::I64,
                line,
            };
            values.push(Expr {
                kind: ExprKind::Rt(RtOp::InoutTake, vec![at]),
                ty: items[i],
                line,
            });
        }
        *e = Expr {
            kind: ExprKind::Block(Block {
                stmts,
                value: Some(Box::new(Expr {
                    kind: ExprKind::Tuple(values),
                    ty: tuple,
                    line,
                })),
            }),
            ty: tuple,
            line,
        };
    });
}

#[cfg(test)]
mod tests;
