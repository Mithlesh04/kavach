// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! KIR to WGSL: the kernel of each `map` that may run on the GPU (spec
//! 15, decision 0044). [`attach`] writes it into the `GpuMap` call's text,
//! which the GPU runtime (`runtime/gpu`) compiles at first use.
//!
//! A kernel reads its items from `items`, writes `out`, reads the item
//! count and the captured values from `info`, and records the first item
//! that panics in `stop`: a flag, a code and two operands. Codes: `10 * k
//! + t` for operator `k` (1 `+`, 2 `-`, 3 `*`, 4 `/`, 5 `%`, 7 `<<`, 8
//! `>>`, 9 negation) on type `t` (1 `Int32`, 2 `UInt32`); 20 division by
//! zero; `300 + 10 * from + to` for a conversion that doesn't fit (3
//! `Float32`); 100 and up for a panic with a message, listed in the
//! kernel's first lines as `// stop <code>: <message>`. Its very first
//! line, `// work <n>`, says roughly how many operations each item takes
//! (`cost.rs`), so the runtime can tell whether the GPU is worth it; a
//! `map` whose result is a record or tuple says next how many words each
//! item's result takes, `// out <n>`.
//!
//! A kernel whose every loop surely ends says `// loops end`, and is
//! compiled without `wgpu`'s counter in each loop (decision 0077).

mod render;
pub use render::{deep, pipeline_sources};
mod convert;
mod cost;
mod expr;
mod filter;
mod ops;
mod prelude;
mod reduce;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_draw;
mod texture;
mod values;
mod vec;

use kavach_kir::{Expr, ExprKind, FuncId, Program, RtOp, Shape, Stmt, Type, walk};
use std::collections::HashMap;

pub(crate) use prelude::{PRELUDE, load, store, wgsl_type};

/// The line of a kernel whose every loop surely ends (decision 0077).
pub const LOOPS_END: &str = "// loops end";

/// The kernel of the `map` whose function is closure function `fid`
/// (its parameters: the closure, then the item), or `None` when KIR uses
/// something a kernel can't do; then the CPU runs the `map`.
pub fn kernel(program: &Program, fid: FuncId) -> Option<String> {
    chain_kernel(program, &[(fid, 0)])
}

/// The kernel of a chain of `map`s, `xs.map(f).map(g)` (spec 15.2): each
/// item goes through every closure function in turn, on the GPU, so the
/// lists between them never come back.
fn chain_kernel(program: &Program, stages: &[(FuncId, u32)]) -> Option<String> {
    let p = chain_parts(program, stages)?;
    let mut text = p.text;
    let mut value = p.value;
    // An item of four words (a record or tuple of four 32-bit fields) is
    // read as one `vec4`: the GPU reads 16 bytes at once faster than four
    // words apart (300 frames: 30-37 ms as words, 19-22 ms as vec4,
    // benchmarks/gpu).
    if value.contains("items[i * 4u + 0u]") {
        text = text.replacen("items: array<u32>;", "items: array<vec4<u32>>;", 1);
        for (k, lane) in ["x", "y", "z", "w"].iter().enumerate() {
            value = value.replace(
                &format!("items[i * 4u + {k}u]"),
                &format!("items[i].{lane}"),
            );
        }
    }
    text.push_str(
        "@compute @workgroup_size(256)\nfn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {\n    let i = id.x + id.y * groups.x * 256u;\n    if (i >= info[0]) { return; }\n",
    );
    text.push_str(&p.setup);
    match struct_fields(program, p.ret) {
        // A record or tuple result: its fields, `width` words per item.
        // The kernel's second line tells the runtime the width.
        Some(fields) => {
            let width = fields.len();
            let second = text.find('\n').map_or(0, |i| i + 1);
            text.insert_str(second, &format!("// out {width}\n"));
            text.push_str(&format!("    let r = {value};\n"));
            let words: Vec<String> = (fields.iter().enumerate())
                .map(|(k, t)| store(&format!("r.f{k}"), *t))
                .collect();
            // Four words go out as one `vec4`, as they come in.
            if width == 4 {
                text = text.replacen("out: array<u32>;", "out: array<vec4<u32>>;", 1);
                text.push_str(&format!("    out[i] = vec4<u32>({});\n", words.join(", ")));
            } else {
                for (k, word) in words.iter().enumerate() {
                    text.push_str(&format!("    out[i * {width}u + {k}u] = {word};\n"));
                }
            }
            text.push_str("}\n");
        }
        None => text.push_str(&format!("    out[i] = {};\n}}\n", store(&value, p.ret))),
    }
    Some(text)
}

/// A kernel's text before its `main`, what `main` does first, the value
/// of item `i` after the chain's functions, and its type.
pub(crate) struct Parts {
    pub text: String,
    pub setup: String,
    pub value: String,
    pub ret: Type,
}

