// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `json.stringify` and `json.of` in native code (spec 2.12, decision
//! 0132): any value straight to JSON text, with no `Tree` in between.
//! Every value has a form: dict keys as text, a `secret` field as
//! `"<secret>"`, and a function, task or channel as its printed form. The
//! conformance tests run it against the interpreter.

use crate::display::{boxed, f32_of};
use crate::flat;
use crate::heap::{slots, tag_of};
use crate::rt_big_decimal::any_decimal_of;
use crate::shapes::{innermost, shape, with_str};
use kavach_kir::{FieldShape, Shape, Type, VariantValue};
use kavach_runtime_core::json::{self, write_int, write_text};
use kavach_runtime_core::{AnyDecimal, push_float};

/// How a value is written: `json.of` and `json.stringify` hide secrets,
/// and may write `BigInt` and `Decimal` as text (decision 0132).
#[derive(Clone, Copy, Default)]
pub(crate) struct Encode {
    pub(crate) hide_secrets: bool,
    pub(crate) big_as_text: bool,
}

/// JSON text for `slot` (borrowed) of type `ty`, indented as `json::write`.
pub(crate) fn text_of(slot: i64, ty: Type, indent: usize, o: Encode) -> String {
    let mut w = Writer {
        out: String::new(),
        indent,
        o,
    };
    w.value(slot, ty, 0);
    w.out
}

/// A dict key as JSON text (decision 0132): text as itself, numbers,
/// `Bool`, `Char` and a variant without fields as they print (or as its
/// value, when it carries one), any other key as its own compact JSON.
pub(crate) fn key_text(slot: i64, ty: Type, o: Encode) -> String {
    if !ty.is_ref() {
        return crate::display::display(slot, ty, true);
    }
    match shape(ty) {
        Shape::Str => with_str(slot, str::to_string),
        Shape::Decimal | Shape::BigInt => crate::display::display(slot, ty, true),
        Shape::Union { variants, .. } if !crate::rt_json::is_json(ty) => {
            let v = &variants[tag_of(innermost(slot)) as usize];
            match (v.fields.is_empty(), &v.value) {
                (true, Some(VariantValue::Int(n))) => n.to_string(),
                (true, Some(VariantValue::Text(t))) => t.clone(),
                (true, None) => v.name.clone(),
                (false, _) => text_of(slot, ty, 0, o),
            }
        }
        _ => text_of(slot, ty, 0, o),
    }
}

struct Writer {
    out: String,
    indent: usize,
    o: Encode,
}

impl Writer {
    fn new_line(&mut self, level: usize) {
        if self.indent > 0 {
            self.out.push('\n');
            self.out
                .extend(std::iter::repeat_n(' ', self.indent * level));
        }
    }

    fn float(&mut self, f: f64) {
        if f.is_finite() {
            // Negative zero is `0.0`, as JavaScript writes it (2.12).
            push_float(&mut self.out, f + 0.0);
        } else {
            // As JavaScript does: JSON has no NaN or infinity.
            self.out.push_str("null");
        }
    }

    /// An array of `n` items, each written by `item` one level down.
    fn array(&mut self, n: usize, level: usize, mut item: impl FnMut(&mut Self, usize)) {
        if n == 0 {
            self.out.push_str("[]");
            return;
        }
        self.out.push('[');
        for i in 0..n {
            if i > 0 {
                self.out.push(',');
            }
            self.new_line(level + 1);
            item(self, i);
        }
        self.new_line(level);
        self.out.push(']');
    }

    /// An object of `n` fields: `field` writes the key and its value.
    fn object(&mut self, n: usize, level: usize, mut field: impl FnMut(&mut Self, usize)) {
        if n == 0 {
            self.out.push_str("{}");
            return;
        }
        self.out.push('{');
        for i in 0..n {
            if i > 0 {
                self.out.push(',');
            }
            self.new_line(level + 1);
            field(self, i);
        }
        self.new_line(level);
        self.out.push('}');
    }

    fn key(&mut self, name: &str) {
        write_text(&mut self.out, name);
        self.out.push_str(if self.indent > 0 { ": " } else { ":" });
    }

    /// Declared fields, in order, from their slots.
    fn fields(&mut self, fields: &[FieldShape], items: &[u64], level: usize) {
        self.object(fields.len().min(items.len()), level, |w, i| {
            w.key(&fields[i].name);
            // As `print` shows it (decision 0132).
            if fields[i].secret && w.o.hide_secrets {
                return w.out.push_str("\"<secret>\"");
            }
            w.value(items[i] as i64, fields[i].ty, level + 1);
        });
    }

