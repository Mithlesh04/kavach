//! Expressions. Each gives its value as an LLVM operand, or `None` for
//! `Unit`. A value made in branches (`if`, `and`, `or`) goes through a
//! stack slot, which LLVM turns into a register.

use crate::func::{FnEmit, fn_name};
use kavach_kir::{Block, Expr, ExprKind, Type};

impl FnEmit<'_> {
    pub(crate) fn block(&mut self, b: &Block) -> Option<String> {
        for s in &b.stmts {
            self.stmt(s);
        }
        b.value.as_deref().and_then(|v| self.expr(v))
    }

    pub(crate) fn expr(&mut self, e: &Expr) -> Option<String> {
        if let Some(v) = self.data_expr(e) {
            return v;
        }
        match &e.kind {
            // In two's complement at the type's width, as LLVM reads it.
            ExprKind::Int(n) => {
                let shift = 64 - e.ty.bits().max(1);
                Some((((*n as i64) << shift) >> shift).to_string())
            }
            // A float constant as its exact bits, as LLVM wants.
            // A `Float32` constant too: LLVM writes it as the double it widens to.
            ExprKind::Float(f) if e.ty == Type::F32 => {
                Some(format!("0x{:016X}", f64::from(*f as f32).to_bits()))
            }
            ExprKind::Float(f) => Some(format!("0x{:016X}", f.to_bits())),
            ExprKind::Bool(v) => Some(v.to_string()),
            ExprKind::Char(c) => Some((*c as u32).to_string()),
            ExprKind::Local(l) => {
                let v = self.load_local(*l)?;
                if e.ty.is_ref() {
                    self.retain(&v);
                }
                Some(v)
            }
            // The last read: hand the value over and empty the variable.
            ExprKind::Move(l) => {
                let v = self.load_local(*l)?;
                if e.ty.is_ref() && self.borrowed[*l as usize] {
                    // It holds no reference of its own: the value given out
                    // is a new one.
                    self.retain(&v);
                } else if e.ty.is_ref() {
                    self.store_local(*l, "0");
                }
                Some(v)
            }
            ExprKind::Assign(l, value) => {
                self.assign(*l, value);
                None
            }
            ExprKind::Str(text) => Some(self.new_text(text, e.ty)),
            ExprKind::Format(parts) => Some(self.format(parts, e.ty)),
            ExprKind::Binary(op, l, r) if l.ty.is_ref() => {
                let (a, b) = (self.expr(l)?, self.expr(r)?);
                Some(self.ref_binary(*op, l.ty, &a, &b))
            }
            ExprKind::Rt(kavach_kir::RtOp::StrLen, args) => self.str_len(&args[0]),
            // An item of a list of text, as a text object.
            ExprKind::Index(x, i) if self.text_list(x.ty) => {
                let (list, owned) = self.borrow(x, crate::heap::plain(i))?;
                let (ptr, len) = self.view(x, &list, owned);
                let index = self.expr(i)?;
                self.check_index(&index, &len, e.line);
                let item = self.text_item(&ptr, &index, e.ty);
                if owned {
                    self.release(&list);
                }
                Some(item)
            }
            ExprKind::Rt(kavach_kir::RtOp::ListPush, args) if self.text_list(e.ty) => {
                let list = self.slot_of(&args[0]);
                let text = self.pushed_text(&args[1]);
                Some(self.text_push(&list, text, e.ty, e.line))
            }
            // A flat list's items are stored by the runtime, field by field;
            // so are a list of text's.
            ExprKind::List(items) if self.flat_width(e.ty) > 0 || self.text_list(e.ty) => {
                let n = items.len().to_string();
                let args = [("i64", Self::code(e.ty)), ("i64", n)];
                let list = self.call_rt("kv_list", "i64", &args, false);
                for (i, item) in items.iter().enumerate() {
                    let slot = self.slot_of(item);
                    let args = [("i64", list.clone()), ("i64", i.to_string()), ("i64", slot)];
                    self.call_rt("kv_init", "void", &args, false);
                }
                Some(list)
            }
            ExprKind::Record { fields, .. } | ExprKind::Tuple(fields) => {
                let slots: Vec<String> = fields.iter().map(|f| self.slot_of(f)).collect();
                Some(self.new_fixed(e.ty, &slots))
            }
            ExprKind::GetField(x, f) => self.get_field(x, *f, e.ty),
            // Element `n` of a tuple.
            ExprKind::Index(x, i) if self.fixed(x.ty) => {
                let ExprKind::Int(n) = i.kind else {
                    unreachable!("a constant index")
                };
                self.get_field(x, n as u32, e.ty)
            }
            // An item of a flat list, used whole: boxed.
            ExprKind::Index(x, i) if self.flat_width(x.ty) > 0 => {
                let (list, owned) = self.borrow(x, crate::heap::plain(i))?;
                let (ptr, len) = self.view(x, &list, owned);
                let index = self.expr(i)?;
                if owned || !self.known_in_range(x, i) {
                    self.check_index(&index, &len, e.line);
                }
                let width = self.flat_width(x.ty);
                let addr = self.flat_addr(&ptr, &index, width);
                let item = self.flat_box(&addr, e.ty, width);
                if owned {
                    self.release(&list);
                }
                Some(item)
            }
            ExprKind::SetField(x, i, value) => {
                let obj = self.expr(x)?;
                let slot = self.slot_of(value);
                let args = [("i64", obj), ("i64", i.to_string()), ("i64", slot)];
                Some(self.call_rt("kv_set", "i64", &args, false))
            }
            ExprKind::List(items) => {
                let n = items.len().to_string();
                let list = self.call_rt(
                    "kv_list",
                    "i64",
                    &[("i64", Self::code(e.ty)), ("i64", n)],
                    false,
                );
                let packed = self.item_bytes_of(e.ty) > 0;
                for (i, item) in items.iter().enumerate() {
                    let slot = self.slot_of(item);
                    if packed {
                        // The runtime packs the items (0030, 0045).
                        let args = [("i64", list.clone()), ("i64", i.to_string()), ("i64", slot)];
                        self.call_rt("kv_init", "void", &args, false);
                        continue;
                    }
                    let buffer = self.load_at(&list, kavach_runtime_native::heap::PTR);
                    self.store_item_at(&slot, &buffer, 8 * i as i32);
                }
                Some(list)
            }
            ExprKind::Index(x, i) | ExprKind::InRange(x, i) => {
                let (list, owned) = self.borrow(x, crate::heap::plain(i))?;
                let view = self.view(x, &list, owned);
                let index = self.expr(i)?;
                let checked =
                    matches!(e.kind, ExprKind::Index(..)) && (owned || !self.known_in_range(x, i));
                let item = self.item(view, &index, e.ty, e.line, checked);
                if e.ty.is_ref() {
                    self.retain(&item);
                }
                if owned {
                    self.release(&list);
                }
                Some(item)
            }
            ExprKind::Rt(kavach_kir::RtOp::ListPrefetch, args) => {
                self.prefetch(&args[0], &args[1]);
                None
            }
            // Wrapping arithmetic (`a.wrapping_add(b)`, spec 5.6): one
            // instruction, no overflow check.
            ExprKind::Rt(
                op @ (kavach_kir::RtOp::WrappingAdd
                | kavach_kir::RtOp::WrappingSub
                | kavach_kir::RtOp::WrappingMul),
                args,
            ) if e.ty == Type::I64 => {
                let (a, b) = (self.expr(&args[0])?, self.expr(&args[1])?);
                let name = match op {
                    kavach_kir::RtOp::WrappingAdd => "add",
                    kavach_kir::RtOp::WrappingSub => "sub",
                    _ => "mul",
                };
                Some(self.val(format!("{name} i64 {a}, {b}")))
            }
            ExprKind::Rt(kavach_kir::RtOp::ListLen, args) => {
                let (list, owned) = self.borrow(&args[0], true)?;
                let n = self.list_len(&list);
                if owned {
                    self.release(&list);
                }
                Some(n)
            }
            ExprKind::Map(pairs) => {
                let code = Self::code(e.ty);
                let mut map = self.call_rt("kv_map_new", "i64", &[("i64", code)], false);
                for (k, v) in pairs {
                    let (ks, vs) = (self.slot_of(k), self.slot_of(v));
                    let op = kavach_kir::RtOp::MapInsert;
                    map = self.rt_op(op, &map, &ks, &vs, e.ty, e.line);
                }
                Some(map)
            }
            // `m[k] ?? d` for an integer key and a scalar value: the map is
            // read in place, without the generic entry's dispatch.
            ExprKind::Rt(kavach_kir::RtOp::MapGetOr, args)
                if self.int_keys(args[0].ty) && !e.ty.is_ref() =>
            {
                let (map, owned) = self.borrow(&args[0], true)?;
                let (key, fallback) = (self.slot_of(&args[1]), self.slot_of(&args[2]));
                let args3 = [("i64", map.clone()), ("i64", key), ("i64", fallback)];
                let slot = self.call_rt("kv_map_get_int", "i64", &args3, false);
                if owned {
                    self.release(&map);
                }
                Some(self.unpack(&slot, e.ty))
            }
            // `m[k]` for a text key and a value held by reference: the map
            // is read in place, and a constant key needs no text object.
            ExprKind::Rt(kavach_kir::RtOp::MapGet, args) if self.text_keys(args[0].ty) => {
                let (map, owned) = self.borrow(&args[0], true)?;
                let slot = if let ExprKind::Str(key) = &args[1].kind {
                    let (ptr, len) = self.m.text(key);
                    let args3 = [("i64", map.clone()), ("i64", ptr), ("i64", len.to_string())];
                    self.call_rt("kv_map_get_text", "i64", &args3, false)
                } else {
                    let (key, key_owned) = self.borrow(&args[1], true)?;
                    let args2 = [("i64", map.clone()), ("i64", key.clone())];
                    let slot = self.call_rt("kv_map_get_str", "i64", &args2, false);
                    if key_owned {
                        self.release(&key);
                    }
                    slot
                };
                if owned {
                    self.release(&map);
                }
                Some(slot)
            }
            ExprKind::Rt(kavach_kir::RtOp::ListPush, args) if self.flat_width(e.ty) > 0 => {
                let list = self.slot_of(&args[0]);
                let fields = self.flat_fields(&args[1], self.flat_width(e.ty));
                Some(self.flat_push(&list, &fields, e.ty, e.line))
            }
            ExprKind::Rt(kavach_kir::RtOp::ListPush, args) => {
                let (list, item) = (self.slot_of(&args[0]), self.slot_of(&args[1]));
                Some(self.list_push(&list, &item, e.ty, e.line))
            }
            ExprKind::Rt(kavach_kir::RtOp::ListSet, args) if self.plain_items(args[0].ty) => {
                let (list, index, item) = (
                    self.slot_of(&args[0]),
                    self.slot_of(&args[1]),
                    self.slot_of(&args[2]),
                );
                Some(self.list_set(&list, &index, &item, e.ty, e.line))
            }
            // `math.sqrt` and friends: one exactly rounded instruction,
            // not a runtime call (the runtime's Rust calls round the
            // same way, so results don't change).
            ExprKind::Rt(
                op @ (kavach_kir::RtOp::MathSqrt
                | kavach_kir::RtOp::MathFloor
                | kavach_kir::RtOp::MathCeil
                | kavach_kir::RtOp::MathRound),
                args,
            ) => {
                let name = match op {
                    kavach_kir::RtOp::MathSqrt => "llvm.sqrt.f64",
                    kavach_kir::RtOp::MathFloor => "llvm.floor.f64",
                    kavach_kir::RtOp::MathCeil => "llvm.ceil.f64",
                    _ => "llvm.round.f64",
                };
                let x = self.expr(&args[0])?;
                Some(self.call_rt(name, "double", &[("double", x)], false))
            }
            // Any other built-in method: the runtime's generic entry, with
            // every operand handed over as a slot.
            ExprKind::Rt(kavach_kir::RtOp::PtrRead | kavach_kir::RtOp::PtrWrite, args) => {
                self.ptr_access(e.ty, args)
            }
            ExprKind::Rt(op @ (kavach_kir::RtOp::ParFor | kavach_kir::RtOp::ParForRed), args) => {
                let lo = self.expr(&args[0])?;
                let hi = self.expr(&args[1])?;
                let ExprKind::Int(chunk) = args[2].kind else {
                    unreachable!("the chunk's number is a constant")
                };
                let caps = self.slot_array(&args[3..]);
                let name = fn_name(chunk as usize);
                let rt = match op {
                    kavach_kir::RtOp::ParForRed => "kv_par_for_red",
                    _ => "kv_par_for",
                };
                let list = [
                    ("i64", lo),
                    ("i64", hi),
                    ("ptr", format!("@{name}")),
                    ("ptr", caps),
                    ("i64", (args.len() - 3).to_string()),
                ];
                Some(self.call_rt(rt, "i64", &list, false))
            }
            ExprKind::Rt(op, args) => {
                let mut slots: Vec<String> = args.iter().map(|a| self.slot_of(a)).collect();
                slots.resize(3, "0".into());
                let slot = self.rt_op(*op, &slots[0], &slots[1], &slots[2], e.ty, e.line);
                (e.ty != Type::Unit).then(|| self.unpack(&slot, e.ty))
            }
            ExprKind::Unary(op, x) => {
                let v = self.expr(x)?;
                Some(self.unary(*op, x.ty, &v, e.line))
            }
            ExprKind::Binary(op, l, r) => {
                let (a, b) = (self.expr(l)?, self.expr(r)?);
                Some(self.binary(*op, l.ty, &a, &b, e.line))
            }
            ExprKind::Fast(op, l, r) => {
                let (a, b) = (self.expr(l)?, self.expr(r)?);
                Some(self.fast(*op, l.ty, &a, &b))
            }
            ExprKind::And(l, r) | ExprKind::Or(l, r) => {
                Some(self.short_circuit(matches!(e.kind, ExprKind::And(..)), l, r))
            }
            ExprKind::Call(f, args) => self.call(*f as usize, args, e.ty, e.line),
            ExprKind::If(cond, then, otherwise) => self.if_else(cond, then, otherwise, e.ty),
            ExprKind::Fallback(fast, slow) => self.fallback(fast, slow, e.ty),
            ExprKind::Block(b) => {
                let v = self.block(b);
                if e.ty == Type::Unit {
                    None
                } else {
                    v.or_else(|| Self::zero(e.ty))
                }
            }
            ExprKind::Convert(x, checked) => {
                let v = self.expr(x)?;
                Some(self.convert(&v, x.ty, e.ty, *checked, e.line))
            }
            ExprKind::Panic(message) => {
                self.panic(message, e.line);
                Self::zero(e.ty)
            }
            _ => unreachable!("checked by `check.rs`"),
        }
    }

    /// `local = value`. `local.push(x)` written back to the same local
    /// moves the local's reference into the push, so an unshared list
    /// grows in place instead of being copied.
    fn assign(&mut self, l: kavach_kir::Local, value: &Expr) {
        // The receiver is the local itself, read or moved (as in the
        // Cranelift backend's `is_local`).
        let is_l = |e: &Expr| {
            matches!(e.kind, ExprKind::Local(x) | ExprKind::Move(x) if x == l) && e.ty.is_ref()
        };
        if self.assign_flat_set(l, value) {
            return;
        }
        if let ExprKind::Rt(kavach_kir::RtOp::ListSet, args) = &value.kind {
            // A store proven safe for the whole loop (`stmt.rs`): in
            // range, and the list unshared, so it goes straight in.
            let raw = self
                .raw_sets
                .iter()
                .find(|(list, var, _)| {
                    *list == l && matches!(args[1].kind, ExprKind::Local(x) if x == *var)
                })
                .map(|(_, _, ptr)| ptr.clone());
            if let Some(ptr) = raw {
                let i = self.expr(&args[1]).expect("an index");
                let slot = self.slot_of(&args[2]);
                let base = self.val(format!("inttoptr i64 {ptr} to ptr"));
                if let size @ 1.. = self.item_bytes_of(value.ty) {
                    return self.store_packed(&base, &i, &slot, size);
                }
                let at = self.val(format!("getelementptr i64, ptr {base}, i64 {i}"));
                self.ins(format!("store i64 {slot}, ptr {at}, {}", crate::heap::ITEM));
                return;
            }
        }
        if let ExprKind::Rt(kavach_kir::RtOp::ListPush, args) = &value.kind {
            // A push with its room already reserved (`stmt.rs`): one
            // store, the length written after the loop.
            let raw = self
                .raw_pushes
                .iter()
                .find(|(list, _, _, _)| *list == l)
                .map(|(_, var, ptr, off)| (*var, ptr.clone(), off.clone()));
            if let Some((var, ptr, off)) = raw
                && is_l(&args[0])
            {
                let slot = self.slot_of(&args[1]);
                let i = self.load_local(var).expect("the loop variable");
                let at = self.val(format!("add i64 {i}, {off}"));
                let base = self.val(format!("inttoptr i64 {ptr} to ptr"));
                if let size @ 1.. = self.item_bytes_of(value.ty) {
                    return self.store_packed(&base, &at, &slot, size);
                }
                let spot = self.val(format!("getelementptr i64, ptr {base}, i64 {at}"));
                self.ins(format!(
                    "store i64 {slot}, ptr {spot}, {}",
                    crate::heap::ITEM
                ));
                return;
            }
            if self.text_list(value.ty) && is_l(&args[0]) {
                let text = self.pushed_text(&args[1]);
                let current = self.load_local(l).expect("a list");
                let changed = self.text_push(&current, text, value.ty, value.line);
                self.store_local(l, &changed);
                return;
            }
            let width = self.flat_width(value.ty);
            if width > 0 && is_l(&args[0]) {
                // A record built right here goes in field by field.
                let fields = self.flat_fields(&args[1], width);
                let current = self.load_local(l).expect("a list");
                let changed = self.flat_push(&current, &fields, value.ty, value.line);
                self.store_local(l, &changed);
                return;
            }
        }
        if let ExprKind::SetField(base, i, field) = &value.kind
            && is_l(base)
            && self.fixed(base.ty)
        {
            // `p.x = v`: the local's reference moves into the change.
            let slot = self.slot_of(field);
            let current = self.load_local(l).expect("a record");
            let changed = self.set_field(&current, *i, &slot, field.ty);
            self.store_local(l, &changed);
            return;
        }
        if let ExprKind::Rt(op, args) = &value.kind
            && op.changes_receiver()
            && is_l(&args[0])
        {
            // The other operands first: they may read the local.
            let mut slots: Vec<String> = args[1..].iter().map(|a| self.slot_of(a)).collect();
            slots.resize(2, "0".into());
            let current = self.load_local(l).expect("a list");
            let changed = match op {
                kavach_kir::RtOp::ListPush => {
                    self.list_push(&current, &slots[0], value.ty, value.line)
                }
                kavach_kir::RtOp::ListSet if self.plain_items(value.ty) => {
                    self.list_set(&current, &slots[0], &slots[1], value.ty, value.line)
                }
                kavach_kir::RtOp::MapInsert => {
                    let args = [
                        ("i64", current.clone()),
                        ("i64", slots[0].clone()),
                        ("i64", slots[1].clone()),
                    ];
                    self.call_rt("kv_map_insert", "i64", &args, false)
                }
                _ => self.rt_op(*op, &current, &slots[0], &slots[1], value.ty, value.line),
            };
            self.store_local(l, &changed);
            return;
        }
        if let Some(v) = self.expr(value) {
            self.set_local(l, &v);
        }
    }

    /// `e` as a 64-bit slot (owned).
    pub(crate) fn slot_of(&mut self, e: &Expr) -> String {
        match self.expr(e) {
            Some(v) => self.pack(&v, e.ty),
            None => "0".into(),
        }
    }
}
