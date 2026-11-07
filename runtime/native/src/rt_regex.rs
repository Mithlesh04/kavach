//! `std.regex` in native code (spec 14.8), with the same engine as the
//! interpreter (`kavach_runtime_core::regex`).

use crate::heap::{Data, alloc, alloc_slots, new_str, release, shape, str_of};
use crate::rt::{panic_with, type_of};
use crate::rt_data::optional;
use crate::rt_errors::set_error;
use crate::rt_maps::kv_map_new;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_core::regex::{GaveUp, Pattern};
use std::sync::Arc;

crate::opaque::opaque_kind!(REGEX, Pattern, wrap, unwrap, |p: &Pattern| p
    .source()
    .to_string());

fn pattern_of(p: i64) -> Arc<Pattern> {
    let found = match &*crate::heap::data(p).borrow() {
        Data::Regex(r) => unwrap(r),
        _ => None,
    };
    found.unwrap_or_else(|| Arc::new(Pattern::new("", false).expect("an empty pattern")))
}

fn text(p: i64) -> String {
    let s = str_of(p);
    release(p);
    s
}

/// The item type of list (or optional) type `ty`.
fn inner(ty: Type) -> Type {
    match shape(ty) {
        Shape::List(t) | Shape::Optional(t) => *t,
        _ => ty,
    }
}

/// The first text type in the program.
fn str_type() -> Type {
    crate::heap::first_of(&Shape::Str)
}

fn strings(xs: Vec<String>, code: i64) -> i64 {
    let item = inner(type_of(code));
    let slots = xs.into_iter().map(|x| new_str(x, item) as u64).collect();
    alloc_slots(type_of(code), 0, slots)
}

pub fn regex_op(op: RtOp, a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    if matches!(op, RtOp::RegexNew | RtOp::RegexAdvanced) {
        return match Pattern::new(&text(a), op == RtOp::RegexAdvanced) {
            Ok(p) => alloc(type_of(code), Data::Regex(wrap(Arc::new(p)))),
            Err(e) => {
                set_error(new_str(e, str_type()));
                0
            }
        };
    }
    let r = pattern_of(a);
    release(a);
    let t = text(b);
    let gave_up = |_: GaveUp| -> ! {
        panic_with(
            "regex gave up: this advanced pattern needed too much work on this text (spec 14.8)",
            line,
        )
    };
    match op {
        RtOp::RegexIsMatch => r.is_match(&t).unwrap_or_else(|e| gave_up(e)) as i64,
        RtOp::RegexFind => {
            let found = r.find(&t).unwrap_or_else(|e| gave_up(e));
            optional(found.map(|m| new_str(m, inner(type_of(code)))), code)
        }
        RtOp::RegexFindAll => strings(r.find_all(&t).unwrap_or_else(|e| gave_up(e)), code),
        RtOp::RegexSplit => strings(r.split(&t).unwrap_or_else(|e| gave_up(e)), code),
        RtOp::RegexCaptures => match r.captures(&t).unwrap_or_else(|e| gave_up(e)) {
            Some(groups) => {
                // A list of optional text: `none` is a null slot.
                let list = inner(type_of(code));
                let text_ty = inner(inner(list));
                let slots = groups
                    .into_iter()
                    .map(|g| g.map_or(0, |s| new_str(s, text_ty)) as u64)
                    .collect();
                alloc_slots(list, 0, slots)
            }
            None => 0,
        },
        RtOp::RegexNamedCaptures => match r.named_captures(&t).unwrap_or_else(|e| gave_up(e)) {
            Some(pairs) => {
                let map_ty = inner(type_of(code));
                let mut map = kv_map_new(crate::rt::type_code(map_ty));
                let text_ty = str_type();
                for (k, v) in pairs {
                    let (k, v) = (new_str(k, text_ty), new_str(v, text_ty));
                    map = crate::rt_maps::map_or_shared(RtOp::MapInsert, map, k, v, 0, 0);
                }
                map
            }
            None => 0,
        },
        _ => {
            // RegexReplace and RegexReplaceAll: `c` is the replacement.
            let with = text(c);
            let all = op == RtOp::RegexReplaceAll;
            new_str(
                r.replace(&t, &with, all).unwrap_or_else(|e| gave_up(e)),
                type_of(code),
            )
        }
    }
}

// The group's runtime entry (`RtOp::entry`, decision 0024).
crate::rt_data::entry!(kv_rt_regex, |op, a, b, c, code, line| {
    regex_op(op, a, b, c, code, line)
});
