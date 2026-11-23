// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A pipeline's shaders (spec 15.3, decision 0106): one WGSL module with
//! the `gpu vertex fn` and `gpu fragment fn`, as the WGSL backend writes
//! any gpu fn, and `vs_main` and `fs_main` around them. Its first line
//! tells the runtime how to lay out the buffers:
//!
//! `// draw f f | f f f | 1 | 0 | 0`: the vertex's fields (`f` a
//! `Float32`, `i` an `Int32`, `u` a `UInt32`), the instance's, the shared
//! value's number of words, whether it has depth, and how many textures
//! the fragment fn takes. A draw can't stop the program, so in these shaders a
//! panic only marks itself (`halt` does nothing), and the arithmetic goes
//! on with the value it has. Its two fragment entries are `fs_main`, and
//! `fs_mask`, which leaves out the pixels of no opacity, for marking a
//! frame's mask (spec 14.24.1).

use crate::expr::Writer;
use crate::vec::vec_type;
use crate::{PRELUDE, load, wgsl_type};
use kavach_kir::{FuncId, Program, Shape, Type};

/// The letter of a field's type in the first line, and its WGSL type.
fn letter(t: Type) -> Option<(char, &'static str)> {
    Some(match t {
        Type::F32 => ('f', "f32"),
        Type::I32 => ('i', "i32"),
        Type::U32 => ('u', "u32"),
        _ => return None,
    })
}

/// A record's fields: their names and types.
fn fields(program: &Program, t: Type) -> Option<Vec<(String, Type)>> {
    let Type::Ref(id) = t else { return None };
    match program.shapes.get(id as usize)? {
        Shape::Record { fields, .. } => {
            Some(fields.iter().map(|f| (f.name.clone(), f.ty)).collect())
        }
        _ => None,
    }
}

/// The structs a module's functions use, a vector field's struct first.
fn structs(program: &Program, ids: &[u32]) -> Option<String> {
    let mut all: Vec<u32> = Vec::new();
    for &id in ids {
        if !matches!(
            program.shapes.get(id as usize),
            Some(Shape::Record { .. } | Shape::Tuple(..))
        ) {
            return None;
        }
        for t in parts(program, Type::Ref(id))? {
            if let Type::Ref(inner) = t
                && !all.contains(&inner)
            {
                all.push(inner);
            }
        }
        if !all.contains(&id) {
            all.push(id);
        }
    }
    let mut text = String::new();
    // `std.vec`'s are WGSL's own types, with nothing to declare.
    all.retain(|id| vec_type(program, Type::Ref(*id)).is_none());
    for id in all {
        let typed: Vec<String> = (parts(program, Type::Ref(id))?.iter().enumerate())
            .map(|(k, t)| Some(format!("f{k}: {}", type_name(program, *t)?)))
            .collect::<Option<_>>()?;
        text.push_str(&format!("struct S{id} {{ {} }}\n", typed.join(", ")));
    }
    Some(text)
}

/// The field types of record or tuple `t`.
fn parts(program: &Program, t: Type) -> Option<Vec<Type>> {
    let Type::Ref(id) = t else { return None };
    Some(match program.shapes.get(id as usize)? {
        Shape::Tuple(ts) => ts.clone(),
        Shape::Record { fields, .. } => fields.iter().map(|f| f.ty).collect(),
        _ => return None,
    })
}

fn type_name(program: &Program, t: Type) -> Option<String> {
    if let Some(v) = vec_type(program, t) {
        return Some(v.to_string());
    }
    match wgsl_type(t) {
        Some(name) => Some(name.to_string()),
        None => match t {
            Type::Ref(id) => Some(format!("S{id}")),
            _ => None,
        },
    }
}

