// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! KIR function bodies as WGSL functions. Every value an expression
//! needs statements for (an `if` with a value, a block) goes through a
//! temporary declared at the top of the function, so expressions stay
//! WGSL expressions. `None` means KIR used something a kernel can't do.

use crate::ops::{binary, can_stop};
use crate::values::{field, zero};
use crate::{ty_name, wgsl_type};
use kavach_kir::{
    BinOp, Block, Expr, ExprKind as K, FastOp, FuncId, Function, Local, Program, RtOp, Stmt, Type,
    UnOp,
};
use std::collections::{BTreeSet, HashMap};

pub(crate) struct Writer<'p> {
    program: &'p Program,
    /// The finished WGSL functions.
    pub(crate) out: String,
    /// The closure's captured values: where each is in `info`, and its
    /// type.
    pub(crate) captures: HashMap<u32, Type>,
    /// Added to a capture's field index for its place in `info`: in a
    /// chain of `map`s (`lib.rs`), each closure's captures follow those of
    /// the closures before it.
    pub(crate) capture_base: u32,
    /// The messages of `Panic`s, coded from 100 (see `lib.rs`).
    pub(crate) panics: Vec<String>,
    /// The entry's item when it is a tuple or record: its local and field
    /// types. Each field comes in as its own parameter, `p<local>_<k>`.
    pub(crate) item: Option<(Local, Vec<Type>)>,
    /// The record and tuple shapes the functions use, each a WGSL struct
    /// `S<shape>` with fields `f0`, `f1`, … (`lib.rs` declares them).
    pub(crate) structs: BTreeSet<u32>,
    /// A loop was written that may not end: a `while`, or an inclusive
    /// range, which wraps at the largest number (decision 0077).
    pub(crate) open_loop: bool,
    /// The shape of `Texture`, which a fragment fn takes (spec 15.3), and
    /// that of the colors `sample` gives, once one is written.
    pub(crate) texture: Option<u32>,
    pub(crate) sampled: Option<u32>,
    names: HashMap<FuncId, String>,
}

/// One function being written.
pub(crate) struct Body<'p> {
    program: &'p Program,
    pub(crate) f: &'p Function,
    env: Option<Local>,
    /// The item, when it is a tuple or record (`Writer::item`).
    item: Option<Local>,
    code: String,
    temps: Vec<Type>,
    depth: usize,
    pub(crate) texture: Option<u32>,
}

impl<'p> Writer<'p> {
    pub(crate) fn new(program: &'p Program) -> Self {
        Writer {
            program,
            out: String::new(),
            captures: HashMap::new(),
            capture_base: 0,
            panics: Vec::new(),
            item: None,
            structs: BTreeSet::new(),
            open_loop: false,
            texture: None,
            sampled: None,
            names: HashMap::new(),
        }
    }

    /// Writes function `fid` and those it calls; gives its WGSL name. With
    /// `env`, that parameter is the closure, whose fields are captures.
    pub(crate) fn function(&mut self, fid: FuncId, env: Option<Local>) -> Option<String> {
        if let Some(name) = self.names.get(&fid) {
            return Some(name.clone());
        }
        let name = format!("f{fid}");
        self.names.insert(fid, name.clone());
        let f = self.program.functions.get(fid as usize)?;
        let ret = self.name_of(f.ret)?;
        // Only the entry (the closure's function) takes the item.
        let item = self.item.clone().filter(|_| env.is_some());
        let mut b = Body {
            program: self.program,
            f,
            env,
            item: item.as_ref().map(|(l, _)| *l),
            code: String::new(),
            temps: Vec::new(),
            depth: 1,
            texture: self.texture,
        };
        let params: Vec<Local> = f
            .params
            .iter()
            .copied()
            .filter(|p| Some(*p) != env)
            .collect();
        let whole = b.item;
        let plain: Vec<Local> = (params.iter().copied())
            .filter(|p| Some(*p) != whole && !b.is_texture(*p))
            .collect();
        for p in plain {
            b.line(&format!("l{p} = p{p};"));
        }
        // The item as one value too, for a use of all of it.
        if let (Some(l), Some((_, fields))) = (whole, &item)
            && let Some(made) = ty_name(self.program, f.locals[l as usize])
        {
            let parts: Vec<String> = (0..fields.len()).map(|k| format!("p{l}_{k}")).collect();
            b.line(&format!("l{l} = {made}({});", parts.join(", ")));
        }
        for s in &f.body.stmts {
            b.stmt(self, s)?;
        }
        let value = b.expr(self, f.body.value.as_ref()?)?;
        b.line(&format!("return {value};"));
        let mut text = format!("fn {name}(");
        let mut typed = Vec::new();
        for p in &params {
            match &item {
                Some((l, fields)) if l == p => {
                    for (k, t) in fields.iter().enumerate() {
                        typed.push(format!("p{p}_{k}: {}", wgsl_type(*t)?));
                    }
                }
                _ if b.is_texture(*p) => typed.push(format!("p{p}: texture_2d<f32>")),
                _ => typed.push(format!("p{p}: {}", self.name_of(f.locals[*p as usize])?)),
            }
        }
        text.push_str(&typed.join(", "));
        text.push_str(&format!(") -> {ret} {{\n"));
        for (l, t) in f.locals.iter().enumerate() {
            if Some(l as Local) != env
                && let Some(wt) = self.name_of(*t)
            {
                text.push_str(&format!("    var l{l}: {wt};\n"));
            }
        }
        for (k, t) in b.temps.iter().enumerate() {
            text.push_str(&format!("    var t{k}: {};\n", self.name_of(*t)?));
        }
        text.push_str(&b.code);
        text.push_str("}\n\n");
        self.out.push_str(&text);
        Some(name)
    }

