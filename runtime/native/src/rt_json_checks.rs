//! The invariants of decoded values (spec 2.12 and 7.3, decision 0086).
//! At a `json.decode` call, generated code registers each record type
//! with invariants that the decoded type can hold (`JsonCheckRegister`):
//! a closure giving the bits of the invariants a value breaks, and their
//! texts. After decoding, `broken` walks the value and lists each broken
//! invariant with the record's path, as the interpreter does.

use crate::heap::{release, retain, slots, tag_of};
use crate::shapes::{items_of, shape, with_str};
use kavach_kir::{Shape, Type};
use kavach_runtime_core::json::{path_field, path_item};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Each registered type's check and the texts of its invariants.
type Check = (i64, Vec<String>);

static CHECKS: OnceLock<Mutex<HashMap<u32, Check>>> = OnceLock::new();

fn checks() -> std::sync::MutexGuard<'static, HashMap<u32, Check>> {
    CHECKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// `JsonCheckRegister`: records of type number `ty` are checked by
/// closure `check` (owned, kept), whose invariants are the `String`s in
/// list `texts` (owned). A type registered before keeps its first check.
pub(crate) fn register(ty: i64, check: i64, texts: i64) -> i64 {
    let words: Vec<String> = items_of(texts)
        .iter()
        .map(|t| with_str(*t as i64, str::to_string))
        .collect();
    release(texts);
    let mut table = checks();
    match table.get(&(ty as u32)) {
        Some(_) => release(check),
        None => {
            table.insert(ty as u32, (check, words));
        }
    }
    static AT_EXIT: std::sync::Once = std::sync::Once::new();
    AT_EXIT.call_once(|| crate::rt::at_exit(forget_all));
    0
}

/// Lets go of the checks at the end (for the leak check).
fn forget_all() {
    let all: Vec<i64> = checks().drain().map(|(_, (c, _))| c).collect();
    for c in all {
        release(c);
    }
}

/// Each invariant the decoded value `slot` of type `ty` breaks: the
/// record's path, and ``invariant `…` ``.
pub(crate) fn broken(slot: u64, ty: Type) -> Vec<(String, String)> {
    if checks().is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    walk(slot, ty, "", &mut out);
    out
}

fn walk(slot: u64, ty: Type, path: &str, out: &mut Vec<(String, String)>) {
    let Type::Ref(id) = ty else { return };
    if slot == 0 {
        return;
    }
    let p = slot as i64;
    match shape(ty) {
        Shape::Record { fields, .. } => {
            let check = checks().get(&id).cloned();
            if let Some((check, texts)) = check {
                let f = slots(check)[0] as usize;
                // SAFETY: slot 0 of a closure is its function's address;
                // the code generator compiled a check as `fn(closure,
                // value) -> Int`, borrowing the closure and taking the
                // value (held once more here), and the code lives as long
                // as the program.
                #[allow(unsafe_code)]
                let f: extern "C-unwind" fn(i64, i64) -> i64 = unsafe { std::mem::transmute(f) };
                retain(p);
                let bits = f(check, p) as u64;
                for (i, text) in texts.iter().enumerate().take(64) {
                    if bits >> i & 1 == 1 {
                        out.push((path.to_string(), format!("invariant `{text}`")));
                    }
                }
            }
            let held: Vec<u64> = slots(p).to_vec();
            for (f, v) in fields.iter().zip(held) {
                walk(v, f.ty, &path_field(path, &f.name), out);
            }
        }
        Shape::Union { variants, .. } => {
            let Some(v) = variants.get(tag_of(p) as usize) else {
                return;
            };
            let at = path_field(path, &v.name);
            let held: Vec<u64> = slots(p).to_vec();
            for (f, value) in v.fields.iter().zip(held) {
                walk(value, f.ty, &path_field(&at, &f.name), out);
            }
        }
        Shape::List(item) if item.is_ref() => {
            for (i, v) in items_of(p).iter().enumerate() {
                walk(*v, *item, &path_item(path, i), out);
            }
        }
        Shape::Optional(inner) if inner.is_ref() => walk(slot, *inner, path, out),
        Shape::Map(_, value) if value.is_ref() => {
            for pair in items_of(p).chunks(2) {
                let key = with_str(pair[0] as i64, str::to_string);
                walk(pair[1], *value, &path_field(path, &key), out);
            }
        }
        Shape::Tuple(ts) => {
            let held: Vec<u64> = slots(p).to_vec();
            for (i, (t, v)) in ts.iter().zip(held).enumerate() {
                walk(v, *t, &path_item(path, i), out);
            }
        }
        _ => {}
    }
}
