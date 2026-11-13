//! Maps (spec 2.2): entries in insertion order, plus an index from each
//! key's hash to its entries, so finding a key doesn't scan the map.

use crate::value::Value;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Clone, Debug, Default)]
pub struct MapValue<'a> {
    pub entries: Vec<(Value<'a>, Value<'a>)>,
    index: HashMap<u64, Vec<usize>>,
}

/// Maps are equal when they hold the same keys with equal values, in any
/// order (spec 1.8): each key looked up by hash.
impl PartialEq for MapValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && (self.entries.iter()).all(|(k, v)| other.get(k).is_some_and(|w| v == w))
    }
}

impl<'a> MapValue<'a> {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn find(&self, key: &Value<'a>) -> Option<usize> {
        self.index
            .get(&key_hash(key))?
            .iter()
            .copied()
            .find(|e| self.entries[*e].0 == *key)
    }

    pub fn get(&self, key: &Value<'a>) -> Option<&Value<'a>> {
        self.find(key).map(|e| &self.entries[e].1)
    }

    /// The value under `key`, added as `Unit` first if it is missing.
    pub fn slot(&mut self, key: Value<'a>) -> &mut Value<'a> {
        let e = match self.find(&key) {
            Some(e) => e,
            None => {
                let hash = key_hash(&key);
                self.index.entry(hash).or_default().push(self.entries.len());
                self.entries.push((key, Value::Unit));
                self.entries.len() - 1
            }
        };
        &mut self.entries[e].1
    }

    pub fn insert(&mut self, key: Value<'a>, value: Value<'a>) {
        *self.slot(key) = value;
    }

    /// Removes `key`, keeping the order of the other entries.
    pub fn remove(&mut self, key: &Value<'a>) {
        if let Some(e) = self.find(key) {
            self.entries.remove(e);
            self.index.clear();
            for (i, (k, _)) in self.entries.iter().enumerate() {
                self.index.entry(key_hash(k)).or_default().push(i);
            }
        }
    }
}

/// A hash that agrees with `==` on values: equal values hash the same.
fn key_hash(v: &Value<'_>) -> u64 {
    let mut h = DefaultHasher::new();
    hash_into(v, &mut h);
    h.finish()
}

fn hash_into(v: &Value<'_>, h: &mut DefaultHasher) {
    match v {
        Value::Int(n, _) => n.hash(h),
        // 0.0 and -0.0 are equal, so they hash the same.
        Value::Float(f) => (f + 0.0).to_bits().hash(h),
        Value::Float32(f) => (f + 0.0).to_bits().hash(h),
        // Equal decimals (1.10 and 1.1) have the same value as a float.
        Value::Decimal(d) => d.to_f64().to_bits().hash(h),
        Value::BigInt(n) => n.hash(h),
        Value::BigDecimal(_) => crate::bigint::any_decimal(v)
            .map(|d| d.to_f64().to_bits())
            .hash(h),
        Value::Bool(b) => b.hash(h),
        Value::Char(c) => c.hash(h),
        Value::Str(s) | Value::BuiltinError(s) => s.hash(h),
        Value::List(items) => items.iter().for_each(|x| hash_into(x, h)),
        Value::Tuple(items) => items.iter().for_each(|x| hash_into(x, h)),
        // Equal maps may hold their entries in another order.
        Value::Map(m) => m
            .entries
            .iter()
            .fold(0u64, |acc, (k, x)| {
                let mut eh = DefaultHasher::new();
                hash_into(k, &mut eh);
                hash_into(x, &mut eh);
                acc.wrapping_add(eh.finish())
            })
            .hash(h),
        // Equal sets may hold their values in another order.
        Value::Set(s) => s
            .entries
            .iter()
            .fold(0u64, |acc, (k, _)| acc.wrapping_add(key_hash(k)))
            .hash(h),
        Value::Record { fields, .. } => fields.iter().for_each(|f| hash_into(&f.value, h)),
        Value::Variant { name, fields, .. } => {
            name.hash(h);
            fields.iter().for_each(|f| hash_into(&f.value, h));
        }
        // A `context` error equals the error it wraps.
        Value::Context(_, inner) | Value::Ok(inner) | Value::Err(inner) => hash_into(inner, h),
        Value::Range(a, b, _) => (a, b).hash(h),
        Value::Regex(r) => r.source().hash(h),
        Value::Ticker(ms) => ms.hash(h),
        Value::Task(t) => t.id.hash(h),
        Value::Bytes(b) => b.hash(h),
        Value::Reader(r) => std::rc::Rc::as_ptr(r).hash(h),
        Value::Socket(s) => std::sync::Arc::as_ptr(s).hash(h),
        Value::FakeConn(c) => std::rc::Rc::as_ptr(c).hash(h),
        Value::Channel(c, end) => (std::rc::Rc::as_ptr(c), end.is_none()).hash(h),
        Value::Shared(cell) => std::rc::Rc::as_ptr(cell).hash(h),
        Value::Weak(cell) => cell.as_ptr().hash(h),
        Value::Func(def) => def.hash(h),
        Value::Agent(a) => std::rc::Rc::as_ptr(a).hash(h),
        Value::Unit | Value::None | Value::Closure(_) => 0u8.hash(h),
    }
}
