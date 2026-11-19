// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A copy for a task on another core (`copy`) shares no object with what
//! it was made from, and holds the same value: records, unions, lists of
//! objects, lists of numbers, flat lists and short and long text.

use super::copy;
use crate::heap::{
    Data, alloc_fixed, has_data, new_obj, new_text, obj, release, slot_type, slots, slots_mut,
    str_of,
};
use kavach_kir::{FieldShape, Shape, Type, VariantShape};

const STR: Type = Type::Ref(0);
const WORD: Type = Type::Ref(1);
const WORDS: Type = Type::Ref(2);
const CMD: Type = Type::Ref(3);
const CMDS: Type = Type::Ref(4);
const INTS: Type = Type::Ref(5);
const PAIR: Type = Type::Ref(6);
const PAIRS: Type = Type::Ref(7);

fn field(name: &str, ty: Type) -> FieldShape {
    FieldShape {
        name: name.into(),
        ty,
        secret: false,
    }
}

/// The types of S16's probe: `Word = Text(text: String) | Items(items:
/// [Word])`, `Cmd {name: String, words: [Word], line: Int}`, and lists.
fn set_shapes() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let variant = |name: &str, f: FieldShape| VariantShape {
            name: name.into(),
            fields: vec![f],
        };
        let record = |name: &str, fields| Shape::Record {
            name: name.into(),
            fields,
            drop: false,
        };
        crate::shapes::set_shapes(&[
            Shape::Str,
            Shape::Union {
                name: "Word".into(),
                variants: vec![
                    variant("Text", field("text", STR)),
                    variant("Items", field("items", WORDS)),
                ],
            },
            Shape::List(WORD),
            record(
                "Cmd",
                vec![
                    field("name", STR),
                    field("words", WORDS),
                    field("line", Type::I64),
                ],
            ),
            Shape::List(CMD),
            Shape::List(Type::I64),
            record("Pair", vec![field("a", Type::I64), field("b", Type::I64)]),
            Shape::List(PAIR),
        ]);
    });
}

fn text(s: &str) -> u64 {
    new_text(s, STR) as u64
}

fn fixed(ty: Type, tag: u32, words: &[u64]) -> u64 {
    let p = alloc_fixed(ty, tag, words.len());
    slots_mut(p).copy_from_slice(words);
    p as u64
}

fn list(ty: Type, items: Vec<u64>) -> u64 {
    new_obj(ty, 0, items, Data::Slots) as u64
}

fn cmd(i: u64, last: &str) -> u64 {
    let inner = list(WORDS, vec![fixed(WORD, 0, &[text(last)])]);
    let words = list(
        WORDS,
        vec![fixed(WORD, 0, &[text("0.1")]), fixed(WORD, 1, &[inner])],
    );
    fixed(CMD, 0, &[text("set_load"), words, i])
}

/// Checks that `b` is a copy of `a`, of type `ty`, with no object of its
/// own shared, and gathers the objects of both.
fn same(a: u64, b: u64, ty: Type, objects: &mut (Vec<u64>, Vec<u64>)) {
    if !ty.is_ref() || a == 0 {
        assert_eq!(a, b);
        return;
    }
    let (pa, pb) = (a as i64, b as i64);
    assert_ne!(a, b, "a copy is an object of its own");
    assert_eq!(obj(pb).rc, 1, "the caller owns the copy");
    assert_eq!(obj(pa).ty(), obj(pb).ty());
    assert_eq!(obj(pa).tag, obj(pb).tag);
    objects.0.push(a);
    objects.1.push(b);
    if has_data(pa) {
        assert_eq!(str_of(pa), str_of(pb));
        return;
    }
    assert_eq!(slots(pa).len(), slots(pb).len());
    for (i, (x, y)) in slots(pa).iter().zip(slots(pb)).enumerate() {
        same(*x, *y, slot_type(pa, i), objects);
    }
}

#[test]
fn a_copy_shares_no_object_and_keeps_every_value() {
    set_shapes();
    let long = "a text longer than thirty bytes, kept outside the object";
    let values = [
        (
            list(
                CMDS,
                (0..50)
                    .map(|i| cmd(i, if i % 2 == 0 { long } else { "p" }))
                    .collect(),
            ),
            CMDS,
        ),
        (cmd(7, long), CMD),
        (fixed(WORD, 0, &[text(long)]), WORD),
        (list(INTS, (0..1000).collect()), INTS),
        (list(PAIRS, vec![1, 2, 3, 4, 5, 6]), PAIRS),
    ];
    for (value, ty) in values {
        let copied = copy(value, ty);
        let mut objects = (Vec::new(), Vec::new());
        same(value, copied, ty, &mut objects);
        let mut all: Vec<u64> = objects.0.iter().chain(&objects.1).copied().collect();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), objects.0.len() * 2, "no object is in both");
        release(value as i64);
        release(copied as i64);
    }
}
