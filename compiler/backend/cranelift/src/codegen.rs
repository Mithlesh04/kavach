//! KIR → Cranelift IR: functions, statements and control flow.
//! Operators are in `ops.rs`, heap values in `data.rs`.
//!
//! Ownership of heap values (`Type::Ref`): every expression produces an
//! owned reference. Reading a local takes a new reference; storing into a
//! local releases the value it held; a function releases all its locals
//! when it returns; a value nobody uses is released at once.

use crate::abi::{Runtime, cl_type};
use cranelift_codegen::ir::{Block as CBlock, InstBuilder, TrapCode, Value, types};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::{DataDescription, DataId, FuncId as CFuncId, Module};
use kavach_kir::{Block, Expr, ExprKind, Local, PrintPart, Stmt, Type};
use kavach_runtime_native::rt::type_code;
use std::collections::HashMap;

pub(crate) struct FnGen<'a, 'b> {
    pub(crate) b: FunctionBuilder<'b>,
    pub(crate) module: &'a mut dyn Module,
    /// Text constants already placed in the module's data.
    pub(crate) texts: &'a mut HashMap<String, DataId>,
    /// The runtime's stack limit (`kv_stack_limit`).
    pub(crate) stack_limit: DataId,
    /// The runtime's object pool (`kv_pool`) and live-object counter.
    pub(crate) pool: DataId,
    pub(crate) live: DataId,
    /// In `main` only: the runtime's flag saying whether to free locals
    /// on return (`kv_free_at_exit`).
    pub(crate) free_at_exit: Option<DataId>,
    pub(crate) funcs: &'a [CFuncId],
    /// Whether each program function can fail (returns an error too).
    pub(crate) fails: &'a [bool],
    /// Whether each program function is a closure's, which borrows its
    /// closure (`kavach_kir::borrows`).
    pub(crate) closures: &'a [bool],
    /// Whether the function being compiled can fail, and its return type.
    pub(crate) this_fails: bool,
    pub(crate) ret: Type,
    pub(crate) rt: &'a Runtime,
    /// The program's heap shapes, by type id.
    pub(crate) shapes: &'a [kavach_kir::Shape],
    pub(crate) vars: Vec<Variable>,
    /// The KIR type of each local.
    pub(crate) types: Vec<Type>,
    /// (continue target, exit) of each enclosing loop.
    pub(crate) loops: Vec<(CBlock, CBlock)>,
    /// Locals that borrow their value instead of holding a reference:
    /// `for` variables over a list that stays alive during the loop.
    pub(crate) borrowed: Vec<bool>,
    /// For list locals that the enclosing loop never changes: variables
    /// holding the list's buffer and length, read once before the loop.
    pub(crate) steady: Vec<Option<(Variable, Variable)>>,
    /// (list, index) local pairs whose `list[index]` is known in range: in
    /// the fast copy of a loop that checked its bounds once (`loops.rs`).
    pub(crate) unchecked: Vec<(Local, Local)>,
    /// `for` variables over a list of text that hold the address of their
    /// item instead of an object, because the body only asks their length.
    pub(crate) text_views: Vec<bool>,
}

impl FnGen<'_, '_> {
    /// Calls a runtime or program function and returns its results.
    pub(crate) fn call(&mut self, id: CFuncId, args: &[Value]) -> Vec<Value> {
        let fref = self.module.declare_func_in_func(id, self.b.func);
        let inst = self.b.ins().call(fref, args);
        self.b.inst_results(inst).to_vec()
    }

    pub(crate) fn call_rt(&mut self, name: &str, args: &[Value]) -> Vec<Value> {
        let id = self.rt.id(name);
        self.call(id, args)
    }

    pub(crate) fn int(&mut self, ty: types::Type, n: i64) -> Value {
        self.b.ins().iconst(ty, n)
    }

    pub(crate) fn i32(&mut self, n: i64) -> Value {
        self.b.ins().iconst(types::I32, n)
    }

    /// The runtime code of a KIR type.
    pub(crate) fn code(&mut self, t: Type) -> Value {
        self.int(types::I64, type_code(t))
    }

    /// After a jump, return or trap: continue in a fresh block that nothing
    /// reaches, so later code still has a place to go.
    pub(crate) fn after_terminator(&mut self) {
        let dead = self.b.create_block();
        self.b.switch_to_block(dead);
    }

    /// A pointer and length for constant text, placed in the module's
    /// read-only data (so it works in memory and in executables alike).
    pub(crate) fn constant_text(&mut self, text: &str) -> (Value, Value) {
        let id = match self.texts.get(text) {
            Some(id) => *id,
            None => {
                let id = place_text(self.module, text);
                self.texts.insert(text.to_string(), id);
                id
            }
        };
        let gv = self.module.declare_data_in_func(id, self.b.func);
        let ptr = self.b.ins().symbol_value(types::I64, gv);
        let len = self.int(types::I64, text.len() as i64);
        (ptr, len)
    }