    fn list(&mut self, p: i64, ty: Type, item: Type, level: usize) {
        let words = slots(p);
        if flat::is_text(ty) {
            let width = flat::width(ty);
            self.array(words.len() / width, level, |w, i| {
                let word = &words[i * width..(i + 1) * width];
                crate::text_items::with_item(word, |s| write_text(&mut w.out, s));
            });
        } else if flat::packed(ty) {
            let n = crate::heap::list_len_items_pub(p);
            self.array(n, level, |w, i| {
                w.value(flat::slot_of(words, i, item) as i64, item, level + 1);
            });
        } else if flat::width(ty) > 0 {
            // A flat list holds the fields of its items: only numbers.
            let width = flat::width(ty);
            self.array(words.len() / width, level, |w, i| {
                let fields = &words[i * width..(i + 1) * width];
                match shape(item) {
                    Shape::Record { fields: named, .. } => {
                        match crate::rt_json::date_text(item, fields) {
                            Some(text) => write_text(&mut w.out, &text),
                            None => w.fields(named, fields, level + 1),
                        }
                    }
                    Shape::Tuple(ts) => w.array(ts.len(), level + 1, |w, f| {
                        w.value(fields[f] as i64, ts[f], level + 2);
                    }),
                    _ => w.out.push_str("null"),
                }
            });
        } else {
            self.array(words.len(), level, |w, i| {
                w.value(words[i] as i64, item, level + 1);
            });
        }
    }

    /// A map's entries, keys and values in turn.
    fn entries(&mut self, p: i64, key: Type, value: Type, level: usize) {
        let items = crate::shapes::items_of(p);
        self.object(items.len() / 2, level, |w, i| {
            if key.is_ref() && matches!(shape(key), Shape::Str) {
                with_str(items[i * 2] as i64, |k| w.key(k));
            } else {
                let k = key_text(items[i * 2] as i64, key, w.o);
                w.key(&k);
            }
            w.value(items[i * 2 + 1] as i64, value, level + 1);
        });
    }

    /// Value `w` of a parsed document, written from its tape.
    fn tape(&mut self, doc: &crate::rt_json_tape::Doc, w: u64, level: usize) {
        use crate::rt_json_tape::{ARRAY, NUMBER, OBJECT, TEXT, kind};
        match kind(w) {
            1 => self.out.push_str("false"),
            2 => self.out.push_str("true"),
            NUMBER => self.out.push_str(doc.str(w)),
            TEXT => write_text(&mut self.out, doc.str(w)),
            ARRAY => {
                let items = doc.run(w);
                self.array(items.len(), level, |o, i| o.tape(doc, items[i], level + 1));
            }
            OBJECT => {
                let fields = doc.run(w);
                self.object(fields.len() / 2, level, |o, i| {
                    o.key(doc.str(fields[i * 2]));
                    o.tape(doc, fields[i * 2 + 1], level + 1);
                });
            }
            _ => self.out.push_str("null"),
        }
    }

    fn json(&mut self, p: i64, ty: Type, level: usize) {
        if let Some((doc, w)) = crate::rt_json_tape::on_tape(p) {
            return self.tape(&doc, w, level);
        }
        let Shape::Union { variants, .. } = shape(ty) else {
            self.out.push_str("null");
            return;
        };
        let slot = slots(p).first().copied().unwrap_or(0) as i64;
        let variant = &variants[tag_of(p) as usize];
        match variant.name.as_str() {
            "Boolean" => self.out.push_str(if slot != 0 { "true" } else { "false" }),
            // Digits that aren't a JSON number are written `null`, as `NaN` is.
            "Number" => with_str(slot, |d| {
                self.out
                    .push_str(if json::is_number(d) { d } else { "null" });
            }),
            "Text" => with_str(slot, |s| write_text(&mut self.out, s)),
            "Array" => {
                let items = slots(slot);
                self.array(items.len(), level, |w, i| {
                    w.json(items[i] as i64, obj_type(items[i] as i64), level + 1);
                });
            }
            "Object" => {
                let items = crate::shapes::items_of(slot);
                self.object(items.len() / 2, level, |w, i| {
                    with_str(items[i * 2] as i64, |k| w.key(k));
                    let v = items[i * 2 + 1] as i64;
                    w.json(v, obj_type(v), level + 1);
                });
            }
            _ => self.out.push_str("null"),
        }
    }

