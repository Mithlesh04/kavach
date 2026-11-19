// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Text and lists of text, as in the Cranelift backend
//! (`cranelift/src/text_lists.rs`, `cranelift/src/data.rs`): text is a
//! runtime object; a list of text keeps each item in two words, short
//! text inside them (`runtime/native/src/text_items.rs`). Items go in
//! and out through the runtime; only the length of short ASCII text is
//! read here, from the item's tag.

use crate::func::FnEmit;
use kavach_kir::{BinOp, Expr, ExprKind, RtOp, Type};
use kavach_runtime_native::heap::{CAP, LEN, PTR, RC};
use kavach_runtime_native::text_items::{ASCII, INLINE, LEN_MASK, SHORT, WIDTH, encode};

/// Text given to `list.push` on a list of text.
pub(crate) enum Pushed {
    /// An owned text object.
    Object(String),
    /// The address of an item built on the stack, and the text's type code.
    Item(String, String),
}

impl FnEmit<'_> {
    /// Whether `t` is a list of text.
    pub(crate) fn text_list(&self, t: Type) -> bool {
        kavach_kir::text_list(self.shapes, t)
    }

    /// A text constant as a new text object of type `ty`.
    pub(crate) fn new_text(&mut self, text: &str, ty: Type) -> String {
        let (ptr, len) = self.m.text(text);
        let args = [
            ("i64", ptr),
            ("i64", len.to_string()),
            ("i64", Self::code(ty)),
        ];
        self.call_rt("kv_str_new", "i64", &args, false)
    }

    /// `"…{x}…"`: text built part by part, in place.
    pub(crate) fn format(&mut self, parts: &[Expr], ty: Type) -> String {
        let (mut acc, code) = ("0".to_string(), Self::code(ty));
        for p in parts {
            if let ExprKind::Str(text) = &p.kind {
                let (ptr, len) = self.m.text(text);
                let args = [
                    ("i64", acc),
                    ("i64", ptr),
                    ("i64", len.to_string()),
                    ("i64", code.clone()),
                ];
                acc = self.call_rt("kv_format_text", "i64", &args, false);
                continue;
            }
            let slot = self.slot_of(p);
            let args = [
                ("i64", acc),
                ("i64", slot),
                ("i64", Self::code(p.ty)),
                ("i64", code.clone()),
            ];
            acc = self.call_rt("kv_format", "i64", &args, false);
        }
        acc
    }

    /// Operators on heap values: `==` and `!=` on any of them, text `+`,
    /// and ordering of text.
    pub(crate) fn ref_binary(&mut self, op: BinOp, ty: Type, a: &str, b: &str) -> String {
        match op {
            BinOp::Eq | BinOp::Ne => {
                let args = [
                    ("i64", a.to_string()),
                    ("i64", b.to_string()),
                    ("i64", Self::code(ty)),
                ];
                let same = self.call_rt("kv_eq", "i8", &args, false);
                let cc = if op == BinOp::Eq { "ne" } else { "eq" };
                self.val(format!("icmp {cc} i8 {same}, 0"))
            }
            BinOp::Add => {
                let args = [("i64", a.to_string()), ("i64", b.to_string())];
                self.call_rt("kv_concat", "i64", &args, false)
            }
            _ => {
                let args = [("i64", a.to_string()), ("i64", b.to_string())];
                let order = self.call_rt("kv_cmp", "i64", &args, false);
                let cc = match op {
                    BinOp::Lt => "slt",
                    BinOp::Le => "sle",
                    BinOp::Gt => "sgt",
                    _ => "sge",
                };
                self.val(format!("icmp {cc} i64 {order}, 0"))
            }
        }
    }

    /// `text.len()`: from the item's tag for a text view, otherwise the
    /// runtime reads the text in place.
    pub(crate) fn str_len(&mut self, arg: &Expr) -> Option<String> {
        if let Some(addr) = self.text_view(arg) {
            return Some(self.text_len(&addr));
        }
        let (text, owned) = self.borrow(arg, true)?;
        let n = self.call_rt("kv_str_len", "i64", &[("i64", text.clone())], false);
        if owned {
            self.release(&text);
        }
        Some(n)
    }

    /// The item address held by `e` when it is a `for` variable that is a
    /// view of an item of a list of text (see `text_views`).
    fn text_view(&mut self, e: &Expr) -> Option<String> {
        match e.kind {
            ExprKind::Local(l) | ExprKind::Move(l) if self.text_views[l as usize] => {
                self.load_local(l)
            }
            _ => None,
        }
    }

    /// The number of characters of the item at `addr`: its tag holds it
    /// for short ASCII text; otherwise the runtime counts.
    pub(crate) fn text_len(&mut self, addr: &str) -> String {
        let high = self.load_item_at(addr, 8);
        let tag = self.val(format!("lshr i64 {high}, 56"));
        let kind = self.val(format!("and i64 {tag}, {}", INLINE | ASCII));
        let short = self.val(format!("icmp eq i64 {kind}, {}", INLINE | ASCII));
        let result = self.slot(Type::I64);
        let (fast, rare, done) = (self.label(), self.label(), self.label());
        self.cond_br(&short, &fast, &rare);
        self.start(&fast);
        let len = self.val(format!("and i64 {tag}, {LEN_MASK}"));
        self.ins(format!("store i64 {len}, ptr {result}"));
        self.br(&done);
        self.start(&rare);
        let len = self.call_rt("kv_text_len", "i64", &[("i64", addr.to_string())], false);
        self.ins(format!("store i64 {len}, ptr {result}"));
        self.br(&done);
        self.start(&done);
        self.val(format!("load i64, ptr {result}"))
    }

    /// The item at `index` of a list of text, as an owned text object.
    pub(crate) fn text_item(&mut self, ptr: &str, index: &str, ty: Type) -> String {
        let addr = self.flat_addr(ptr, index, WIDTH);
        let args = [("i64", addr), ("i64", Self::code(ty))];
        self.call_rt("kv_text_box", "i64", &args, false)
    }

    /// The text pushed by `list.push(text)`: `"…{x}…"` is built right in
    /// an item on the stack (`kv_item_*`), so short text never becomes an
    /// object; anything else is an owned text object.
    pub(crate) fn pushed_text(&mut self, text: &Expr) -> Pushed {
        // A short literal is its item already, and an item of another list
        // of text is copied as it is: neither becomes an object (Sakshi's
        // S7: 2,000,000 pushes of "" took 166 ms).
        match &text.kind {
            ExprKind::Str(s) if s.len() <= SHORT => {
                let item = self.slot_of_type("[2 x i64]");
                let addr = self.val(format!("ptrtoint ptr {item} to i64"));
                for (i, word) in encode(s, Type::I64).iter().enumerate() {
                    self.store_item_at(&(*word as i64).to_string(), &addr, 8 * i as i32);
                }
                return Pushed::Item(addr, Self::code(text.ty));
            }
            ExprKind::Index(x, i) if self.text_list(x.ty) => {
                if let Some(item) = self.copied_item(x, i, text.line) {
                    return Pushed::Item(item, Self::code(text.ty));
                }
            }
            // `push(if c { "1" } else { "0" })`: two short literals, the
            // item's words chosen with no object made (Sakshi's S8: 143 ms
            // for 2,000,000, where a literal took 32).
            ExprKind::If(cond, yes, no) => {
                if let (Some(x), Some(y)) = (short_value(yes), short_value(no)) {
                    let c = self.slot_of(cond);
                    let c = self.val(format!("icmp ne i64 {c}, 0"));
                    let item = self.slot_of_type("[2 x i64]");
                    let addr = self.val(format!("ptrtoint ptr {item} to i64"));
                    let words = encode(x, Type::I64).into_iter().zip(encode(y, Type::I64));
                    for (i, (a, b)) in words.enumerate() {
                        let w =
                            self.val(format!("select i1 {c}, i64 {}, i64 {}", a as i64, b as i64));
                        self.store_item_at(&w, &addr, 8 * i as i32);
                    }
                    return Pushed::Item(addr, Self::code(text.ty));
                }
            }
            _ => {}
        }
        let ExprKind::Format(parts) = &text.kind else {
            return Pushed::Object(self.slot_of(text));
        };
        let item = self.slot_of_type("[2 x i64]");
        let addr = self.val(format!("ptrtoint ptr {item} to i64"));
        let code = Self::code(text.ty);
        let empty = encode("", Type::I64);
        let start = match parts.first().map(|p| &p.kind) {
            Some(ExprKind::Str(s)) if s.len() <= SHORT => encode(s, Type::I64),
            _ => empty,
        };
        for (i, word) in start.iter().enumerate() {
            self.store_item_at(&(*word as i64).to_string(), &addr, 8 * i as i32);
        }
        let skip = usize::from(start != empty);
        for p in &parts[skip..] {
            if let ExprKind::Str(s) = &p.kind {
                let (ptr, len) = self.m.text(s);
                let args = [
                    ("i64", addr.clone()),
                    ("i64", ptr),
                    ("i64", len.to_string()),
                    ("i64", code.clone()),
                ];
                self.call_rt("kv_item_text", "void", &args, false);
                continue;
            }
            let slot = self.slot_of(p);
            let args = [
                ("i64", addr.clone()),
                ("i64", slot),
                ("i64", Self::code(p.ty)),
                ("i64", code.clone()),
            ];
            self.call_rt("kv_item_format", "void", &args, false);
        }
        Pushed::Item(addr, code)
    }

    /// `list.push(text)` on a list of text (both owned): stored in place
    /// when nobody else holds the list and it has room; otherwise the
    /// runtime copies or grows it. Gives the list to keep.
    pub(crate) fn text_push(&mut self, list: &str, text: Pushed, ty: Type, line: u32) -> String {
        let result = self.slot(Type::I64);
        let (room_check, fast, rare, done) =
            (self.label(), self.label(), self.label(), self.label());
        let rc = self.load_at(list, RC);
        let alone = self.val(format!("icmp eq i64 {rc}, 1"));
        self.cond_br(&alone, &room_check, &rare);
        self.start(&room_check);
        let (len, cap) = (self.load_at(list, LEN), self.load_at(list, CAP));
        let room = self.val(format!("icmp ult i64 {len}, {cap}"));
        self.cond_br(&room, &fast, &rare);
        self.start(&fast);
        let buffer = self.load_at(list, PTR);
        let addr = self.flat_addr(&buffer, &len, WIDTH);
        match &text {
            Pushed::Object(p) => {
                let args = [("i64", addr), ("i64", p.clone())];
                self.call_rt("kv_text_store", "void", &args, false);
            }
            Pushed::Item(item, _) => {
                for at in [0, 8] {
                    let word = self.load_item_at(item, at);
                    self.store_item_at(&word, &addr, at);
                }
            }
        }
        let more = self.val(format!("add i64 {len}, 1"));
        self.store_at(&more, list, LEN);
        self.ins(format!("store i64 {list}, ptr {result}"));
        self.br(&done);
        self.start(&rare);
        let object = match text {
            Pushed::Object(p) => p,
            Pushed::Item(item, code) => {
                let args = [("i64", item), ("i64", code)];
                self.call_rt("kv_item_take", "i64", &args, false)
            }
        };
        let grown = self.rt_op(RtOp::ListPush, list, &object, "0", ty, line);
        self.ins(format!("store i64 {grown}, ptr {result}"));
        self.br(&done);
        self.start(&done);
        self.val(format!("load i64, ptr {result}"))
    }
}