    /// Panics with `message` at `line` (never returns).
    pub(crate) fn panic(&mut self, message: &str, line: u32) {
        let (ptr, len) = self.constant_text(message);
        let line = self.i32(line as i64);
        self.call_rt("kv_panic", &[ptr, len, line]);
        self.b.ins().trap(TrapCode::user(1).expect("a trap code"));
        self.after_terminator();
    }

    /// A placeholder value of type `t` for code after a panic.
    pub(crate) fn zero(&mut self, t: Type) -> Option<Value> {
        let ty = cl_type(t)?;
        Some(if t == Type::F64 {
            self.b.ins().f64const(0.0)
        } else if t == Type::F32 {
            self.b.ins().f32const(0.0)
        } else {
            self.int(ty, 0)
        })
    }

    /// Releases every heap local, then returns `value`, and `error` (0 when
    /// `None`) if the function can fail.
    pub(crate) fn leave_and_return(&mut self, value: Option<Value>, error: Option<Value>) {
        // `main` frees its locals only when leaks are checked: the program
        // ends right after it, and the system takes back all memory.
        let done = self.free_at_exit.map(|flag| {
            let gv = self.module.declare_data_in_func(flag, self.b.func);
            let addr = self.b.ins().symbol_value(types::I64, gv);
            let flags = cranelift_codegen::ir::MemFlags::trusted();
            let free = self.b.ins().load(types::I64, flags, addr, 0);
            let (release, done) = (self.b.create_block(), self.b.create_block());
            self.b.ins().brif(free, release, &[], done, &[]);
            self.b.switch_to_block(release);
            done
        });
        for i in 0..self.vars.len() {
            if self.types[i].is_ref() && !self.borrowed[i] {
                let v = self.b.use_var(self.vars[i]);
                self.release(v);
            }
        }
        if let Some(done) = done {
            self.b.ins().jump(done, &[]);
            self.b.switch_to_block(done);
        }
        let mut results: Vec<Value> = value.into_iter().collect();
        if self.this_fails {
            results.push(error.unwrap_or_else(|| self.int(types::I64, 0)));
        }
        self.b.ins().return_(&results);
        self.after_terminator();
    }

