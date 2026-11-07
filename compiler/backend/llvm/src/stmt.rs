//! Statements: `let`, loops, `return`, `print` and contracts. Loops keep
//! the semantics of the Cranelift backend (`cranelift/src/loops.rs`): a
//! range's end is computed once, and `..=` stops before `+ 1` overflows.

use crate::func::FnEmit;
use crate::module::llvm_type;
use kavach_kir::{BinOp, Block, Expr, ExprKind, FastOp, Local, PrintPart, Stmt, Type, walk};
use kavach_runtime_native::heap::{LEN, PTR, RC};

/// Whether `b` has a loop inside, at any depth.
fn contains_loop(b: &Block) -> bool {
    let is_loop = |s: &Stmt| {
        matches!(
            s,
            Stmt::While { .. } | Stmt::ForRange { .. } | Stmt::ForEach { .. }
        )
    };
    b.stmts.iter().any(is_loop)
        || walk::any_in_block(b, &mut |e| match &e.kind {
            ExprKind::If(_, t, o) | ExprKind::Fallback(t, o) => {
                contains_loop(t) || contains_loop(o)
            }
            ExprKind::Block(x) => contains_loop(x),
            _ => false,
        })
}

impl FnEmit<'_> {
    pub(crate) fn stmt(&mut self, s: &Stmt) {
        match s {
            // A capture a closure's function borrows: read in place.
            Stmt::Let(local, e) if self.borrowed[*local as usize] && self.is_place(e) => {
                let v = self.place(e);
                self.store_local(*local, &v);
            }
            Stmt::Let(local, e) => {
                if let Some(v) = self.expr(e) {
                    self.set_local(*local, &v);
                }
            }
            // Evaluated for its effects; a heap result is released.
            Stmt::Expr(e) => {
                if let Some(v) = self.expr(e)
                    && e.ty.is_ref()
                {
                    self.release(&v);
                }
            }
            Stmt::While { cond, body } => {
                let (test, body_label, exit) = (self.label(), self.label(), self.label());
                self.br(&test);
                self.start(&test);
                let c = self.expr(cond).expect("a condition");
                self.cond_br(&c, &body_label, &exit);
                self.start(&body_label);
                self.loops.push((test.clone(), exit.clone()));
                self.block(body);
                self.loops.pop();
                let back = self.loop_hint(body);
                self.ins(format!("br label %{test}{back}"));
                self.start(&exit);
            }
            Stmt::ForRange {
                var,
                start,
                end,
                inclusive,
                body,
            } => self.for_range(*var, start, end, *inclusive, body),
            Stmt::Return(value) => {
                let v = value.as_ref().and_then(|e| self.expr(e));
                self.ret(v);
            }
            Stmt::Break | Stmt::Continue => {
                let (next, exit) = self.loops.last().cloned().expect("inside a loop");
                self.br(if matches!(s, Stmt::Break) {
                    &exit
                } else {
                    &next
                });
                self.dead();
            }
            Stmt::Print(parts) => {
                for part in parts {
                    match part {
                        PrintPart::Text(t) => self.print_text(t),
                        PrintPart::Value(e) => self.print_value(e),
                    }
                }
                self.call_rt("kv_print_end", "void", &[], false);
            }
            Stmt::Check {
                cond,
                message,
                line,
            } => {
                let c = self.expr(cond).expect("a condition");
                let (ok, fail) = (self.label(), self.label());
                self.cond_br(&c, &ok, &fail);
                self.start(&fail);
                self.panic(message, *line);
                self.start(&ok);
            }
            Stmt::ForEach {
                var,
                index,
                list,
                body,
            } => self.for_each(*var, *index, *list, body),
            Stmt::LetResult { value, error, call } => self.let_result(*value, *error, call),
        }
    }

    fn for_range(&mut self, var: u32, start: &Expr, end: &Expr, inclusive: bool, body: &Block) {
        let ty = start.ty;
        let t = llvm_type(ty);
        let (s, e) = (
            self.expr(start).expect("a start"),
            self.expr(end).expect("an end"),
        );
        let last = self.slot(ty);
        self.ins(format!("store {t} {e}, ptr {last}"));
        self.store_local(var, &s);
        let held = self.hold_steady(body);
        let safe = self.safe_indexes(var, ty, body);
        let sets = self.raw_set_targets(var, body);
        let pushes = if inclusive {
            Vec::new()
        } else {
            self.raw_push_targets(var, body)
        };
        if safe.is_empty() && sets.is_empty() && pushes.is_empty() {
            self.range_loop(var, &last, ty, inclusive, body);
            return self.let_go(held);
        }
        // Check once that every `list[var]` stays in range for the whole
        // loop (and that a list written by `var` is unshared, so stores
        // go in place); then run a copy of the loop without those checks.
        let cc = if inclusive { "slt" } else { "sle" };
        let mut ok = self.val(format!("icmp sge i64 {s}, 0"));
        for &list in &safe {
            let (_, len) = self.steady[list as usize].clone().expect("a steady list");
            let len = self.val(format!("load i64, ptr {len}"));
            let fits = self.val(format!("icmp {cc} i64 {e}, {len}"));
            ok = self.val(format!("and i1 {ok}, {fits}"));
        }
        let mut raw = Vec::new();
        for &list in &sets {
            let value = self.load_local(list).expect("a list");
            let rc = self.load_at(&value, RC);
            let alone = self.val(format!("icmp eq i64 {rc}, 1"));
            let len = self.load_at(&value, LEN);
            let fits = self.val(format!("icmp {cc} i64 {e}, {len}"));
            let both = self.val(format!("and i1 {alone}, {fits}"));
            ok = self.val(format!("and i1 {ok}, {both}"));
            let ptr = self.load_at(&value, PTR);
            raw.push((list, var, ptr));
        }
        if !pushes.is_empty() {
            // The pushes run once per round, so the loop must count up.
            let counts = self.val(format!("icmp sle i64 {s}, {e}"));
            ok = self.val(format!("and i1 {ok}, {counts}"));
            for &list in &pushes {
                let value = self.load_local(list).expect("a list");
                let rc = self.load_at(&value, RC);
                let alone = self.val(format!("icmp eq i64 {rc}, 1"));
                ok = self.val(format!("and i1 {ok}, {alone}"));
            }
        }
        let (fast, slow, done) = (self.label(), self.label(), self.label());
        self.cond_br(&ok, &fast, &slow);
        self.start(&fast);
        // Room for the whole loop once; then the stores need no checks,
        // and the length is written once at the end.
        let count = self.val(format!("sub i64 {e}, {s}"));
        let mut grown = Vec::new();
        for &list in &pushes {
            self.m
                .declare("kv_list_reserve", "void", &["i64", "i64"], false);
            let value = self.load_local(list).expect("a list");
            self.ins(format!(
                "call void @kv_list_reserve(i64 {value}, i64 {count})"
            ));
            let ptr = self.load_at(&value, PTR);
            let len0 = self.load_at(&value, LEN);
            let off = self.val(format!("sub i64 {len0}, {s}"));
            let after = self.val(format!("add i64 {len0}, {count}"));
            self.raw_pushes.push((list, var, ptr, off));
            grown.push((value, after));
        }
        let before = self.unchecked.len();
        self.unchecked.extend(safe.iter().map(|&l| (l, var)));
        let before_raw = self.raw_sets.len();
        self.raw_sets.extend(raw);
        let before_push = self.raw_pushes.len() - pushes.len();
        self.range_loop(var, &last, ty, inclusive, body);
        self.raw_pushes.truncate(before_push);
        for (value, after) in grown {
            self.store_at(&after, &value, LEN);
        }
        self.raw_sets.truncate(before_raw);
        self.unchecked.truncate(before);
        self.br(&done);
        self.start(&slow);
        self.range_loop(var, &last, ty, inclusive, body);
        self.br(&done);
        self.start(&done);
        self.let_go(held);
    }

    /// The loop of `for var in …`, its first value already in `var` and
    /// its end in the slot `last`.
    fn range_loop(&mut self, var: u32, last: &str, ty: Type, inclusive: bool, body: &Block) {
        let t = llvm_type(ty);
        let (body_label, step, exit) = (self.label(), self.label(), self.label());
        let cur = self.load_local(var).expect("an integer");
        let end_value = self.val(format!("load {t}, ptr {last}"));
        let go = self.compare(BinOp::Lt, ty, &cur, &end_value, inclusive);
        self.cond_br(&go, &body_label, &exit);
        self.start(&body_label);
        self.loops.push((step.clone(), exit.clone()));
        self.block(body);
        self.loops.pop();
        self.br(&step);
        self.start(&step);
        let cur = self.load_local(var).expect("an integer");
        let end_value = self.val(format!("load {t}, ptr {last}"));
        let next = self.val(format!("add {t} {cur}, 1"));
        self.store_local(var, &next);
        let go = if inclusive {
            // `..=` up to the largest value: stop before `+ 1` overflows.
            self.val(format!("icmp ne {t} {cur}, {end_value}"))
        } else {
            self.compare(BinOp::Lt, ty, &next, &end_value, false)
        };
        let back = self.loop_hint(body);
        self.ins(format!(
            "br i1 {go}, label %{body_label}, label %{exit}{back}"
        ));
        self.start(&exit);
    }

    /// For the branch back to a loop's start: `, !llvm.loop !N` when the
    /// loop divides whole numbers, which then stays scalar
    /// (`Module::scalar_loop`).
    fn loop_hint(&mut self, body: &Block) -> String {
        let divides = |e: &Expr| match &e.kind {
            ExprKind::Binary(BinOp::Div | BinOp::Rem, ..) => e.ty.is_int(),
            ExprKind::Fast(op, ..) => matches!(
                op,
                FastOp::DivPositive | FastOp::RemPositive | FastOp::DivSmall | FastOp::RemSmall
            ),
            _ => false,
        };
        if walk::any_in_block(body, &mut { divides }) {
            format!(", !llvm.loop {}", self.m.scalar_loop())
        } else {
            String::new()
        }
    }

    /// `for var in list`, where `list` is a hidden local that holds the
    /// list and doesn't change during the loop: its buffer and length are
    /// read once. The variable borrows each item unless the body takes it
    /// away or replaces it (as in the Cranelift backend).
    fn for_each(&mut self, var: Local, index: Local, list: Local, body: &Block) {
        let ty = self.f.locals[var as usize];
        let (moves, uses) = crate::records::uses_of(body, var);
        self.borrowed[var as usize] = ty.is_ref() && moves == 0;
        // Over a flat list, the variable is a view of each item when the
        // body only reads its fields; otherwise each item is boxed.
        let width = self.flat_width(self.f.locals[list as usize]);
        if width > 0 && uses > 0 {
            self.borrowed[var as usize] = false;
        }
        // Over a list of text, the variable is the address of each item
        // when the body only asks its length; otherwise each is boxed.
        let text = self.text_list(self.f.locals[list as usize]);
        if text {
            let view = moves == 0 && uses == 0;
            self.text_views[var as usize] = view;
            self.borrowed[var as usize] = view;
        }
        self.store_local(index, "0");
        let l = self.load_local(list).expect("a list");
        let view = (self.load_at(&l, PTR), self.load_at(&l, LEN));
        let (body_label, step, exit) = (self.label(), self.label(), self.label());
        let any = self.val(format!("icmp slt i64 0, {}", view.1));
        self.cond_br(&any, &body_label, &exit);
        self.start(&body_label);
        let i = self.load_local(index).expect("an index");
        let item = if text {
            if self.text_views[var as usize] {
                let width = kavach_runtime_native::text_items::WIDTH;
                self.flat_addr(&view.0, &i, width)
            } else {
                self.text_item(&view.0, &i, ty)
            }
        } else if width > 0 {
            let addr = self.flat_addr(&view.0, &i, width);
            if self.borrowed[var as usize] {
                self.flat_view(&addr)
            } else {
                self.flat_box(&addr, ty, width)
            }
        } else {
            let item = self.item(view.clone(), &i, ty, 0, false);
            if ty.is_ref() && !self.borrowed[var as usize] {
                self.retain(&item);
            }
            item
        };
        self.set_local(var, &item);
        self.loops.push((step.clone(), exit.clone()));
        self.block(body);
        self.loops.pop();
        self.br(&step);
        self.start(&step);
        let i = self.load_local(index).expect("an index");
        let next = self.val(format!("add i64 {i}, 1"));
        self.store_local(index, &next);
        let more = self.val(format!("icmp slt i64 {next}, {}", view.1));
        self.cond_br(&more, &body_label, &exit);
        self.start(&exit);
    }

    /// The lists that the body of an innermost `for var in …` loop indexes
    /// with `var` itself (`list[var]`) and never changes: one check before
    /// the loop covers all their bounds checks.
    fn safe_indexes(&self, var: Local, ty: Type, body: &Block) -> Vec<Local> {
        if ty != Type::I64 || contains_loop(body) || walk::sets_local(body, var) {
            return Vec::new();
        }
        let mut lists = Vec::new();
        walk::any_in_block(body, &mut |e| {
            if let ExprKind::Index(base, i) = &e.kind
                && let (ExprKind::Local(l), ExprKind::Local(x)) = (&base.kind, &i.kind)
                && *x == var
                && self.steady[*l as usize].is_some()
                && !lists.contains(l)
            {
                lists.push(*l);
            }
            false
        });
        lists.retain(|&l| !walk::sets_local(body, l));
        lists
    }

    fn print_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let (ptr, len) = self.m.text(text);
        self.call_rt(
            "kv_print_str",
            "void",
            &[("i64", ptr), ("i64", len.to_string())],
            false,
        );
    }

    fn print_value(&mut self, e: &Expr) {
        let Some(v) = self.expr(e) else { return };
        match e.ty {
            Type::F64 => {
                self.call_rt("kv_print_f64", "void", &[("double", v)], false);
            }
            Type::Bool => {
                let byte = self.val(format!("zext i1 {v} to i8"));
                self.call_rt("kv_print_bool", "void", &[("i8", byte)], false);
            }
            // The runtime prints it, and releases a heap value.
            Type::Char | Type::Ref(_) | Type::F32 => {
                let slot = self.pack(&v, e.ty);
                let args = [("i64", slot), ("i64", Self::code(e.ty))];
                self.call_rt("kv_print_value", "void", &args, false);
            }
            t => {
                let wide = self.widen(&v, t);
                let name = if t.is_signed() {
                    "kv_print_i64"
                } else {
                    "kv_print_u64"
                };
                self.call_rt(name, "void", &[("i64", wide)], false);
            }
        }
    }
}

mod raw;
