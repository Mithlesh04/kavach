//! One function's LLVM IR. Every local lives in a stack slot (`alloca`):
//! reads are loads and writes are stores, and LLVM's `mem2reg` turns them
//! into registers. So the code here never builds SSA form itself.

use crate::module::{Module, llvm_type, ret_type};
use kavach_kir::{Function, Local, Shape, Type};
use kavach_runtime_native::rt::type_code;
use std::fmt::Write;

pub(crate) struct FnEmit<'a> {
    pub(crate) m: &'a mut Module,
    pub(crate) f: &'a Function,
    /// The program's heap types, and whether this is `main`.
    pub(crate) shapes: &'a [Shape],
    is_main: bool,
    /// The instructions after the entry block's stack slots.
    out: String,
    /// The stack slots, all in the entry block, as LLVM wants them.
    slots: String,
    next: usize,
    /// Whether the current block still needs its terminator.
    open: bool,
    /// (continue label, exit label) of each enclosing loop.
    pub(crate) loops: Vec<(String, String)>,
    /// A quiet loop chunk (decision 0031): a failing check returns 1
    /// instead of panicking, and the caller re-runs the loop serially.
    quiet: bool,
    /// (list, index) local pairs whose `list[index]` is known in range: in
    /// the copy of a loop that checked its bounds once (`stmt.rs`).
    pub(crate) unchecked: Vec<(Local, Local)>,
    /// Lists written as `list[var] = v` in a loop proven safe: stores go
    /// straight to the buffer read before the loop (`stmt.rs`).
    pub(crate) raw_sets: Vec<(Local, Local, String)>,
    /// Lists a counted loop pushes to, with room reserved up front
    /// (`stmt.rs`): the loop variable, the buffer, and the offset that
    /// turns the variable into the item's index.
    pub(crate) raw_pushes: Vec<(Local, Local, String, String)>,
    /// For list locals that the enclosing loop never changes: stack slots
    /// holding the list's buffer and length, read once before the loop.
    pub(crate) steady: Vec<Option<(String, String)>>,
    /// Locals that borrow their value instead of holding a reference:
    /// `for` variables over a list that stays alive during the loop.
    pub(crate) borrowed: Vec<bool>,
    /// `for` variables over a list of text that hold the address of their
    /// item instead of an object, because the body only asks its length.
    pub(crate) text_views: Vec<bool>,
    /// For each enclosing `Fallback`'s fast block: the label of its slow
    /// block, where a failed check goes instead of panicking.
    pub(crate) fallbacks: Vec<String>,
}

/// The name of program function `i`.
pub(crate) fn fn_name(i: usize) -> String {
    format!("kv_fn_{i}")
}

impl<'a> FnEmit<'a> {
    pub(crate) fn new(
        m: &'a mut Module,
        f: &'a Function,
        shapes: &'a [Shape],
        is_main: bool,
        quiet: bool,
    ) -> Self {
        FnEmit {
            m,
            f,
            shapes,
            is_main,
            quiet,
            out: String::new(),
            slots: String::new(),
            next: 0,
            open: true,
            loops: Vec::new(),
            unchecked: Vec::new(),
            raw_sets: Vec::new(),
            raw_pushes: Vec::new(),
            steady: vec![None; f.locals.len()],
            borrowed: vec![false; f.locals.len()],
            text_views: vec![false; f.locals.len()],
            fallbacks: Vec::new(),
        }
    }

    /// For a closure's function: its closure, and the captures it reads
    /// from it and never changes, borrow (`kavach_kir::borrows`).
    pub(crate) fn borrowing(mut self, closure: bool) -> Self {
        if closure {
            for l in kavach_kir::borrows::borrowing_locals(self.f) {
                self.borrowed[l as usize] = true;
            }
        }
        self
    }

    /// The whole function, `define … { … }`, named `name`.
    pub(crate) fn emit(mut self, name: &str) -> String {
        let f = self.f;
        for (l, t) in f.locals.iter().enumerate() {
            if *t != Type::Unit {
                let _ = writeln!(self.slots, "  %l{l} = alloca {}", llvm_type(*t));
            }
            // Heap locals start as null, so storing into them can release
            // the old value.
            if t.is_ref() && !f.params.contains(&(l as Local)) {
                let _ = writeln!(self.slots, "  store i64 0, ptr %l{l}");
            }
        }
        let mut params = Vec::new();
        for p in &f.params {
            let t = f.locals[*p as usize];
            if t != Type::Unit {
                params.push(format!("{} %p{p}", llvm_type(t)));
                self.ins(format!("store {} %p{p}, ptr %l{p}", llvm_type(t)));
            }
        }
        let value = self.block(&f.body);
        self.ret(value);
        if self.open {
            self.ins("unreachable".into());
        }
        // Unwind tables, so a panic can unwind through generated frames to
        // the start of its task (decision 0065).
        format!(
            "define internal {} @{name}({}) uwtable {{\nentry:\n{}{}}}\n\n",
            ret_type(f.ret, f.fails),
            params.join(", "),
            self.slots,
            self.out
        )
    }