    /// Before a call at `line`: panic there if the stack is nearly used
    /// up (spec 3.8), as in the interpreter. It is a load, a compare and a
    /// branch, with no store, so calls stay as cheap as the hardware
    /// allows.
    pub(crate) fn check_stack(&mut self, line: u32) {
        let gv = self
            .module
            .declare_data_in_func(self.stack_limit, self.b.func);
        let addr = self.b.ins().symbol_value(types::I64, gv);
        let flags = cranelift_codegen::ir::MemFlags::trusted();
        let limit = self.b.ins().load(types::I64, flags, addr, 0);
        let sp = self.b.ins().get_stack_pointer(types::I64);
        let too_deep = self.b.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThan,
            sp,
            limit,
        );
        let (bad, ok) = (self.b.create_block(), self.b.create_block());
        self.b.ins().brif(too_deep, bad, &[], ok, &[]);
        self.b.switch_to_block(bad);
        self.b.set_cold_block(bad);
        let line = self.i32(line as i64);
        self.call_rt("kv_stack_overflow", &[line]);
        self.b.ins().trap(TrapCode::user(1).expect("a trap code"));
        self.b.switch_to_block(ok);
    }

    /// Stores an owned value into a local, releasing what it held.
    pub(crate) fn set_local(&mut self, local: Local, v: Value) {
        let var = self.vars[local as usize];
        if self.types[local as usize].is_ref() && !self.borrowed[local as usize] {
            let old = self.b.use_var(var);
            self.release(old);
        }
        self.b.def_var(var, v);
    }

    pub(crate) fn block(&mut self, block: &Block) -> Option<Value> {
        for s in &block.stmts {
            self.stmt(s);
        }
        block.value.as_ref().and_then(|v| self.expr(v))
    }

    pub(crate) fn stmt(&mut self, s: &Stmt) {
        match s {
            // A capture a closure's function borrows: read in place.
            Stmt::Let(local, e) if self.borrowed[*local as usize] && self.is_place(e) => {
                let v = self.place(e);
                self.b.def_var(self.vars[*local as usize], v);
            }
            Stmt::Let(local, e) => {
                if let Some(v) = self.expr(e) {
                    self.set_local(*local, v);
                }
            }
            Stmt::Expr(e) => self.discard(e),
            Stmt::While { cond, body } => {
                // The condition is tested before the loop and then at the
                // bottom of each round (its code appears twice; each round
                // still runs it once), so a round takes one branch.
                let (body_block, bottom, exit) = (
                    self.b.create_block(),
                    self.b.create_block(),
                    self.b.create_block(),
                );
                let c = self.expr(cond).expect("a condition");
                self.b.ins().brif(c, body_block, &[], exit, &[]);
                self.b.switch_to_block(body_block);
                self.loops.push((bottom, exit));
                self.block(body);
                self.loops.pop();
                self.b.ins().jump(bottom, &[]);
                self.b.switch_to_block(bottom);
                let c = self.expr(cond).expect("a condition");
                self.b.ins().brif(c, body_block, &[], exit, &[]);
                self.b.switch_to_block(exit);
            }
            Stmt::ForRange {
                var,
                start,
                end,
                inclusive,
                body,
            } => self.for_range(*var, start, end, *inclusive, body),
            Stmt::ForEach {
                var,
                index,
                list,
                body,
            } => self.for_each(*var, *index, *list, body),
            Stmt::Return(value) => {
                let v = value.as_ref().and_then(|e| self.expr(e));
                self.leave_and_return(v, None);
            }
            Stmt::LetResult { value, error, call } => self.let_result(*value, *error, call),
            Stmt::Break | Stmt::Continue => {
                let (next, exit) = *self.loops.last().expect("inside a loop");
                let target = if matches!(s, Stmt::Break) { exit } else { next };
                self.b.ins().jump(target, &[]);
                self.after_terminator();
            }
            Stmt::Print(parts) => {
                for part in parts {
                    match part {
                        PrintPart::Text(t) => self.print_text(t),
                        PrintPart::Value(e) => self.print_value(e),
                    }
                }
                self.call_rt("kv_print_end", &[]);
            }
            Stmt::Check {
                cond,
                message,
                line,
            } => {
                let c = self.expr(cond).expect("a condition");
                let (ok, fail) = (self.b.create_block(), self.b.create_block());
                self.b.ins().brif(c, ok, &[], fail, &[]);
                self.b.switch_to_block(fail);
                self.panic(message, *line);
                self.b.ins().jump(ok, &[]);
                self.b.switch_to_block(ok);
            }
        }
    }

    /// Evaluates `e` for its effects only; a heap result is released.
    fn discard(&mut self, e: &Expr) {
        if let Some(v) = self.expr(e)
            && e.ty.is_ref()
        {
            self.release(v);
        }
    }

    fn print_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let (ptr, len) = self.constant_text(text);
        self.call_rt("kv_print_str", &[ptr, len]);
    }

    fn print_value(&mut self, e: &Expr) {
        let Some(v) = self.expr(e) else { return };
        match e.ty {
            Type::F64 => {
                self.call_rt("kv_print_f64", &[v]);
            }
            Type::Bool => {
                self.call_rt("kv_print_bool", &[v]);
            }
            Type::Ref(_) | Type::Char | Type::F32 => {
                let slot = self.pack(v, e.ty);
                let code = self.code(e.ty);
                self.call_rt("kv_print_value", &[slot, code]);
            }
            t => {
                let wide = self.widen(v, t);
                self.call_rt(
                    if t.is_signed() {
                        "kv_print_i64"
                    } else {
                        "kv_print_u64"
                    },
                    &[wide],
                );
            }
        }
    }

    /// An integer value as 64 bits, keeping its sign or not.
    pub(crate) fn widen(&mut self, v: Value, t: Type) -> Value {
        match (t.bits(), t.is_signed()) {
            (64, _) => v,
            (_, true) => self.b.ins().sextend(types::I64, v),
            (_, false) => self.b.ins().uextend(types::I64, v),
        }
    }

    pub(crate) fn expr(&mut self, e: &Expr) -> Option<Value> {
        let ty = cl_type(e.ty);
        match &e.kind {
            // Cranelift checks every index.
            ExprKind::InRange(x, i) => {
                let kind = ExprKind::Index(x.clone(), i.clone());
                self.expr(&Expr { kind, ..*e })
            }
            ExprKind::Int(n) => Some(self.int(ty?, *n as i64)),
            ExprKind::Float(f) if e.ty == Type::F32 => Some(self.b.ins().f32const(*f as f32)),
            ExprKind::Float(f) => Some(self.b.ins().f64const(*f)),
            ExprKind::Bool(v) => Some(self.int(types::I8, *v as i64)),
            ExprKind::Char(c) => Some(self.int(types::I32, *c as i64)),
            ExprKind::Move(l) => Some(self.moved(*l, e.ty.is_ref())),
            ExprKind::Local(l) => {
                let v = self.b.use_var(self.vars[*l as usize]);
                if e.ty.is_ref() {
                    self.retain(v);
                }
                Some(v)
            }
            ExprKind::Assign(l, value) => {
                self.assign(*l, value);
                None
            }
            ExprKind::Unary(_, x) if x.ty.is_ref() => {
                // `-d` on a decimal.
                let v = self.expr(x)?;
                let code = self.code(e.ty);
                Some(self.call_rt("kv_dec_neg", &[v, code])[0])
            }
            ExprKind::Unary(op, x) => {
                let v = self.expr(x)?;
                Some(self.unary(*op, x.ty, v, e.line))
            }
            ExprKind::Binary(op, l, r) => {
                let (a, b) = (self.expr(l)?, self.expr(r)?);
                Some(self.binary(*op, l.ty, a, b, e.line))
            }
            ExprKind::Fast(op, l, r) => {
                let (a, b) = (self.expr(l)?, self.expr(r)?);
                Some(self.fast(*op, a, b))
            }
            ExprKind::And(l, r) | ExprKind::Or(l, r) => {
                Some(self.short_circuit(matches!(e.kind, ExprKind::And(..)), l, r))
            }
            ExprKind::Call(f, args) => {
                let (values, made) = self.call_args(*f, args)?;
                self.check_stack(e.line);
                let id = self.funcs[*f as usize];
                let results = self.call(id, &values);
                if let Some(c) = made {
                    self.release(c);
                }
                ty.and(results.first().copied())
            }
            ExprKind::If(cond, then, otherwise) => {
                let c = self.expr(cond)?;
                let (then_block, else_block, merge) = (
                    self.b.create_block(),
                    self.b.create_block(),
                    self.b.create_block(),
                );
                if let Some(t) = ty {
                    self.b.append_block_param(merge, t);
                }
                self.b.ins().brif(c, then_block, &[], else_block, &[]);
                for (block, body) in [(then_block, then), (else_block, otherwise)] {
                    self.b.switch_to_block(block);
                    let v = self.block(body);
                    match (ty, v) {
                        (Some(_), Some(v)) => self.b.ins().jump(merge, &[v.into()]),
                        (Some(t), None) => {
                            let z = self.zero(e.ty).unwrap_or_else(|| self.int(t, 0));
                            self.b.ins().jump(merge, &[z.into()])
                        }
                        (None, _) => self.b.ins().jump(merge, &[]),
                    };
                }
                self.b.switch_to_block(merge);
                ty.map(|_| self.b.block_params(merge)[0])
            }
            // The code as written; only the LLVM backend runs the fast
            // version (`kavach_kir::fuse`).
            ExprKind::Block(block) | ExprKind::Fallback(_, block) => {
                let v = self.block(block);
                if ty.is_some() {
                    v.or_else(|| self.zero(e.ty))
                } else {
                    None
                }
            }
            ExprKind::Fail(x) => {
                self.fail(x);
                None
            }
            // Decision 0031: Cranelift always takes the plain loop.
            ExprKind::Rt(kavach_kir::RtOp::ParFor | kavach_kir::RtOp::ParForRed, _) => {
                Some(self.b.ins().iconst(types::I64, 1))
            }
            ExprKind::Convert(x, checked) => {
                let v = self.expr(x)?;
                Some(self.convert(v, x.ty, e.ty, *checked, e.line))
            }
            ExprKind::Panic(message) => {
                self.panic(message, e.line);
                self.zero(e.ty)
            }
            _ => self.data_expr(e),
        }
    }

    fn short_circuit(&mut self, is_and: bool, l: &Expr, r: &Expr) -> Value {
        let a = self.expr(l).expect("a Bool");
        let (rhs, merge) = (self.b.create_block(), self.b.create_block());
        self.b.append_block_param(merge, types::I8);
        // `and` stops at false, `or` at true: either way the result is `a`.
        if is_and {
            self.b.ins().brif(a, rhs, &[], merge, &[a.into()]);
        } else {
            self.b.ins().brif(a, merge, &[a.into()], rhs, &[]);
        }
        self.b.switch_to_block(rhs);
        let b = self.expr(r).expect("a Bool");
        self.b.ins().jump(merge, &[b.into()]);
        self.b.switch_to_block(merge);
        self.b.block_params(merge)[0]
    }
}

/// Puts `text` in the module's read-only data. A zero byte follows it, so
/// even empty text has an address.
pub(crate) fn place_text(module: &mut dyn Module, text: &str) -> DataId {
    let id = module
        .declare_anonymous_data(false, false)
        .expect("declare text data");
    let mut data = DataDescription::new();
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    data.define(bytes.into_boxed_slice());
    module.define_data(id, &data).expect("define text data");
    id
}
