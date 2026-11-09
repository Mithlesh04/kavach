//! Operators and conversions with the run-time checks of spec 5.6, as in
//! the Cranelift backend (`cranelift/src/ops.rs`): integer overflow and
//! division by zero call the same runtime functions, so panics read the
//! same.

use crate::func::FnEmit;
use crate::module::llvm_type;
use kavach_kir::{BinOp, FastOp, Type, UnOp};

impl FnEmit<'_> {
    pub(crate) fn unary(&mut self, op: UnOp, ty: Type, v: &str, line: u32) -> String {
        let t = llvm_type(ty);
        match op {
            UnOp::Not => self.val(format!("xor i1 {v}, true")),
            UnOp::BitNot => self.val(format!("xor {t} {v}, -1")),
            UnOp::Neg if ty.is_float() => self.val(format!("fneg {t} {v}")),
            UnOp::Neg => {
                let kind = if ty.is_signed() { "ssub" } else { "usub" };
                let (result, overflow) = self.with_overflow(kind, ty, "0", v);
                let wide = self.widen(v, ty);
                self.on_overflow(&overflow, "kv_neg_overflow", vec![("i64", wide)], ty, line);
                result
            }
        }
    }

    /// `llvm.<kind>.with.overflow` on `a` and `b`: the result and the flag.
    fn with_overflow(&mut self, kind: &str, ty: Type, a: &str, b: &str) -> (String, String) {
        let t = llvm_type(ty);
        let name = format!("llvm.{kind}.with.overflow.{t}");
        let pair = format!("{{{t}, i1}}");
        self.m.declare(&name, &pair, &[t, t], false);
        let both = self.val(format!("call {pair} @{name}({t} {a}, {t} {b})"));
        let result = self.val(format!("extractvalue {pair} {both}, 0"));
        let flag = self.val(format!("extractvalue {pair} {both}, 1"));
        (result, flag)
    }

    /// Calls the panicking runtime function `name` when `flag` is set,
    /// with `values`, then the type code and line.
    fn on_overflow(
        &mut self,
        flag: &str,
        name: &str,
        mut args: Vec<(&str, String)>,
        ty: Type,
        line: u32,
    ) {
        // The panic functions are `cold`, so LLVM keeps this path aside.
        let (bad, ok) = (self.label(), self.label());
        self.cond_br(flag, &bad, &ok);
        self.start(&bad);
        args.extend([("i32", Self::code(ty)), ("i32", line.to_string())]);
        self.call_rt(name, "void", &args, true);
        self.start(&ok);
    }

    pub(crate) fn binary(&mut self, op: BinOp, ty: Type, a: &str, b: &str, line: u32) -> String {
        use BinOp::*;
        if matches!(op, Eq | Ne | Lt | Le | Gt | Ge) {
            return self.compare(op, ty, a, b, false);
        }
        if ty.is_float() {
            let t = llvm_type(ty);
            let name = match op {
                Add => "fadd",
                Sub => "fsub",
                Mul => "fmul",
                Div => "fdiv",
                // `%` is exact, so a `Float32` one is worked out as a
                // `Float` and rounded back.
                _ if ty == Type::F32 => {
                    let wa = self.val(format!("fpext float {a} to double"));
                    let wb = self.val(format!("fpext float {b} to double"));
                    let r = self.call_rt(
                        "kv_fmod",
                        "double",
                        &[("double", wa), ("double", wb)],
                        false,
                    );
                    return self.val(format!("fptrunc double {r} to float"));
                }
                _ => {
                    let args = [("double", a.to_string()), ("double", b.to_string())];
                    return self.call_rt("kv_fmod", "double", &args, false);
                }
            };
            return self.val(format!("{name} {t} {a}, {b}"));
        }
        let (t, signed) = (llvm_type(ty), ty.is_signed());
        match op {
            Add | Sub | Mul => {
                let kind = match (op, signed) {
                    (Add, true) => "sadd",
                    (Add, false) => "uadd",
                    (Sub, true) => "ssub",
                    (Sub, false) => "usub",
                    (_, true) => "smul",
                    (_, false) => "umul",
                };
                let (result, overflow) = self.with_overflow(kind, ty, a, b);
                self.overflow_check(&overflow, op, a, b, ty, line);
                result
            }
            Div | Rem => self.divide(op, ty, a, b, line),
            Shl | Shr => self.shift(op, ty, a, b, line),
            BitAnd => self.val(format!("and {t} {a}, {b}")),
            BitOr => self.val(format!("or {t} {a}, {b}")),
            _ => self.val(format!("xor {t} {a}, {b}")),
        }
    }

    /// Arithmetic the range pass proved safe: no checks, and LLVM is told
    /// it doesn't overflow.
    pub(crate) fn fast(&mut self, op: FastOp, ty: Type, a: &str, b: &str) -> String {
        let t = llvm_type(ty);
        match op {
            FastOp::Add => self.val(format!("add nsw {t} {a}, {b}")),
            FastOp::Sub => self.val(format!("sub nsw {t} {a}, {b}")),
            FastOp::Mul => self.val(format!("mul nsw {t} {a}, {b}")),
            FastOp::DivPositive => self.val(format!("udiv {t} {a}, {b}")),
            FastOp::RemPositive => self.val(format!("urem {t} {a}, {b}")),
            FastOp::DivSmall | FastOp::RemSmall => {
                let op = if op == FastOp::DivSmall {
                    "udiv"
                } else {
                    "urem"
                };
                if t != "i64" {
                    return self.val(format!("{op} {t} {a}, {b}"));
                }
                let x = self.val(format!("trunc i64 {a} to i32"));
                let y = self.val(format!("trunc i64 {b} to i32"));
                let r = self.val(format!("{op} i32 {x}, {y}"));
                self.val(format!("zext i32 {r} to i64"))
            }
            FastOp::RemOnce => {
                // 0 <= a < 2b, so `a % b` is `a - b` when a >= b.
                let less = self.val(format!("sub {t} {a}, {b}"));
                let over = self.val(format!("icmp sge {t} {a}, {b}"));
                self.val(format!("select i1 {over}, {t} {less}, {t} {a}"))
            }
        }
    }

    /// `a << n` and `a >> n` (spec 1): `n` must be less than the type's
    /// width, and `<<` must keep every bit. The shift itself uses `n`
    /// masked to the width, so it never gives LLVM's poison value.
    fn shift(&mut self, op: BinOp, ty: Type, a: &str, n: &str, line: u32) -> String {
        let (t, bits) = (llvm_type(ty), ty.bits());
        let right = if ty.is_signed() { "ashr" } else { "lshr" };
        let bad = self.val(format!("icmp uge {t} {n}, {bits}"));
        let by = self.val(format!("and {t} {n}, {}", bits - 1));
        let (result, flag) = if op == BinOp::Shl {
            let r = self.val(format!("shl {t} {a}, {by}"));
            let back = self.val(format!("{right} {t} {r}, {by}"));
            let lost = self.val(format!("icmp ne {t} {back}, {a}"));
            (r, self.val(format!("or i1 {bad}, {lost}")))
        } else {
            (self.val(format!("{right} {t} {a}, {by}")), bad)
        };
        self.overflow_check(&flag, op, a, n, ty, line);
        result
    }

    fn overflow_check(&mut self, flag: &str, op: BinOp, a: &str, b: &str, ty: Type, line: u32) {
        let symbol = op.symbol().chars().next().unwrap_or('?') as u32;
        let (wa, wb) = (self.widen(a, ty), self.widen(b, ty));
        let args = vec![("i32", symbol.to_string()), ("i64", wa), ("i64", wb)];
        self.on_overflow(flag, "kv_overflow", args, ty, line);
    }

    fn divide(&mut self, op: BinOp, ty: Type, a: &str, b: &str, line: u32) -> String {
        let (t, signed) = (llvm_type(ty), ty.is_signed());
        let name = match (op, signed) {
            (BinOp::Div, true) => "sdiv",
            (BinOp::Div, false) => "udiv",
            (_, true) => "srem",
            (_, false) => "urem",
        };
        // A constant divisor other than 0 and -1 needs no checks.
        if b.parse::<i64>().is_ok_and(|c| c != 0 && c != -1) {
            return self.val(format!("{name} {t} {a}, {b}"));
        }
        let is_zero = self.val(format!("icmp eq {t} {b}, 0"));
        let (zero, ok) = (self.label(), self.label());
        self.cond_br(&is_zero, &zero, &ok);
        self.start(&zero);
        self.call_rt("kv_div_zero", "void", &[("i32", line.to_string())], true);
        self.start(&ok);
        if !signed {
            return self.val(format!("{name} {t} {a}, {b}"));
        }
        let minus_one = self.val(format!("icmp eq {t} {b}, -1"));
        if op == BinOp::Rem {
            // `MIN % -1` is 0; the machine instruction would trap instead.
            let safe = self.val(format!("select i1 {minus_one}, {t} 1, {t} {b}"));
            return self.val(format!("srem {t} {a}, {safe}"));
        }
        let min = i64::MIN >> (64 - ty.bits());
        let is_min = self.val(format!("icmp eq {t} {a}, {min}"));
        let overflow = self.val(format!("and i1 {is_min}, {minus_one}"));
        self.overflow_check(&overflow, op, a, b, ty, line);
        self.val(format!("sdiv {t} {a}, {b}"))
    }

    /// A comparison giving a Bool. `or_equal` turns `<` into `<=`.
    pub(crate) fn compare(
        &mut self,
        op: BinOp,
        ty: Type,
        a: &str,
        b: &str,
        or_equal: bool,
    ) -> String {
        use BinOp::*;
        let op = match (op, or_equal) {
            (Lt, true) => Le,
            (op, _) => op,
        };
        let t = llvm_type(ty);
        if ty.is_float() {
            let cc = match op {
                Eq => "oeq",
                Ne => "une",
                Lt => "olt",
                Le => "ole",
                Gt => "ogt",
                _ => "oge",
            };
            return self.val(format!("fcmp {cc} {t} {a}, {b}"));
        }
        let s = ty.is_signed();
        let cc = match op {
            Eq => "eq",
            Ne => "ne",
            Lt if s => "slt",
            Le if s => "sle",
            Gt if s => "sgt",
            Ge if s => "sge",
            Lt => "ult",
            Le => "ule",
            Gt => "ugt",
            _ => "uge",
        };
        self.val(format!("icmp {cc} {t} {a}, {b}"))
    }

    /// An integer as 64 bits, keeping its sign or not.
    pub(crate) fn widen(&mut self, v: &str, ty: Type) -> String {
        let t = llvm_type(ty);
        match (ty.bits(), ty.is_signed()) {
            (64, _) => v.to_string(),
            (_, true) => self.val(format!("sext {t} {v} to i64")),
            (_, false) => self.val(format!("zext {t} {v} to i64")),
        }
    }

    /// `Float(n)`, `Int8(x)`…: explicit conversions (spec 2.1).
    pub(crate) fn convert(
        &mut self,
        v: &str,
        from: Type,
        to: Type,
        checked: bool,
        line: u32,
    ) -> String {
        if from == to {
            return v.to_string();
        }
        // A `Float32` into a whole number goes as the `Float` it widens to.
        if from == Type::F32 && to != Type::F64 {
            let wide = self.val(format!("fpext float {v} to double"));
            return self.convert(&wide, Type::F64, to, checked, line);
        }
        let (to_t, line) = (llvm_type(to), line.to_string());
        let narrow = |g: &mut Self, wide: String| match to.bits() {
            64 => wide,
            _ => g.val(format!("trunc i64 {wide} to {to_t}")),
        };
        match (from, to) {
            (Type::F64, Type::F32) => self.val(format!("fptrunc double {v} to float")),
            (Type::F32, Type::F64) => self.val(format!("fpext float {v} to double")),
            (_, Type::F32) if from.is_signed() => {
                self.val(format!("sitofp {} {v} to float", llvm_type(from)))
            }
            (_, Type::F32) => self.val(format!("uitofp {} {v} to float", llvm_type(from))),
            (Type::F64, _) => {
                let args = [
                    ("double", v.to_string()),
                    ("i32", Self::code(to)),
                    ("i32", line),
                ];
                let wide = self.call_rt("kv_float_to_int", "i64", &args, false);
                narrow(self, wide)
            }
            (_, Type::F64) if from.is_signed() => {
                self.val(format!("sitofp {} {v} to double", llvm_type(from)))
            }
            (_, Type::F64) => self.val(format!("uitofp {} {v} to double", llvm_type(from))),
            // Proven to fit (`ranges.rs`): no check, only the width.
            _ if !checked => {
                let wide = self.widen(v, from);
                narrow(self, wide)
            }
            _ => {
                let wide = self.widen(v, from);
                let args = [
                    ("i64", wide),
                    ("i32", Self::code(from)),
                    ("i32", Self::code(to)),
                    ("i32", line),
                ];
                let in_range = self.call_rt("kv_check_int", "i64", &args, false);
                narrow(self, in_range)
            }
        }
    }
}
