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