impl FnEmit<'_> {
    /// A map key that is an item of a list of text, read where it is so a
    /// short one never becomes an object: its address, and whether it is
    /// a copy (`xs[i]`) whose long text must be let go after, rather than
    /// a `for` variable's view.
    pub(crate) fn key_item(&mut self, key: &Expr) -> Option<(String, bool)> {
        if let Some(addr) = self.text_view(key) {
            return Some((addr, false));
        }
        match &key.kind {
            ExprKind::Index(x, i) if self.text_list(x.ty) => {
                Some((self.copied_item(x, i, key.line)?, true))
            }
            _ => None,
        }
    }

    /// `x[i]` of a list of text, copied onto the stack as an item: its two
    /// words, with one more holder for long text (whose first word is its
    /// object). Gives the item's address.
    fn copied_item(&mut self, x: &Expr, i: &Expr, line: u32) -> Option<String> {
        let (list, owned) = self.borrow(x, crate::heap::plain(i))?;
        let (ptr, len) = self.view(x, &list, owned);
        let index = self.expr(i)?;
        self.check_index(&index, &len, line);
        let from = self.flat_addr(&ptr, &index, WIDTH);
        let item = self.slot_of_type("[2 x i64]");
        let addr = self.val(format!("ptrtoint ptr {item} to i64"));
        let (first, second) = (self.load_item_at(&from, 0), self.load_item_at(&from, 8));
        self.store_item_at(&first, &addr, 0);
        self.store_item_at(&second, &addr, 8);
        let tag = self.val(format!("lshr i64 {second}, 56"));
        let inline = self.val(format!("and i64 {tag}, {INLINE}"));
        let long = self.val(format!("icmp eq i64 {inline}, 0"));
        let (more, done) = (self.label(), self.label());
        self.cond_br(&long, &more, &done);
        self.start(&more);
        self.retain(&first);
        self.br(&done);
        self.start(&done);
        if owned {
            self.release(&list);
        }
        Some(addr)
    }
}