/// The module of the pipeline of vertex fn `vertex` and fragment fn
/// `fragment`, each as the function of a closure (its first parameter),
/// with depth or not.
pub(crate) fn module(
    program: &Program,
    vertex: FuncId,
    fragment: FuncId,
    depth: bool,
) -> Option<String> {
    let (vf, ff) = (
        program.functions.get(vertex as usize)?,
        program.functions.get(fragment as usize)?,
    );
    let (&venv, inputs) = vf.params.split_first()?;
    let &fenv = ff.params.first()?;
    // After the fragment fn's input: the draw's shared value, if it reads
    // it, then the textures (spec 15.3).
    let after: Vec<Type> = (ff.params.iter().skip(2))
        .map(|p| ff.locals[*p as usize])
        .collect();
    let is_texture = |t: &Type| is_named(program, *t, "Texture");
    let fshared = after.first().copied().filter(|t| !is_texture(t));
    let textures: Vec<Type> = after.iter().copied().filter(is_texture).collect();
    let inputs: Vec<Type> = inputs.iter().map(|p| vf.locals[*p as usize]).collect();
    let corner = fields(program, vf.ret)?;
    let pos = corner.iter().position(|(n, _)| n == "position")?;
    let mut w = Writer::new(program);
    w.texture = textures.first().and_then(|t| struct_id(*t));
    let vname = w.function(vertex, Some(venv))?;
    let fname = w.function(fragment, Some(fenv))?;
    let ids: Vec<u32> = w.structs.iter().copied().collect();

    // The vertex's and the instance's fields, at locations in turn.
    let mut header = Vec::new();
    let mut attributes = Vec::new();
    let mut made = Vec::new();
    let mut at = 0;
    for &t in inputs.iter().take(2) {
        let fs = fields(program, t)?;
        let mut letters = String::new();
        let mut args = Vec::new();
        for (_, ft) in &fs {
            let (l, wt) = letter(*ft)?;
            letters.push(l);
            letters.push(' ');
            attributes.push(format!("@location({at}) a{at}: {wt}"));
            args.push(format!("a.a{at}"));
            at += 1;
        }
        header.push(letters.trim_end().to_string());
        made.push(format!("{}({})", type_name(program, t)?, args.join(", ")));
    }
    while header.len() < 2 {
        header.push(String::new());
    }
    let mut text = String::new();
    // The value shared by the whole draw, as words, read as a map's
    // captures are, by the vertex fn, the fragment fn or both.
    let mut shared_value = String::new();
    if let Some(shared) = inputs.get(2).copied().or(fshared) {
        let fs = fields(program, shared)?;
        header.push(fs.len().to_string());
        text.push_str("@group(0) @binding(0) var<storage, read> shared_words: array<u32>;\n");
        let loads: Vec<String> = (fs.iter().enumerate())
            .map(|(k, (_, t))| load(&format!("shared_words[{k}u]"), *t))
            .collect();
        shared_value = format!("{}({})", type_name(program, shared)?, loads.join(", "));
        if inputs.len() > 2 {
            made.push(shared_value.clone());
        }
    } else {
        header.push("0".to_string());
    }
    header.push(u8::from(depth).to_string());
    header.push(textures.len().to_string());
    let first = format!("// draw {}\n", header.join(" | "));

    // What goes from the vertex fn to the fragment fn: `position`, then
    // each other field.
    let mut outs = vec!["@builtin(position) position: vec4<f32>".to_string()];
    let mut sets = Vec::new();
    let mut back = Vec::new();
    let mut loc = 0;
    for (k, (_, t)) in corner.iter().enumerate() {
        if k == pos && vec_type(program, *t).is_some() {
            // A `Vec4` of `std.vec` is WGSL's `vec4<f32>` already.
            sets.push(format!("    o.position = r.f{k};"));
            back.push("i.position".to_string());
        } else if k == pos {
            let p = format!("r.f{k}");
            sets.push(format!(
                "    o.position = vec4<f32>({p}.f0, {p}.f1, {p}.f2, {p}.f3);"
            ));
            back.push(format!(
                "S{}(i.position.x, i.position.y, i.position.z, i.position.w)",
                struct_id(*t)?
            ));
        } else {
            outs.push(format!("@location({loc}) v{loc}: f32"));
            sets.push(format!("    o.v{loc} = r.f{k};"));
            back.push(format!("i.v{loc}"));
            loc += 1;
        }
    }

    let mut module = first;
    // Checked arithmetic, with nothing to tell when it would panic.
    let helpers = &PRELUDE[PRELUDE.find("fn add_i")?..];
    module.push_str("var<private> halted: bool;\nfn halt(code: u32, a: u32, b: u32, line: u32) { halted = true; }\n");
    module.push_str(helpers);
    module.push_str(&structs(program, &ids)?);
    if !textures.is_empty() {
        module.push_str(&crate::texture::declarations(textures.len(), w.sampled));
    }
    module.push_str(&text);
    if !attributes.is_empty() {
        module.push_str(&format!("struct VIn {{ {} }}\n", attributes.join(", ")));
    }
    module.push_str(&format!("struct VOut {{ {} }}\n", outs.join(", ")));
    module.push_str(&w.out);
    let take = if attributes.is_empty() { "" } else { "a: VIn" };
    module.push_str(&format!(
        "@vertex\nfn vs_main({take}) -> VOut {{\n    let r = {vname}({});\n    var o: VOut;\n{}\n    return o;\n}}\n",
        made.join(", "),
        sets.join("\n")
    ));
    module.push_str(&format!(
        "fn fs_color(i: VOut) -> vec4<f32> {{\n    let c = {fname}(S{}({}){}{});\n    return {};\n}}\n",
        struct_id(vf.ret)?,
        back.join(", "),
        match fshared {
            Some(_) => format!(", {shared_value}"),
            None => String::new(),
        },
        (0..textures.len())
            .map(|k| format!(", ktex{k}"))
            .collect::<String>(),
        // The color: a `Vec4` already, or four fields.
        match vec_type(program, ff.ret) {
            Some(_) => "c".to_string(),
            None => "vec4<f32>(c.f0, c.f1, c.f2, c.f3)".to_string(),
        }
    ));
    module.push_str(
        "@fragment\nfn fs_main(i: VOut) -> @location(0) vec4<f32> {\n    return fs_color(i);\n}\n",
    );
    module.push_str("@fragment\nfn fs_mask(i: VOut) -> @location(0) vec4<f32> {\n    let c = fs_color(i);\n    if (c.w <= 0.0) {\n        discard;\n    }\n    return c;\n}\n");
    Some(module)
}