    pub(crate) fn program(&self) -> &'p Program {
        self.program
    }

    /// The WGSL type of `t`, noting a record's or tuple's struct.
    pub(crate) fn name_of(&mut self, t: Type) -> Option<String> {
        let name = ty_name(self.program, t)?;
        if let Type::Ref(id) = t
            && crate::vec::vec_type(self.program, t).is_none()
        {
            self.structs.insert(id);
        }
        Some(name)
    }
}

impl<'p> Body<'p> {
    fn line(&mut self, s: &str) {
        self.code.push_str(&"    ".repeat(self.depth));
        self.code.push_str(s);
        self.code.push('\n');
    }

    fn temp(&mut self, t: Type) -> Option<String> {
        ty_name(self.program, t)?;
        self.temps.push(t);
        Some(format!("t{}", self.temps.len() - 1))
    }

    /// A local's name, if a kernel can hold its type.
    fn local(&self, l: Local) -> Option<String> {
        // A texture stays the parameter it came in as.
        if self.is_texture(l) {
            return Some(format!("p{l}"));
        }
        ty_name(self.program, *self.f.locals.get(l as usize)?)?;
        Some(format!("l{l}"))
    }

    /// The check at the top of a loop that ends the item once it has
    /// stopped; gives where it starts, for `drop_halted`.
    fn halted(&mut self) -> usize {
        let at = self.code.len();
        self.line(&format!(
            "if (halted) {{ return {}; }}",
            zero(self.program, self.f.ret)
        ));
        at
    }

    /// Removes the check `halted` wrote at `at` when the loop after it
    /// calls nothing that can stop the item: then it costs a test on
    /// every round for nothing.
    fn drop_halted(&mut self, at: usize) {
        let end = self.code[at..]
            .find('\n')
            .map_or(self.code.len(), |i| at + i + 1);
        if !can_stop(&self.code[end..]) {
            self.code.replace_range(at..end, "");
        }
    }

    fn block(&mut self, w: &mut Writer<'p>, b: &Block) -> Option<()> {
        self.depth += 1;
        for s in &b.stmts {
            self.stmt(w, s)?;
        }
        if let Some(v) = &b.value {
            let v = self.expr(w, v)?;
            if !v.is_empty() {
                self.line(&format!("_ = {v};"));
            }
        }
        self.depth -= 1;
        Some(())
    }

