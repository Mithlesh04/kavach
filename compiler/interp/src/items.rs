//! A list's items, and the most it may hold when it was made with
//! `List.capacity(n)` (spec 2.2). Used as a `Vec`: the limit is checked
//! where items are added.

use crate::calls::Args;
use crate::interp::Interp;
use crate::map::MapValue;
use crate::value::{Eval, Value};
use kavach_ast::Span;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub struct Items<'a> {
    items: Vec<Value<'a>>,
    /// At most this many items (spec 2.2), or no limit.
    pub limit: Option<usize>,
    /// A ring (`Ring<T>`): when full, a push drops the oldest item.
    pub ring: bool,
}

impl<'a> Items<'a> {
    /// A list with no limit.
    pub fn new(items: Vec<Value<'a>>) -> Self {
        Items {
            items,
            limit: None,
            ring: false,
        }
    }

    /// Whether the list holds as many items as its limit allows.
    pub fn is_full(&self) -> bool {
        self.limit.is_some_and(|n| self.items.len() >= n)
    }

    /// The items, the limit left behind.
    pub fn into_vec(self) -> Vec<Value<'a>> {
        self.items
    }
}

impl<'a> Deref for Items<'a> {
    type Target = Vec<Value<'a>>;
    fn deref(&self) -> &Vec<Value<'a>> {
        &self.items
    }
}

impl<'a> DerefMut for Items<'a> {
    fn deref_mut(&mut self) -> &mut Vec<Value<'a>> {
        &mut self.items
    }
}

impl<'a> FromIterator<Value<'a>> for Items<'a> {
    fn from_iter<I: IntoIterator<Item = Value<'a>>>(iter: I) -> Self {
        Items::new(iter.into_iter().collect())
    }
}

impl<'a> From<Vec<Value<'a>>> for Items<'a> {
    fn from(items: Vec<Value<'a>>) -> Self {
        Items::new(items)
    }
}

/// Lists are equal when their items are (spec 1.8): the limit is not part
/// of what a list holds.
impl PartialEq for Items<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}

pub use kavach_runtime_core::full_panic as full;

/// Adds `item` to the list, or the panic message when it is full. A
/// full ring drops its oldest item instead (spec 2.2).
pub fn push<'a>(items: &mut Rc<Items<'a>>, item: Value<'a>) -> Result<(), String> {
    if let Some(n) = items.limit.filter(|n| items.len() >= *n) {
        if !items.ring {
            return Err(full("list", n));
        }
        if n == 0 {
            return Ok(());
        }
        Rc::make_mut(items).remove(0);
    }
    Rc::make_mut(items).push(item);
    Ok(())
}

impl<'a> Interp<'a> {
    /// `List.capacity(n, first)`, `Map.capacity(…)`, `Set.capacity(…)`:
    /// at most `n` entries, the first ones maybe given (spec 2.2).
    pub(crate) fn capacity(&mut self, kind: &str, args: Args<'a>, span: Span) -> Eval<'a> {
        let mut args = args.into_iter().map(|(_, v)| v);
        let asked = match args.next() {
            Some(Value::Int(n, _)) => n,
            _ => 0,
        };
        if asked > i128::from(kavach_runtime_core::MAX_CAPACITY) {
            let message = kavach_runtime_core::capacity_panic(kind, asked as i64, 0);
            return Err(self.panic(message, span));
        }
        let n = asked.max(0) as usize;
        let first = args.next();
        if kind == "Ring" {
            let mut ring = Rc::new(Items::new(Vec::with_capacity(n.min(1 << 20))));
            let ring_mut = Rc::make_mut(&mut ring);
            (ring_mut.limit, ring_mut.ring) = (Some(n), true);
            if let Some(Value::List(xs)) = first {
                for x in xs.iter() {
                    push(&mut ring, x.clone()).map_err(|m| self.panic(m, span))?;
                }
            }
            return Ok(Value::List(ring));
        }
        let (made, len) = match (kind, first) {
            ("Dict", first) => {
                let mut m = match first {
                    Some(Value::Map(m)) => Rc::unwrap_or_clone(m),
                    _ => MapValue::default(),
                };
                m.limit = Some(n);
                let len = m.len();
                (Value::Map(Rc::new(m)), len)
            }
            ("Set", first) => {
                let items = match first {
                    Some(Value::List(xs)) => xs.to_vec(),
                    _ => Vec::new(),
                };
                let Value::Set(set) = self.set_of(items) else {
                    unreachable!("a set")
                };
                let mut set = Rc::unwrap_or_clone(set);
                set.limit = Some(n);
                let len = set.len();
                (Value::Set(Rc::new(set)), len)
            }
            (_, first) => {
                let mut items = Items::new(Vec::with_capacity(n.min(1 << 20)));
                if let Some(Value::List(xs)) = first {
                    items.extend(xs.iter().cloned());
                }
                items.limit = Some(n);
                let len = items.len();
                (Value::List(Rc::new(items)), len)
            }
        };
        if len > n {
            let message = kavach_runtime_core::capacity_panic(kind, n as i64, len);
            return Err(self.panic(message, span));
        }
        Ok(made)
    }
}