/// The parts of a chain's kernel. With each function, how many captures
/// the closures before it have, so its own follow them in `info`. Between
/// two functions, the value must be one word.
pub(crate) fn chain_parts(program: &Program, stages: &[(FuncId, u32)]) -> Option<Parts> {
    let &(fid, _) = stages.first()?;
    let f = program.functions.get(fid as usize)?;
    let [env, item] = f.params[..] else {
        return None;
    };
    let item_ty = f.locals[item as usize];
    let mut ret = f.ret;
    for &(next, _) in &stages[1..] {
        ty_name(program, ret)?;
        let g = program.functions.get(next as usize)?;
        let [_, x] = g.params[..] else {
            return None;
        };
        if g.locals[x as usize] != ret {
            return None;
        }
        ret = g.ret;
    }
    ty_name(program, ret)?;
    // A tuple or record item (spec 15.2) is its fields, one word each.
    let fields = match wgsl_type(item_ty) {
        Some(_) => Vec::new(),
        None => struct_fields(program, item_ty)?,
    };
    let mut w = expr::Writer::new(program);
    if !fields.is_empty() {
        w.item = Some((item, fields.clone()));
    }
    let entry = w.function(fid, Some(env))?;
    let mut calls = format!("{entry}(ITEM)");
    for &(next, base) in &stages[1..] {
        w.item = None;
        w.capture_base = base;
        let g = &program.functions[next as usize];
        let name = w.function(next, Some(g.params[0]))?;
        calls = format!("{name}({calls})");
    }
    let mut captures: Vec<(u32, Type)> = w.captures.iter().map(|(k, t)| (*k, *t)).collect();
    captures.sort_by_key(|(k, _)| *k);
    let work: u64 = stages.iter().map(|(f, _)| cost::work(program, *f)).sum();
    let mut text = format!("// work {work}\n");
    if !w.open_loop {
        text.push_str(LOOPS_END);
        text.push('\n');
    }
    for (code, message) in w.panics.iter().enumerate() {
        text.push_str(&format!(
            "// stop {}: {}\n",
            100 + code,
            message.replace('\n', " ")
        ));
    }
    text.push_str(PRELUDE);
    for &id in &w.structs {
        let fields = struct_fields(program, Type::Ref(id))?;
        let typed: Vec<String> = (fields.iter().enumerate())
            .map(|(k, t)| Some(format!("f{k}: {}", wgsl_type(*t)?)))
            .collect::<Option<_>>()?;
        text.push_str(&format!("struct S{id} {{ {} }}\n", typed.join(", ")));
    }
    for (k, t) in &captures {
        text.push_str(&format!("var<private> cap{k}: {};\n", wgsl_type(*t)?));
    }
    text.push_str(&w.out);
    let mut setup = String::new();
    // Field `k` of the closure is `info[k]`: the runtime sends them all.
    for (k, t) in &captures {
        setup.push_str(&format!(
            "    cap{k} = {};\n",
            load(&format!("info[{k}u]"), *t)
        ));
    }
    let item = match fields.len() {
        0 => load("items[i]", item_ty),
        width => fields
            .iter()
            .enumerate()
            .map(|(k, t)| load(&format!("items[i * {width}u + {k}u]"), *t))
            .collect::<Vec<_>>()
            .join(", "),
    };
    Some(Parts {
        text,
        setup,
        value: calls.replace("ITEM", &item),
        ret,
    })
}

/// The WGSL type of `t`: one of the four types, or the struct of a record
/// or tuple of them (`S<shape>`).
pub(crate) fn ty_name(program: &Program, t: Type) -> Option<String> {
    if let Some(name) = wgsl_type(t) {
        return Some(name.to_string());
    }
    let Type::Ref(id) = t else { return None };
    // `std.vec`'s vectors and matrices are WGSL's own (spec 14.28).
    if let Some(v) = crate::vec::vec_type(program, t) {
        return Some(v.to_string());
    }
    // Fields of the four types, or one level down a vector (a draw's
    // `position`, spec 15.3); a `map`'s items stay flat (`struct_fields`).
    let parts: Vec<Type> = match program.shapes.get(id as usize)? {
        Shape::Tuple(ts) => ts.clone(),
        Shape::Record { fields, .. } => fields.iter().map(|f| f.ty).collect(),
        _ => return None,
    };
    for p in parts {
        if wgsl_type(p).is_none() {
            struct_fields(program, p)?;
        }
    }
    Some(format!("S{id}"))
}

/// The field types of tuple or record type `t`, when a kernel can hold
/// each of them.
pub(crate) fn struct_fields(program: &Program, t: Type) -> Option<Vec<Type>> {
    let Type::Ref(id) = t else { return None };
    let fields: Vec<Type> = match program.shapes.get(id as usize)? {
        Shape::Tuple(ts) => ts.clone(),
        Shape::Record { fields, .. } => fields.iter().map(|f| f.ty).collect(),
        _ => return None,
    };
    for f in &fields {
        wgsl_type(*f)?;
    }
    Some(fields)
}