    fn stmt(&mut self, w: &mut Writer<'p>, s: &Stmt) -> Option<()> {
        match s {
            Stmt::Let(l, e) => {
                let v = self.expr(w, e)?;
                let name = self.local(*l)?;
                self.line(&format!("{name} = {v};"));
            }
            // Values with `drop` freed at a block's end: none on the GPU.
            Stmt::Expr(Expr {
                kind: K::Rt(RtOp::DropRun, _),
                ..
            }) => {}
            Stmt::Expr(e) => {
                let v = self.expr(w, e)?;
                if !v.is_empty() && e.ty != Type::Unit {
                    self.line(&format!("_ = {v};"));
                }
            }
            Stmt::While { cond, body } => {
                w.open_loop = true;
                self.line("loop {");
                self.depth += 1;
                let check = self.halted();
                let c = self.expr(w, cond)?;
                self.line(&format!("if (!({c})) {{ break; }}"));
                self.depth -= 1;
                self.block(w, body)?;
                self.drop_halted(check);
                self.line("}");
            }
            Stmt::ForRange {
                var,
                start,
                end,
                inclusive,
                body,
            } => {
                w.open_loop |= *inclusive;
                let name = self.local(*var)?;
                let (s, e) = (self.expr(w, start)?, self.expr(w, end)?);
                let last = self.temp(end.ty)?;
                self.line(&format!("{last} = {e};"));
                self.line(&format!("{name} = {s};"));
                let cmp = if *inclusive { "<=" } else { "<" };
                self.line("loop {");
                self.depth += 1;
                let check = self.halted();
                self.line(&format!("if (!({name} {cmp} {last})) {{ break; }}"));
                self.depth -= 1;
                self.block(w, body)?;
                self.drop_halted(check);
                let one = if start.ty == Type::U32 { "1u" } else { "1" };
                self.line(&format!("    continuing {{ {name} = {name} + {one}; }}"));
                self.line("}");
            }
            Stmt::Return(Some(e)) => {
                let v = self.expr(w, e)?;
                self.line(&format!("return {v};"));
            }
            Stmt::Break => self.line("break;"),
            Stmt::Continue => self.line("continue;"),
            _ => return None,
        }
        Some(())
    }

