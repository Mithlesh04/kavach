//! `Drop` (spec 6.1): a value of a type that implements it runs `drop`
//! once, when the last holder of it ends. A block's variables end at the
//! block's end, the last made first (`stmts.rs`), a temporary at the end
//! of its statement, a function's parameters when it returns.
//!
//! The interpreter's values are copies, so a value of such a type
//! carries an identity its copies share (`value::Life`). When the last
//! one goes, the value waits in a queue, and `run_drops` runs `drop` at
//! the next statement boundary: in the same order as native code, which
//! frees the object when its count reaches 0.

use crate::interp::Interp;
use crate::value::{Eval, Field, Flow, Life, Value};
use kavach_ast::Span;
use kavach_resolver::DefId;
use std::cell::RefCell;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// The identity of a new value of record type `def`, when the type
    /// implements `Drop`.
    pub(crate) fn life_for(
        &self,
        def: DefId,
        name: &Rc<str>,
        fields: &[Field<'a>],
    ) -> Option<Rc<Life<'a>>> {
        self.drop_types.contains(&def).then(|| {
            Rc::new(Life {
                def,
                name: Rc::clone(name),
                variant: false,
                fields: RefCell::new(fields.to_vec()),
                queue: Rc::clone(&self.drops),
            })
        })
    }

    /// The identity of a new value of union `union`, variant `name`, when
    /// the union implements `Drop`.
    pub(crate) fn variant_life(
        &self,
        union: DefId,
        name: &Rc<str>,
        fields: &[Field<'a>],
    ) -> Option<Rc<Life<'a>>> {
        self.drop_types.contains(&union).then(|| {
            Rc::new(Life {
                def: union,
                name: Rc::clone(name),
                variant: true,
                fields: RefCell::new(fields.to_vec()),
                queue: Rc::clone(&self.drops),
            })
        })
    }

    /// Runs `drop` for the values whose last holder has ended, oldest
    /// first: dropping one may end others. Inside a `drop`, only the
    /// values that ended in it run here; the others wait for it to finish,
    /// as in native code, where each runs when its count reaches 0.
    pub(crate) fn run_drops(&mut self, span: Span) -> Result<(), Flow<'a>> {
        loop {
            let next = {
                let mut queue = self.drops.borrow_mut();
                if queue.len() <= self.drop_floor {
                    return Ok(());
                }
                queue.remove(self.drop_floor)
            };
            let def = match &next {
                Value::Record { def, .. } => def,
                Value::Variant { union, .. } => union,
                _ => continue,
            };
            let drop = self.methods.get(def).and_then(|m| m.get("drop")).copied();
            if let Some(f) = drop {
                let floor = std::mem::replace(&mut self.drop_floor, self.drops.borrow().len());
                let ran: Eval<'a> = self.call_fn(f, Some(next), Vec::new(), span);
                self.drop_floor = floor;
                if let Err(e @ (Flow::Panic(..) | Flow::Exit(_))) = ran {
                    return Err(e);
                }
            }
        }
    }
}

impl<'a> Interp<'a> {
    /// A panic runs no `drop` for what its task or handler held (spec 6.1,
    /// decision 0065), as native code, whose unwound frames free nothing:
    /// the values queued since `from` go without it. Each may hold more,
    /// which queue as it goes.
    pub(crate) fn forget_drops(&mut self, from: usize) {
        loop {
            let gone = {
                let mut queue = self.drops.borrow_mut();
                let from = from.min(queue.len());
                queue.split_off(from)
            };
            if gone.is_empty() {
                return;
            }
            drop(gone);
        }
    }
}

/// After `fields` changed: the value's identity keeps up. Changed in
/// place (`new` false) and held by nobody else, it is the same value;
/// otherwise it is a value of its own, with an identity of its own.
pub(crate) fn renew<'a>(life: &mut Option<Rc<Life<'a>>>, fields: &[Field<'a>], new: bool) {
    let Some(old) = life.as_mut() else { return };
    if !new && let Some(only) = Rc::get_mut(old) {
        *only.fields.get_mut() = fields.to_vec();
        return;
    }
    let fresh = Life {
        def: old.def,
        name: Rc::clone(&old.name),
        variant: old.variant,
        fields: RefCell::new(fields.to_vec()),
        queue: Rc::clone(&old.queue),
    };
    *life = Some(Rc::new(fresh));
}
