//! Loops over number ranges and lists.

use crate::abi::cl_type;
use crate::codegen::FnGen;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, types};
use kavach_kir::{Block, Expr, ExprKind, Local};
use kavach_runtime_native::heap::{LEN, PTR};

impl FnGen<'_, '_> {
    pub(crate) fn for_range(
        &mut self,
        var: u32,
        start: &Expr,
        end: &Expr,
        inclusive: bool,
        body: &Block,
    ) {
        let ty = start.ty;
        let cty = cl_type(ty).expect("an integer");
        let (s, e) = (
            self.expr(start).expect("a start"),
            self.expr(end).expect("an end"),
        );
        let end_var = self.b.declare_var(cty);
        self.b.def_var(end_var, e);
        let v = self.vars[var as usize];
        self.b.def_var(v, s);
        let held = self.hold_steady(body);
        let exit = self.b.create_block();
        let safe = self.safe_indexes(var, ty, body);
        if safe.is_empty() {
            self.range_loop(var, end_var, ty, inclusive, body, exit);
        } else {
            // Check once that every `list[var]` stays in range for the whole
            // loop; then run a copy of the loop without those checks.
            let (fast, slow) = (self.b.create_block(), self.b.create_block());
            let first = self.b.use_var(v);
            let last = self.b.use_var(end_var);
            let mut ok = self
                .b
                .ins()
                .icmp_imm(IntCC::SignedGreaterThanOrEqual, first, 0);
            for &list in &safe {
                let (_, len) = self.steady[list as usize].expect("a steady list");
                let len = self.b.use_var(len);
                let cc = if inclusive {
                    IntCC::SignedLessThan
                } else {
                    IntCC::SignedLessThanOrEqual
                };
                let fits = self.b.ins().icmp(cc, last, len);
                ok = self.b.ins().band(ok, fits);
            }
            self.b.ins().brif(ok, fast, &[], slow, &[]);
            self.b.switch_to_block(fast);
            let before = self.unchecked.len();
            self.unchecked.extend(safe.iter().map(|&l| (l, var)));
            self.range_loop(var, end_var, ty, inclusive, body, exit);
            self.unchecked.truncate(before);
            self.b.switch_to_block(slow);
            self.range_loop(var, end_var, ty, inclusive, body, exit);
        }
        self.let_go(held);
        self.b.switch_to_block(exit);
    }

    /// The loop of `for var in …` (bounds already set), leaving to `exit`.
    /// The test is made once before the loop, then at the bottom of each
    /// round, so a round takes one branch instead of two.
    fn range_loop(
        &mut self,
        var: Local,
        end_var: cranelift_frontend::Variable,
        ty: kavach_kir::Type,
        inclusive: bool,
        body: &Block,
        exit: cranelift_codegen::ir::Block,
    ) {
        let v = self.vars[var as usize];
        let (header, body_block, step) = (
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
        );
        self.b.ins().jump(header, &[]);
        self.b.switch_to_block(header);
        let (cur, last) = (self.b.use_var(v), self.b.use_var(end_var));
        let go = self.compare(kavach_kir::BinOp::Lt, ty, cur, last, inclusive);
        self.b.ins().brif(go, body_block, &[], exit, &[]);
        self.b.switch_to_block(body_block);
        self.loops.push((step, exit));
        self.block(body);
        self.loops.pop();
        self.b.ins().jump(step, &[]);
        self.b.switch_to_block(step);
        let (cur, last) = (self.b.use_var(v), self.b.use_var(end_var));
        if inclusive {
            // `..=` up to the largest value: stop before `+ 1` overflows.
            let more = self.b.ins().icmp(IntCC::NotEqual, cur, last);
            let next = self.b.ins().iadd_imm(cur, 1);
            self.b.def_var(v, next);
            self.b.ins().brif(more, body_block, &[], exit, &[]);
        } else {
            let next = self.b.ins().iadd_imm(cur, 1);
            self.b.def_var(v, next);
            let go = self.compare(kavach_kir::BinOp::Lt, ty, next, last, false);
            self.b.ins().brif(go, body_block, &[], exit, &[]);
        }
    }