/// Whether the key `lower_draw.rs` gives a pipeline, its two fns'
/// definitions, says it has depth: `12 14 depth`.
pub fn deep(key: &str) -> bool {
    key.split(' ').nth(2) == Some("depth")
}

/// Whether `t` is the record named `name`, as `Texture`.
fn is_named(program: &Program, t: Type, name: &str) -> bool {
    let shape = struct_id(t).and_then(|id| program.shapes.get(id as usize));
    matches!(shape, Some(Shape::Record { name: n, .. }) if n == name)
}

fn struct_id(t: Type) -> Option<u32> {
    match t {
        Type::Ref(id) => Some(id),
        _ => None,
    }
}

/// Writes each `DrawPipeline`'s module into its text (`attach`).
pub(crate) fn attach_pipelines(program: &mut Program) {
    use kavach_kir::ExprKind;
    use kavach_kir::{RtOp, walk};
    let closures = |e: &kavach_kir::Expr| match &e.kind {
        ExprKind::Rt(RtOp::DrawPipeline, args) => {
            match (&args.first()?.kind, &args.get(1)?.kind, &args.get(2)?.kind) {
                (
                    ExprKind::Closure { func: v, .. },
                    ExprKind::Closure { func: f, .. },
                    ExprKind::Str(key),
                ) => Some((*v, *f, deep(key))),
                _ => None,
            }
        }
        _ => None,
    };
    let mut texts = std::collections::HashMap::new();
    for f in &program.functions {
        walk::any_in_block(&f.body, &mut |e| {
            if let Some(key) = closures(e) {
                texts
                    .entry(key)
                    .or_insert_with(|| module(program, key.0, key.1, key.2).unwrap_or_default());
            }
            false
        });
    }
    for f in &mut program.functions {
        walk::exprs_mut(&mut f.body, &mut |e| {
            let Some(key) = closures(e) else { return };
            if let ExprKind::Rt(_, args) = &mut e.kind
                && let Some(slot) = args.get_mut(2)
            {
                slot.kind = ExprKind::Str(texts.get(&key).cloned().unwrap_or_default());
            }
        });
    }
}

/// Each pipeline's module, by the text its `DrawPipeline` holds before
/// `attach` (the two fns' definitions, `compiler/kir/src/lower_draw.rs`):
/// for the interpreter, which can't write WGSL.
pub fn pipeline_sources(program: &Program) -> Vec<(String, String)> {
    use kavach_kir::{ExprKind, RtOp, walk};
    let mut out = Vec::new();
    for f in &program.functions {
        walk::any_in_block(&f.body, &mut |e| {
            if let ExprKind::Rt(RtOp::DrawPipeline, args) = &e.kind
                && let (
                    ExprKind::Closure { func: v, .. },
                    ExprKind::Closure { func: g, .. },
                    ExprKind::Str(key),
                ) = (&args[0].kind, &args[1].kind, &args[2].kind)
                && let Some(text) = module(program, *v, *g, deep(key))
            {
                out.push((key.clone(), text));
            }
            false
        });
    }
    out
}