    /// A new value name.
    pub(crate) fn fresh(&mut self) -> String {
        self.next += 1;
        format!("%v{}", self.next)
    }

    /// A new block label.
    pub(crate) fn label(&mut self) -> String {
        self.next += 1;
        format!("L{}", self.next)
    }

    /// One instruction.
    pub(crate) fn ins(&mut self, text: String) {
        let ends = text.starts_with("br ") || text.starts_with("ret") || text == "unreachable";
        if ends {
            self.open = false;
        }
        let _ = writeln!(self.out, "  {text}");
    }

    /// An instruction that gives a value; returns its name.
    pub(crate) fn val(&mut self, text: String) -> String {
        let v = self.fresh();
        self.ins(format!("{v} = {text}"));
        v
    }

    /// Starts block `label`. A block left open (code after a `return`,
    /// which nothing reaches) is closed by a jump to it.
    pub(crate) fn start(&mut self, label: &str) {
        if self.open {
            self.br(label);
        }
        let _ = writeln!(self.out, "{label}:");
        self.open = true;
    }

    pub(crate) fn br(&mut self, label: &str) {
        self.ins(format!("br label %{label}"));
    }

    pub(crate) fn cond_br(&mut self, cond: &str, yes: &str, no: &str) {
        self.ins(format!("br i1 {cond}, label %{yes}, label %{no}"));
    }

    /// After a branch, return or `unreachable`: later code goes in a block
    /// that nothing reaches.
    pub(crate) fn dead(&mut self) {
        let label = self.label();
        self.start(&label);
    }

    /// A stack slot of type `t` for a temporary (an `if`'s value).
    pub(crate) fn slot(&mut self, t: Type) -> String {
        self.slot_of_type(llvm_type(t))
    }

    /// The capture slots of a chunked loop (decision 0031), as an
    /// `i64` array on the stack.
    pub(crate) fn slot_array(&mut self, args: &[kavach_kir::Expr]) -> String {
        let n = args.len().max(1);
        let arr = self.slot_of_type(&format!("[{n} x i64]"));
        for (k, a) in args.iter().enumerate() {
            // Captures are borrowed: a retain here would hide the list's
            // uniqueness from the chunks' raw-store proof.
            let v = match &a.kind {
                kavach_kir::ExprKind::Local(l) => self.load_local(*l).expect("an i64 slot"),
                _ => self.expr(a).expect("an i64 slot"),
            };
            let at = self.val(format!("getelementptr i64, ptr {arr}, i64 {k}"));
            self.ins(format!("store i64 {v}, ptr {at}"));
        }
        arr
    }

    /// A stack slot of LLVM type `ty`.
    pub(crate) fn slot_of_type(&mut self, ty: &str) -> String {
        let s = self.fresh();
        let _ = writeln!(self.slots, "  {s} = alloca {ty}");
        s
    }

    pub(crate) fn load_local(&mut self, l: Local) -> Option<String> {
        let t = self.f.locals[l as usize];
        (t != Type::Unit).then(|| self.val(format!("load {}, ptr %l{l}", llvm_type(t))))
    }

    pub(crate) fn store_local(&mut self, l: Local, v: &str) {
        let t = self.f.locals[l as usize];
        self.ins(format!("store {} {v}, ptr %l{l}", llvm_type(t)));
    }

    /// `local = v` (owned): a heap local releases its old value first.
    pub(crate) fn set_local(&mut self, l: Local, v: &str) {
        if self.f.locals[l as usize].is_ref() && !self.borrowed[l as usize] {
            let old = self.load_local(l).expect("a heap value");
            self.release(&old);
        }
        self.store_local(l, v);
    }

