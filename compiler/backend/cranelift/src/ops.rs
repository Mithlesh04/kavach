//! Operators and conversions, with the run-time checks of spec 5.6:
//! integer overflow and division by zero panic.

use crate::abi::cl_type;
use crate::codegen::FnGen;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{InstBuilder, InstructionData, Opcode, Value, types};
use kavach_kir::{BinOp, FastOp, Type, UnOp};
use kavach_runtime_native::rt::type_code;

impl FnGen<'_, '_> {
    pub(crate) fn unary(&mut self, op: UnOp, ty: Type, v: Value, line: u32) -> Value {
        match op {
            UnOp::Not => self.b.ins().bxor_imm(v, 1),
            UnOp::BitNot => self.b.ins().bnot(v),
            UnOp::Neg if ty.is_float() => self.b.ins().fneg(v),
            UnOp::Neg => {
                let cty = cl_type(ty).expect("an integer");
                let zero = self.int(cty, 0);
                let (result, overflow) = if ty.is_signed() {
                    self.b.ins().ssub_overflow(zero, v)
                } else {
                    self.b.ins().usub_overflow(zero, v)
                };
                let wide = self.widen(v, ty);
                self.on_overflow(overflow, "kv_neg_overflow", &[wide], ty, line);
                result
            }
        }
    }

    /// Calls the runtime function `name` (which panics) when `flag` is set.
    fn on_overflow(&mut self, flag: Value, name: &str, values: &[Value], ty: Type, line: u32) {
        let (bad, ok) = (self.b.create_block(), self.b.create_block());
        self.b.ins().brif(flag, bad, &[], ok, &[]);
        self.b.switch_to_block(bad);
        let mut args = values.to_vec();
        let (code, line) = (self.i32(type_code(ty)), self.i32(line as i64));
        args.extend([code, line]);
        self.call_rt(name, &args);
        self.b
            .ins()
            .trap(cranelift_codegen::ir::TrapCode::user(1).expect("a trap code"));
        self.b.switch_to_block(ok);
    }

    pub(crate) fn binary(&mut self, op: BinOp, ty: Type, a: Value, b: Value, line: u32) -> Value {
        use BinOp::*;
        if ty.is_ref() {
            return self.ref_binary(op, ty, a, b, line);
        }
        if matches!(op, Eq | Ne | Lt | Le | Gt | Ge) {
            return self.compare(op, ty, a, b, false);
        }
        if ty.is_float() {
            return match op {
                Add => self.b.ins().fadd(a, b),
                Sub => self.b.ins().fsub(a, b),
                Mul => self.b.ins().fmul(a, b),
                Div => self.b.ins().fdiv(a, b),
                _ if ty == Type::F32 => {
                    // `%` is exact, so working it out as a `Float` and
                    // rounding back gives the `Float32` answer.
                    let (wa, wb) = (
                        self.b.ins().fpromote(types::F64, a),
                        self.b.ins().fpromote(types::F64, b),
                    );
                    let r = self.call_rt("kv_fmod", &[wa, wb])[0];
                    self.b.ins().fdemote(types::F32, r)
                }
                _ => self.call_rt("kv_fmod", &[a, b])[0],
            };
        }
        let signed = ty.is_signed();
        match op {
            Add | Sub | Mul => {
                let ins = self.b.ins();
                let (result, overflow) = match (op, signed) {
                    (Add, true) => ins.sadd_overflow(a, b),
                    (Add, false) => ins.uadd_overflow(a, b),
                    (Sub, true) => ins.ssub_overflow(a, b),
                    (Sub, false) => ins.usub_overflow(a, b),
                    (_, true) => ins.smul_overflow(a, b),
                    (_, false) => ins.umul_overflow(a, b),
                };
                self.overflow_check(overflow, op, a, b, ty, line);
                result
            }
            Div | Rem => self.divide(op, ty, a, b, line),
            Shl | Shr => self.shift(op, ty, a, b, line),
            BitAnd => self.b.ins().band(a, b),
            BitOr => self.b.ins().bor(a, b),
            _ => self.b.ins().bxor(a, b),
        }
    }