impl FnEmit<'_> {
    /// `xs[i] = ys[j]` or `xs[i] = "short"` into local list of text `l`
    /// (`value` is the `ListSet`): the item copied into its place, the old
    /// one let go, with no object made (Sakshi's S7). Whether it was one.
    pub(crate) fn text_set(&mut self, l: kavach_kir::Local, value: &Expr) -> bool {
        let ExprKind::Rt(kavach_kir::RtOp::ListSet, args) = &value.kind else {
            return false;
        };
        let mine = matches!(args[0].kind, ExprKind::Local(x) | ExprKind::Move(x) if x == l);
        let copied = match &args[2].kind {
            ExprKind::Str(s) => s.len() <= SHORT,
            ExprKind::Index(x, _) => self.text_list(x.ty),
            _ => false,
        };
        if !mine || !copied || !self.text_list(value.ty) {
            return false;
        }
        let (ty, line) = (value.ty, value.line);
        let Some(index) = self.expr(&args[1]) else {
            return false;
        };
        let (item, code) = match self.pushed_text(&args[2]) {
            Pushed::Item(item, code) => (item, code),
            // Made an object after all: the runtime stores it.
            Pushed::Object(p) => {
                let current = self.load_local(l).expect("a list");
                let copy = self.rt_op(kavach_kir::RtOp::ListSet, &current, &index, &p, ty, line);
                self.store_local(l, &copy);
                return true;
            }
        };
        let current = self.load_local(l).expect("a list");
        let result = self.slot(Type::I64);
        let (fast, old_long, store, rare, done) = (
            self.label(),
            self.label(),
            self.label(),
            self.label(),
            self.label(),
        );
        let rc = self.load_at(&current, RC);
        let alone = self.val(format!("icmp eq i64 {rc}, 1"));
        self.cond_br(&alone, &fast, &rare);
        self.start(&fast);
        let len = self.load_at(&current, LEN);
        self.check_index(&index, &len, line);
        let buffer = self.load_at(&current, PTR);
        let addr = self.flat_addr(&buffer, &index, WIDTH);
        let (old, old_tag) = (self.load_item_at(&addr, 0), self.load_item_at(&addr, 8));
        let tag = self.val(format!("lshr i64 {old_tag}, 56"));
        let inline = self.val(format!("and i64 {tag}, {INLINE}"));
        let long = self.val(format!("icmp eq i64 {inline}, 0"));
        self.cond_br(&long, &old_long, &store);
        self.start(&old_long);
        self.release(&old);
        self.br(&store);
        self.start(&store);
        for at in [0, 8] {
            let word = self.load_item_at(&item, at);
            self.store_item_at(&word, &addr, at);
        }
        self.ins(format!("store i64 {current}, ptr {result}"));
        self.br(&done);
        self.start(&rare);
        let args = [("i64", item), ("i64", code)];
        let object = self.call_rt("kv_item_take", "i64", &args, false);
        let copy = self.rt_op(
            kavach_kir::RtOp::ListSet,
            &current,
            &index,
            &object,
            ty,
            line,
        );
        self.ins(format!("store i64 {copy}, ptr {result}"));
        self.br(&done);
        self.start(&done);
        let changed = self.val(format!("load i64, ptr {result}"));
        self.store_local(l, &changed);
        true
    }
}

/// The text of a block that is only a short literal (`else { "0" }` is
/// a block in a block).
fn short_value(b: &kavach_kir::Block) -> Option<&str> {
    if !b.stmts.is_empty() {
        return None;
    }
    match &b.value.as_deref()?.kind {
        ExprKind::Str(s) if s.len() <= SHORT => Some(s),
        ExprKind::Block(inner) => short_value(inner),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    /// Sakshi's S8 (5): `bits.push(if c { "1" } else { "0" })` made a text
    /// object each time. The item's words are chosen with a `select`.
    #[test]
    fn two_short_literals_are_chosen_without_an_object() {
        let src = "fn main() {\n    let bits: [String] = []\n    for i in 0..20 {\n        bits.push(if i % 3 == 0 { \"1\" } else { \"0\" })\n    }\n    print(bits.len())\n}\n";
        let c = kavach_driver::compile(src, "main.kav", Default::default());
        let program = kavach_kir::lower(&c).expect("lowers");
        let ir = crate::emit(&program);
        assert!(ir.contains("select i1"), "{ir}");
    }
}