    fn value(&mut self, slot: i64, ty: Type, level: usize) {
        match ty {
            Type::F64 => return self.float(f64::from_bits(slot as u64)),
            Type::F32 => return self.float(f64::from(f32_of(slot))),
            Type::Bool => {
                self.out.push_str(if slot != 0 { "true" } else { "false" });
                return;
            }
            Type::Char => {
                let c = char::from_u32(slot as u32).unwrap_or('?');
                write_text(&mut self.out, c.encode_utf8(&mut [0; 4]));
                return;
            }
            Type::Unit => return self.out.push_str("null"),
            Type::U64 if slot < 0 => {
                self.out.push_str(&(slot as u64).to_string());
                return;
            }
            t if !t.is_ref() => return write_int(&mut self.out, slot),
            _ => {}
        }
        let p = slot;
        match shape(ty) {
            Shape::Optional(_) if p == 0 => self.out.push_str("null"),
            Shape::Optional(inner) if inner.is_ref() => self.value(p, *inner, level),
            Shape::Optional(inner) => self.value(boxed(p), *inner, level),
            Shape::Str => with_str(p, |s| write_text(&mut self.out, s)),
            Shape::Bytes => {
                let text = crate::rt_bytes::with_bytes(p, kavach_runtime_core::base64::to_json);
                write_text(&mut self.out, &text);
            }
            // With all their digits (decision 0071).
            Shape::Decimal => self.big(match any_decimal_of(p) {
                AnyDecimal::Small(d) => d.to_string(),
                AnyDecimal::Big(b) => b.to_string(),
            }),
            Shape::BigInt => match crate::rt_bigint::big_of(p) {
                Some(n) => self.big(n.to_string()),
                None => self.out.push_str("null"),
            },
            Shape::List(item) => self.list(p, ty, *item, level),
            Shape::Set(item) => {
                let items = crate::shapes::items_of(p);
                self.array(items.len().div_ceil(2), level, |w, i| {
                    w.value(items[i * 2] as i64, *item, level + 1);
                });
            }
            Shape::Tuple(ts) => {
                let items = slots(p);
                self.array(ts.len().min(items.len()), level, |w, i| {
                    w.value(items[i] as i64, ts[i], level + 1);
                });
            }
            Shape::Map(k, v) => self.entries(p, *k, *v, level),
            // `std.time`'s types as ISO 8601 text (spec 2.12).
            Shape::Record { fields, .. } => match crate::rt_json::date_text(ty, slots(p)) {
                Some(text) => write_text(&mut self.out, &text),
                None => self.fields(fields, slots(p), level),
            },
            Shape::Union { .. } if crate::rt_json::is_json(ty) => self.json(p, ty, level),
            Shape::Union { variants, .. } => {
                let inner = innermost(p);
                let variant = &variants[tag_of(inner) as usize];
                if variant.fields.is_empty() {
                    // Its value when it carries one (spec 2.6).
                    return match &variant.value {
                        Some(VariantValue::Int(n)) => self.out.push_str(&n.to_string()),
                        Some(VariantValue::Text(t)) => write_text(&mut self.out, t),
                        None => write_text(&mut self.out, &variant.name),
                    };
                }
                self.object(1, level, |w, _| {
                    w.key(&variant.name);
                    w.fields(&variant.fields, slots(inner), level + 1);
                });
            }
            // `Shared(x)` is `x`; a `Weak` its value, or `null` once freed.
            Shape::Shared(t) if crate::send::is_weak(p) => {
                let t = *t;
                let wrote = crate::send::with_weak_value(p, |v| self.value(v as i64, t, level));
                if wrote.is_none() {
                    self.out.push_str("null");
                }
            }
            Shape::Shared(t) => {
                let t = *t;
                let wrote = crate::send::with_shared_value(p, |v| self.value(v as i64, t, level));
                if wrote.is_none() {
                    self.out.push_str("null");
                }
            }
            // No JSON meaning: the printed form, one way only (0132).
            _ => write_text(&mut self.out, &crate::display::display(p, ty, true)),
        }
    }

    /// A `BigInt`'s or `Decimal`'s digits: a number, or text when asked.
    fn big(&mut self, digits: String) {
        match self.o.big_as_text {
            true => write_text(&mut self.out, &digits),
            false => self.out.push_str(&digits),
        }
    }
}

fn obj_type(p: i64) -> Type {
    crate::heap::obj(p).ty()
}