    /// Arithmetic the range pass proved safe: no checks.
    pub(crate) fn fast(&mut self, op: FastOp, a: Value, b: Value) -> Value {
        let ins = self.b.ins();
        match op {
            FastOp::Add => ins.iadd(a, b),
            FastOp::Sub => ins.isub(a, b),
            FastOp::Mul => ins.imul(a, b),
            FastOp::DivPositive => ins.udiv(a, b),
            FastOp::RemPositive => ins.urem(a, b),
            FastOp::DivSmall | FastOp::RemSmall => {
                let ty = self.b.func.dfg.value_type(a);
                if ty.bits() <= 32 {
                    return match op {
                        FastOp::DivSmall => self.b.ins().udiv(a, b),
                        _ => self.b.ins().urem(a, b),
                    };
                }
                let (x, y) = (
                    self.b.ins().ireduce(types::I32, a),
                    self.b.ins().ireduce(types::I32, b),
                );
                let r = match op {
                    FastOp::DivSmall => self.b.ins().udiv(x, y),
                    _ => self.b.ins().urem(x, y),
                };
                self.b.ins().uextend(ty, r)
            }
            FastOp::RemOnce => {
                // 0 <= a < 2b, so `a % b` is `a - b` when a >= b.
                let less = self.b.ins().isub(a, b);
                let over = self.b.ins().icmp(IntCC::SignedGreaterThanOrEqual, a, b);
                self.b.ins().select(over, less, a)
            }
        }
    }