    /// The steady lists that the body of an innermost `for var in …` loop
    /// indexes with `var` itself (`list[var]`): one check before the loop
    /// covers all their bounds checks. Only innermost loops are copied, so
    /// code grows at most twofold.
    fn safe_indexes(&self, var: Local, ty: kavach_kir::Type, body: &Block) -> Vec<Local> {
        if ty != kavach_kir::Type::I64 {
            return Vec::new();
        }
        let has_loop = |b: &Block| {
            fn any_loop(b: &Block) -> bool {
                b.stmts.iter().any(|s| {
                    matches!(
                        s,
                        kavach_kir::Stmt::While { .. }
                            | kavach_kir::Stmt::ForRange { .. }
                            | kavach_kir::Stmt::ForEach { .. }
                    )
                })
            }
            any_loop(b)
                || kavach_kir::walk::any_in_block(b, &mut |e| match &e.kind {
                    ExprKind::If(_, t, o) | ExprKind::Fallback(t, o) => any_loop(t) || any_loop(o),
                    ExprKind::Block(x) => any_loop(x),
                    _ => false,
                })
        };
        if has_loop(body) || kavach_kir::walk::sets_local(body, var) {
            return Vec::new();
        }
        let mut lists = Vec::new();
        kavach_kir::walk::any_in_block(body, &mut |e| {
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
        lists
    }

    /// Reads, before a loop, the buffer and length of each list local the
    /// body indexes and never sets. Nothing else can change such a list
    /// meanwhile: a list is changed in place only by the one variable that
    /// holds it (copy-on-write). Returns the locals, for `let_go`.
    fn hold_steady(&mut self, body: &Block) -> Vec<Local> {
        let mut indexed = Vec::new();
        kavach_kir::walk::any_in_block(body, &mut |e| {
            if let ExprKind::Index(base, _) = &e.kind
                && let ExprKind::Local(l) = base.kind
                && !indexed.contains(&l)
            {
                indexed.push(l);
            }
            false
        });
        indexed.retain(|&l| {
            self.is_list(self.types[l as usize])
                && self.steady[l as usize].is_none()
                && !kavach_kir::walk::sets_local(body, l)
        });
        for &l in &indexed {
            let list = self.b.use_var(self.vars[l as usize]);
            let (ptr, len) = (
                self.b.declare_var(types::I64),
                self.b.declare_var(types::I64),
            );
            // Read only when the local holds a list: before the loop runs,
            // it may still be empty (0), and then the body never reads it.
            let (read, done) = (self.b.create_block(), self.b.create_block());
            self.b.append_block_param(done, types::I64);
            self.b.append_block_param(done, types::I64);
            let zero = self.int(types::I64, 0);
            self.b
                .ins()
                .brif(list, read, &[], done, &[zero.into(), zero.into()]);
            self.b.switch_to_block(read);
            let (p, n) = (self.load(list, PTR), self.load(list, LEN));
            self.b.ins().jump(done, &[p.into(), n.into()]);
            self.b.switch_to_block(done);
            let (p, n) = (self.b.block_params(done)[0], self.b.block_params(done)[1]);
            self.b.def_var(ptr, p);
            self.b.def_var(len, n);
            self.steady[l as usize] = Some((ptr, len));
        }
        indexed
    }

    fn let_go(&mut self, held: Vec<Local>) {
        for l in held {
            self.steady[l as usize] = None;
        }
    }

    /// `for item in list`: `index` counts from 0; `list` holds the list.
    pub(crate) fn for_each(&mut self, var: Local, index: Local, list: Local, body: &Block) {
        // The list stays alive and unchanged during the loop (it is a
        // hidden local), so the loop variable can borrow each item, unless
        // the body takes the item away or replaces it.
        let ty = self.types[var as usize];
        let (moves, uses) = crate::flat::uses_of(body, var);
        self.borrowed[var as usize] = ty.is_ref() && moves == 0;
        // Over a flat list, the variable is a view of each item when the
        // body only reads its fields; otherwise each item is boxed.
        let width = self.flat_width(self.types[list as usize]);
        if width > 0 && uses > 0 {
            self.borrowed[var as usize] = false;
        }
        // Over a list of text, the variable is the address of each item
        // when the body only asks its length; otherwise each is boxed.
        let text = self.text_list(self.types[list as usize]);
        if text {
            let view = moves == 0 && uses == 0;
            self.text_views[var as usize] = view;
            self.borrowed[var as usize] = view;
        }
        let iv = self.vars[index as usize];
        let zero = self.int(types::I64, 0);
        self.b.def_var(iv, zero);
        // The hidden list local never changes in the body: read its buffer
        // and length once.
        let l = self.b.use_var(self.vars[list as usize]);
        let (ptr, len) = (self.load(l, PTR), self.load(l, LEN));
        let (header, body_block, step, exit) = (
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
        );
        self.b.ins().jump(header, &[]);
        self.b.switch_to_block(header);
        let i = self.b.use_var(iv);
        let go = self.b.ins().icmp(IntCC::SignedLessThan, i, len);
        self.b.ins().brif(go, body_block, &[], exit, &[]);
        self.b.switch_to_block(body_block);
        let i = self.b.use_var(iv);
        let item = if text {
            let addr = self.text_addr(ptr, i);
            if self.text_views[var as usize] {
                addr
            } else {
                self.text_box(addr, ty)
            }
        } else if width > 0 {
            let addr = self.flat_addr(ptr, i, width);
            if self.borrowed[var as usize] {
                self.flat_view(addr)
            } else {
                self.flat_box(addr, ty, width)
            }
        } else {
            let item = self.item_at(ptr, i, ty);
            if ty.is_ref() && !self.borrowed[var as usize] {
                self.retain(item);
            }
            item
        };
        self.set_local(var, item);
        self.loops.push((step, exit));
        self.block(body);
        self.loops.pop();
        self.b.ins().jump(step, &[]);
        self.b.switch_to_block(step);
        let i = self.b.use_var(iv);
        let next = self.b.ins().iadd_imm(i, 1);
        self.b.def_var(iv, next);
        let go = self.b.ins().icmp(IntCC::SignedLessThan, next, len);
        self.b.ins().brif(go, body_block, &[], exit, &[]);
        self.b.switch_to_block(exit);
    }
}