    /// Calls runtime function `name` with typed `args`; `ret` is its result
    /// type (`void` for none). `never_returns` marks the panicking ones,
    /// after which the block ends; inside a `Fallback`'s fast block they
    /// go to its slow block instead.
    pub(crate) fn call_rt(
        &mut self,
        name: &str,
        ret: &str,
        args: &[(&str, String)],
        never_returns: bool,
    ) -> String {
        if let (true, Some(slow)) = (never_returns, self.fallbacks.last()) {
            let slow = slow.clone();
            self.br(&slow);
            self.dead();
            return String::new();
        }
        // A quiet chunk (decision 0031) calls nothing: a check that
        // fails returns 1 instead of panicking, and any slow path that
        // would enter the runtime (copy-on-write, growth) returns 1 the
        // same way — the runtime is not safe under the chunks' threads,
        // and the caller's plain loop handles the case serially.
        if self.quiet {
            self.ins("ret i64 1".into());
            self.dead();
            // The dead block still spells out whole instructions.
            return match ret {
                "void" => String::new(),
                "double" => "0.0".into(),
                "ptr" => "null".into(),
                _ => "0".into(),
            };
        }
        let types: Vec<&str> = args.iter().map(|(t, _)| *t).collect();
        self.m.declare(name, ret, &types, never_returns);
        let list: Vec<String> = args.iter().map(|(t, v)| format!("{t} {v}")).collect();
        let call = format!("call {ret} @{name}({})", list.join(", "));
        if never_returns {
            self.ins(call);
            self.ins("unreachable".into());
            self.dead();
            return String::new();
        }
        if ret == "void" {
            self.ins(call);
            String::new()
        } else {
            self.val(call)
        }
    }

    /// The runtime's code for type `t` (`rt::type_code`).
    pub(crate) fn code(t: Type) -> String {
        type_code(t).to_string()
    }

    /// A placeholder of type `t` for code after a panic.
    pub(crate) fn zero(t: Type) -> Option<String> {
        match t {
            Type::Unit => None,
            Type::F64 | Type::F32 => Some("0.0".into()),
            _ => Some("0".into()),
        }
    }

    /// Panics with `message` at `line` (spec 5.8).
    pub(crate) fn panic(&mut self, message: &str, line: u32) {
        let (ptr, len) = self.m.text(message);
        let args = [
            ("i64", ptr),
            ("i64", len.to_string()),
            ("i32", line.to_string()),
        ];
        self.call_rt("kv_panic", "void", &args, true);
    }

    /// Releases the heap locals, then returns `value` (or the zero of the
    /// return type after a panic). `main` frees its locals only when leaks
    /// are checked: the program ends right after it, and the system takes
    /// back all memory (as in the Cranelift backend).
    pub(crate) fn ret(&mut self, value: Option<String>) {
        self.leave(value, None);
    }

    /// Returns `value` and, from a function that can fail, `error` (an
    /// owned error, or 0 for none).
    pub(crate) fn leave(&mut self, value: Option<String>, error: Option<String>) {
        if self.is_main {
            let (free, done) = (self.label(), self.label());
            let flag = self.val("load i64, ptr @kv_free_at_exit".into());
            let set = self.val(format!("icmp ne i64 {flag}, 0"));
            self.cond_br(&set, &free, &done);
            self.start(&free);
            self.release_locals();
            self.br(&done);
            self.start(&done);
        } else if !self.quiet {
            // A quiet chunk (decision 0031) only borrows its captures:
            // releasing them here would race with the other chunks.
            self.release_locals();
        }
        let t = self.f.ret;
        let error = error.unwrap_or_else(|| "0".into());
        match (value.or_else(|| Self::zero(t)), self.f.fails) {
            (_, true) if t == Type::Unit => self.ins(format!("ret i64 {error}")),
            (Some(v), true) => {
                let ty = ret_type(t, true);
                let one = self.val(format!("insertvalue {ty} poison, {} {v}, 0", llvm_type(t)));
                let both = self.val(format!("insertvalue {ty} {one}, i64 {error}, 1"));
                self.ins(format!("ret {ty} {both}"));
            }
            (Some(v), false) if t != Type::Unit => self.ins(format!("ret {} {v}", llvm_type(t))),
            _ => self.ins("ret void".into()),
        }
        self.dead();
    }

    /// Before a call at `line`: a panic there when the stack is nearly
    /// used up (spec 3.8), as in the interpreter. Checked at each call
    /// rather than on entry, a path that makes no call (a recursion's
    /// leaf) needs no stack frame.
    pub(crate) fn stack_check(&mut self, line: u32) {
        self.m
            .declare("llvm.read_register.i64", "i64", &["metadata"], false);
        let sp = self.val("call i64 @llvm.read_register.i64(metadata !0)".into());
        let limit = self.val("load i64, ptr @kv_tls_limit".into());
        let low = self.val(format!("icmp ult i64 {sp}, {limit}"));
        let (bad, ok) = (self.label(), self.label());
        self.cond_br(&low, &bad, &ok);
        self.start(&bad);
        self.call_rt(
            "kv_stack_overflow",
            "void",
            &[("i32", line.to_string())],
            true,
        );
        self.br(&ok);
        self.start(&ok);
    }
}