    /// `a << n` and `a >> n` (spec 1): `n` must be less than the type's
    /// width, and `<<` must keep every bit.
    fn shift(&mut self, op: BinOp, ty: Type, a: Value, n: Value, line: u32) -> Value {
        let signed = ty.is_signed();
        let bad = self
            .b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, n, ty.bits() as i64);
        let right = |s: &mut Self, v, n| {
            if signed {
                s.b.ins().sshr(v, n)
            } else {
                s.b.ins().ushr(v, n)
            }
        };
        let (result, flag) = if op == BinOp::Shl {
            let r = self.b.ins().ishl(a, n);
            let back = right(self, r, n);
            let lost = self.b.ins().icmp(IntCC::NotEqual, back, a);
            (r, self.b.ins().bor(bad, lost))
        } else {
            (right(self, a, n), bad)
        };
        self.overflow_check(flag, op, a, n, ty, line);
        result
    }

    fn overflow_check(&mut self, flag: Value, op: BinOp, a: Value, b: Value, ty: Type, line: u32) {
        let symbol = op.symbol().chars().next().unwrap_or('?') as i64;
        let op_code = self.i32(symbol);
        let (wa, wb) = (self.widen(a, ty), self.widen(b, ty));
        self.on_overflow(flag, "kv_overflow", &[op_code, wa, wb], ty, line);
    }

    /// The value of `v` when it is a constant.
    fn constant(&self, v: Value) -> Option<i64> {
        let inst = self.b.func.dfg.value_def(v).inst()?;
        match self.b.func.dfg.insts[inst] {
            InstructionData::UnaryImm {
                opcode: Opcode::Iconst,
                imm,
            } => Some(imm.bits()),
            _ => None,
        }
    }

    fn divide(&mut self, op: BinOp, ty: Type, a: Value, b: Value, line: u32) -> Value {
        // A constant divisor other than 0 and -1 needs no checks, and
        // Cranelift turns the division into a multiplication. A narrow
        // constant is kept in its own bits (an `Int8` -1 is 255), so it is
        // read back with its sign first.
        let width = 64 - ty.bits();
        let signed = |c: i64| {
            if ty.is_signed() {
                (c << width) >> width
            } else {
                c
            }
        };
        if self
            .constant(b)
            .is_some_and(|c| signed(c) != 0 && signed(c) != -1)
        {
            let ins = self.b.ins();
            return match (op, ty.is_signed()) {
                (BinOp::Div, true) => ins.sdiv(a, b),
                (BinOp::Div, false) => ins.udiv(a, b),
                (_, true) => ins.srem(a, b),
                (_, false) => ins.urem(a, b),
            };
        }
        let cty = cl_type(ty).expect("an integer");
        let (zero_block, ok) = (self.b.create_block(), self.b.create_block());
        let is_zero = self.b.ins().icmp_imm(IntCC::Equal, b, 0);
        self.b.ins().brif(is_zero, zero_block, &[], ok, &[]);
        self.b.switch_to_block(zero_block);
        let line_value = self.i32(line as i64);
        self.call_rt("kv_div_zero", &[line_value]);
        self.b
            .ins()
            .trap(cranelift_codegen::ir::TrapCode::user(1).expect("a trap code"));
        self.b.switch_to_block(ok);
        if !ty.is_signed() {
            return if op == BinOp::Div {
                self.b.ins().udiv(a, b)
            } else {
                self.b.ins().urem(a, b)
            };
        }
        let minus_one = self.b.ins().icmp_imm(IntCC::Equal, b, -1);
        if op == BinOp::Rem {
            // `MIN % -1` is 0; the machine instruction would trap instead.
            let one = self.int(cty, 1);
            let safe = self.b.ins().select(minus_one, one, b);
            return self.b.ins().srem(a, safe);
        }
        let min = self.int(cty, i64::MIN >> (64 - ty.bits()));
        let is_min = self.b.ins().icmp(IntCC::Equal, a, min);
        let overflow = self.b.ins().band(is_min, minus_one);
        self.overflow_check(overflow, op, a, b, ty, line);
        self.b.ins().sdiv(a, b)
    }

    /// A comparison giving a Bool. `or_equal` turns `<` into `<=` (for ranges).
    pub(crate) fn compare(
        &mut self,
        op: BinOp,
        ty: Type,
        a: Value,
        b: Value,
        or_equal: bool,
    ) -> Value {
        use BinOp::*;
        let op = match (op, or_equal) {
            (Lt, true) => Le,
            (op, _) => op,
        };
        if ty.is_float() {
            let cc = match op {
                Eq => FloatCC::Equal,
                Ne => FloatCC::NotEqual,
                Lt => FloatCC::LessThan,
                Le => FloatCC::LessThanOrEqual,
                Gt => FloatCC::GreaterThan,
                _ => FloatCC::GreaterThanOrEqual,
            };
            return self.b.ins().fcmp(cc, a, b);
        }
        let s = ty.is_signed();
        let cc = match op {
            Eq => IntCC::Equal,
            Ne => IntCC::NotEqual,
            Lt if s => IntCC::SignedLessThan,
            Le if s => IntCC::SignedLessThanOrEqual,
            Gt if s => IntCC::SignedGreaterThan,
            Ge if s => IntCC::SignedGreaterThanOrEqual,
            Lt => IntCC::UnsignedLessThan,
            Le => IntCC::UnsignedLessThanOrEqual,
            Gt => IntCC::UnsignedGreaterThan,
            _ => IntCC::UnsignedGreaterThanOrEqual,
        };
        self.b.ins().icmp(cc, a, b)
    }

    /// `Float(n)`, `Int8(x)`…: explicit conversions (spec 2.1).
    pub(crate) fn convert(
        &mut self,
        v: Value,
        from: Type,
        to: Type,
        checked: bool,
        line: u32,
    ) -> Value {
        if from == to {
            return v;
        }
        let line = self.i32(line as i64);
        // To and from `Decimal`, the only heap type that converts.
        if to.is_ref() {
            let slot = self.pack(v, from);
            let (from, to) = (self.code(from), self.code(to));
            return self.call_rt("kv_dec_from", &[slot, from, to, line])[0];
        }
        if from.is_ref() {
            let to_code = self.code(to);
            let slot = self.call_rt("kv_dec_to", &[v, to_code, line])[0];
            return self.unpack(slot, to);
        }
        let to_code = self.i32(type_code(to));
        let narrow = |g: &mut Self, wide: Value| match to.bits() {
            64 => wide,
            bits => {
                g.b.ins()
                    .ireduce(types::Type::int(bits as u16).expect("an int type"), wide)
            }
        };
        match (from, to) {
            (Type::F64, Type::F32) => self.b.ins().fdemote(types::F32, v),
            (Type::F32, Type::F64) => self.b.ins().fpromote(types::F64, v),
            (Type::F32, _) => {
                let wide = self.b.ins().fpromote(types::F64, v);
                let whole = self.call_rt("kv_float_to_int", &[wide, to_code, line])[0];
                narrow(self, whole)
            }
            (_, Type::F32) if from.is_signed() => self.b.ins().fcvt_from_sint(types::F32, v),
            (_, Type::F32) => self.b.ins().fcvt_from_uint(types::F32, v),
            (Type::F64, _) => {
                let wide = self.call_rt("kv_float_to_int", &[v, to_code, line])[0];
                narrow(self, wide)
            }
            (_, Type::F64) if from.is_signed() => self.b.ins().fcvt_from_sint(types::F64, v),
            (_, Type::F64) => self.b.ins().fcvt_from_uint(types::F64, v),
            // Proven to fit (`ranges.rs`): no check, only the width.
            _ if !checked => {
                let wide = self.widen(v, from);
                narrow(self, wide)
            }
            _ => {
                let wide = self.widen(v, from);
                let from_code = self.i32(type_code(from));
                let in_range = self.call_rt("kv_check_int", &[wide, from_code, to_code, line])[0];
                narrow(self, in_range)
            }
        }
    }
}