/// Writes each `GpuMap` call's kernel into its text, so the runtime can
/// run it. A `map` whose kernel can't be written keeps an empty text, and
/// runs on the CPU.
pub fn attach(program: &mut Program) {
    render::attach_pipelines(program);
    let mut texts: HashMap<String, String> = HashMap::new();
    for f in &program.functions {
        walk::any_in_block(&f.body, &mut |e| {
            if let Some(key) = kernel_key(e) {
                texts
                    .entry(key)
                    .or_insert_with(|| kernel_of(program, e).unwrap_or_default());
            }
            false
        });
    }
    // Opening the GPU takes a few hundred milliseconds: start it, and the
    // kernels' compiling, as the program starts.
    let kernels: Vec<&str> = texts
        .values()
        .map(String::as_str)
        .filter(|t| !t.is_empty())
        .collect();
    if !kernels.is_empty() {
        let text = kernels.join("\0");
        if let Some(entry) = program.functions.get_mut(program.entry as usize) {
            let at = |kind, ty| Expr {
                kind,
                ty,
                line: entry.line,
            };
            let str_ty = program.shapes.iter().position(|s| *s == Shape::Str);
            if let Some(id) = str_ty {
                let warm = at(
                    ExprKind::Rt(
                        RtOp::GpuWarm,
                        vec![at(ExprKind::Str(text), Type::Ref(id as u32))],
                    ),
                    Type::Unit,
                );
                entry.body.stmts.insert(0, Stmt::Expr(warm));
            }
        }
    }
    for f in &mut program.functions {
        walk::exprs_mut(&mut f.body, &mut |e| {
            let Some(key) = kernel_key(e) else { return };
            let text = texts.get(&key).cloned().unwrap_or_default();
            if let ExprKind::Rt(_, args) = &mut e.kind
                && let Some(slot) = args.get_mut(2)
            {
                slot.kind = ExprKind::Str(text);
            }
        });
    }
}

/// Which kernel a GPU op runs: its op, its list's type and its closures'
/// functions. `None` for any other expression.
fn kernel_key(e: &Expr) -> Option<String> {
    let ExprKind::Rt(op, args) = &e.kind else {
        return None;
    };
    let (list, f) = (args.first()?, args.get(1)?);
    let funcs: Vec<FuncId> = stages(f).unwrap_or_default().iter().map(|s| s.0).collect();
    let op = match op {
        RtOp::GpuMap | RtOp::GpuMapStay => "map",
        RtOp::GpuFilter | RtOp::GpuFilterStay => "filter",
        RtOp::GpuSum | RtOp::GpuSumLater => "sum",
        RtOp::GpuMin | RtOp::GpuMinLater => "min",
        RtOp::GpuMax | RtOp::GpuMaxLater => "max",
        _ => return None,
    };
    Some(format!("{op} {:?} {funcs:?}", list.ty))
}

/// The kernel of GPU op `e`, or `None` when its work can't be one.
fn kernel_of(program: &Program, e: &Expr) -> Option<String> {
    let ExprKind::Rt(op, args) = &e.kind else {
        return None;
    };
    let item = match program.shapes.get(match args.first()?.ty {
        Type::Ref(id) => id as usize,
        _ => return None,
    })? {
        Shape::List(t) => *t,
        _ => return None,
    };
    match op {
        RtOp::GpuMap | RtOp::GpuMapStay => chain_kernel(program, &stages(args.get(1)?)?),
        RtOp::GpuFilter | RtOp::GpuFilterStay => {
            let ExprKind::Closure { func, captures, .. } = &args.get(1)?.kind else {
                return None;
            };
            wgsl_type(item)?;
            filter::kernel(program, *func, captures.len() as u32)
        }
        RtOp::GpuSum | RtOp::GpuSumLater => reduce::kernel(
            program,
            Some(&stages(args.get(1)?)?),
            item,
            reduce::Reduce::Sum,
        ),
        RtOp::GpuMin | RtOp::GpuMinLater => {
            reduce::kernel(program, None, item, reduce::Reduce::Min)
        }
        RtOp::GpuMax | RtOp::GpuMaxLater => {
            reduce::kernel(program, None, item, reduce::Reduce::Max)
        }
        _ => None,
    }
}

/// The closure functions a `GpuMap` or `GpuSum` runs, in order, each with how many
/// captures the ones before it have: one closure, or a pair of them for
/// `xs.map(f).map(g)` (`compiler/kir/src/lower_gpu.rs`).
fn stages(arg: &Expr) -> Option<Vec<(FuncId, u32)>> {
    let closures = match &arg.kind {
        ExprKind::Closure { .. } => std::slice::from_ref(arg),
        ExprKind::Tuple(items) => &items[..],
        _ => return None,
    };
    let mut base = 0;
    let mut out = Vec::new();
    for c in closures {
        let ExprKind::Closure { func, captures, .. } = &c.kind else {
            return None;
        };
        out.push((*func, base));
        base += captures.len() as u32;
    }
    Some(out)
}