    /// `e` as a WGSL expression, after the statements it needs; empty for
    /// one with no value.
    pub(crate) fn expr(&mut self, w: &mut Writer<'p>, e: &Expr) -> Option<String> {
        let t = e.ty;
        Some(match &e.kind {
            K::Int(n) => match t {
                Type::U32 => format!("{}u", u32::try_from(*n).ok()?),
                Type::I32 => format!("bitcast<i32>({}u)", i32::try_from(*n).ok()? as u32),
                _ => return None,
            },
            K::Float(f) if t == Type::F32 => {
                format!("bitcast<f32>({:#010x}u)", (*f as f32).to_bits())
            }
            K::Bool(b) => b.to_string(),
            K::Local(l) | K::Move(l) => self.local(*l)?,
            K::Assign(l, x) => {
                let v = self.expr(w, x)?;
                let name = self.local(*l)?;
                self.line(&format!("{name} = {v};"));
                String::new()
            }
            K::GetField(base, k) if matches!(base.kind, K::Local(l) | K::Move(l) if Some(l) == self.env) =>
            {
                wgsl_type(t)?;
                let at = w.capture_base + *k;
                w.captures.insert(at, t);
                format!("cap{at}")
            }
            // A field of a tuple or record item: its own parameter.
            K::GetField(base, k) if matches!(base.kind, K::Local(l) | K::Move(l) if Some(l) == self.item) =>
            {
                wgsl_type(t)?;
                let Some(K::Local(l) | K::Move(l)) = Some(&base.kind) else {
                    return None;
                };
                format!("p{l}_{k}")
            }
            // A field of a record or tuple value.
            K::GetField(base, k) if ty_name(self.program, base.ty).is_some() => {
                let b = self.expr(w, base)?;
                field(self.program, base.ty, &b, *k as i128)
            }
            K::Index(base, k) if ty_name(self.program, base.ty).is_some() => {
                let K::Int(k) = k.kind else { return None };
                let b = self.expr(w, base)?;
                field(self.program, base.ty, &b, k)
            }
            // A record or tuple made of its fields.
            K::Record { variant: 0, fields } | K::Tuple(fields)
                if ty_name(self.program, t).is_some() =>
            {
                let made = w.name_of(t)?;
                let mut vals = Vec::new();
                for x in fields {
                    vals.push(self.expr(w, x)?);
                }
                format!("{made}({})", vals.join(", "))
            }
            K::Unary(op, x) => {
                let v = self.expr(w, x)?;
                match (op, x.ty) {
                    (UnOp::Neg, Type::I32) => format!("neg_i({v}, {}u)", e.line),
                    (UnOp::Neg, Type::F32) => format!("(-{v})"),
                    (UnOp::Not, Type::Bool) => format!("(!{v})"),
                    (UnOp::BitNot, Type::I32 | Type::U32) => format!("(~{v})"),
                    _ => return None,
                }
            }
            K::Binary(op, a, b) => {
                let (x, y) = (self.expr(w, a)?, self.expr(w, b)?);
                // Only 0 or -1 as a divisor can stop an item.
                let safe = matches!(b.kind, K::Int(n) if n != 0 && n != -1);
                match (op, a.ty) {
                    (BinOp::Div, Type::I32) if safe => format!("div_k({x}, {y})"),
                    (BinOp::Rem, Type::I32) if safe => format!("rem_k({x}, {y})"),
                    (BinOp::Div | BinOp::Rem, Type::U32) if safe => {
                        format!("({x} {} {y})", op.symbol())
                    }
                    _ => binary(*op, a.ty, &x, &y, e.line)?,
                }
            }
            K::Fast(op, a, b) => {
                let (x, y) = (self.expr(w, a)?, self.expr(w, b)?);
                match op {
                    FastOp::Add => format!("({x} + {y})"),
                    FastOp::Sub => format!("({x} - {y})"),
                    FastOp::Mul => format!("({x} * {y})"),
                    FastOp::RemOnce => format!("select({x}, {x} - {y}, {x} >= {y})"),
                    // Neither is negative: unsigned, as `div_k` says why.
                    FastOp::DivPositive | FastOp::DivSmall if a.ty == Type::I32 => {
                        format!("bitcast<i32>(bitcast<u32>({x}) / bitcast<u32>({y}))")
                    }
                    FastOp::RemPositive | FastOp::RemSmall if a.ty == Type::I32 => {
                        format!("bitcast<i32>(bitcast<u32>({x}) % bitcast<u32>({y}))")
                    }
                    FastOp::DivPositive | FastOp::DivSmall => format!("({x} / {y})"),
                    FastOp::RemPositive | FastOp::RemSmall => format!("({x} % {y})"),
                }
            }
            K::And(a, b) | K::Or(a, b) => {
                let and = matches!(e.kind, K::And(..));
                let x = self.expr(w, a)?;
                let r = self.temp(Type::Bool)?;
                self.line(&format!("{r} = {x};"));
                self.line(&format!("if ({}{r}) {{", if and { "" } else { "!" }));
                self.depth += 1;
                let y = self.expr(w, b)?;
                self.line(&format!("{r} = {y};"));
                self.depth -= 1;
                self.line("}");
                r
            }
            K::Call(_, args) if args.first().is_some_and(|a| self.texture_ty(a.ty)) => {
                self.sample(w, args, t)?
            }
            K::Call(fid, args) if crate::vec::vec_op(self.program, *fid, args).is_some() => {
                self.vec_call(w, *fid, args)?
            }
            K::Call(fid, args) => {
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.expr(w, a)?);
                }
                // A neighbour's difference, which only the GPU knows
                // (spec 15.3); 0 when it runs on the CPU.
                let builtin = self.program.functions.get(*fid as usize);
                let name = match builtin.is_some_and(|f| f.name == "std.math fwidth") {
                    true => "fwidth".to_string(),
                    false => w.function(*fid, None)?,
                };
                format!("{name}({})", vals.join(", "))
            }
            K::If(c, then, otherwise) => {
                let cond = self.expr(w, c)?;
                let r = if t == Type::Unit {
                    None
                } else {
                    Some(self.temp(t)?)
                };
                self.line(&format!("if ({cond}) {{"));
                self.arm(w, then, r.as_deref())?;
                self.line("} else {");
                self.arm(w, otherwise, r.as_deref())?;
                self.line("}");
                r.unwrap_or_default()
            }
            K::Block(b) => {
                for s in &b.stmts {
                    self.stmt(w, s)?;
                }
                match &b.value {
                    Some(v) => self.expr(w, v)?,
                    None => String::new(),
                }
            }
            K::Convert(x, _) => self.convert(w, x, t)?,
            K::Panic(message) => {
                w.panics.push(message.clone());
                let code = 99 + w.panics.len();
                self.line(&format!("halt({code}u, 0u, 0u, {}u);", e.line));
                if t == Type::Unit {
                    String::new()
                } else {
                    zero(self.program, t)
                }
            }
            _ => return None,
        })
    }

    /// One branch of an `if`, its value into `r`.
    fn arm(&mut self, w: &mut Writer<'p>, b: &Block, r: Option<&str>) -> Option<()> {
        self.depth += 1;
        for s in &b.stmts {
            self.stmt(w, s)?;
        }
        if let Some(v) = &b.value {
            let v = self.expr(w, v)?;
            match r {
                Some(r) if v.is_empty() => {
                    self.line(&format!("{r} = {};", zero(self.program, v_type(b))));
                }
                Some(r) => self.line(&format!("{r} = {v};")),
                None if !v.is_empty() && v_type(b) != Type::Unit => self.line(&format!("_ = {v};")),
                None => {}
            }
        }
        self.depth -= 1;
        Some(())
    }
}

/// The type of block `b`'s value.
fn v_type(b: &Block) -> Type {
    b.value.as_ref().map_or(Type::Unit, |v| v.ty)
}
