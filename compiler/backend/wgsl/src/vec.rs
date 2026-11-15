//! `std.vec`'s types and operations in WGSL (spec 14.28, decision 0115).
//! A record `Vec2`, `Vec3` or `Vec4` of `Float32` fields `x`, `y`, `z`,
//! `w`, and a `Mat4` of four such `Vec4` columns, are WGSL's own
//! `vec2<f32>` to `mat4x4<f32>`: the same values, so this needs only their
//! shape. Their fields are read by index, as WGSL reads a vector's parts
//! and a matrix's columns. `std.vec`'s own methods, which `kavach-kir`
//! names `std.vec Vec3.dot` and so on, become WGSL's operations.

use crate::expr::{Body, Writer};
use kavach_kir::{Expr, FieldShape, Program, Shape, Type};

/// The WGSL type that record type `t` is written as, if it is one of
/// these.
/// The method of `std.vec` that call `fid(args)` is, when it has a WGSL
/// operation and its first argument is one of these types.
pub(crate) fn vec_op<'a>(program: &'a Program, fid: u32, args: &[Expr]) -> Option<&'a str> {
    let name = &program.functions.get(fid as usize)?.name;
    let method = name.strip_prefix("std.vec ")?.split_once('.')?.1;
    vec_type(program, args.first()?.ty)?;
    const OPS: [&str; 15] = [
        "add",
        "sub",
        "neg",
        "mul",
        "scale",
        "matmul",
        "product",
        "div",
        "dot",
        "length_squared",
        "length",
        "distance",
        "cross",
        "lerp",
        "transposed",
    ];
    OPS.contains(&method).then_some(method)
}

pub(crate) fn vec_type(program: &Program, t: Type) -> Option<&'static str> {
    let Type::Ref(id) = t else { return None };
    let Shape::Record { name, fields, .. } = program.shapes.get(id as usize)? else {
        return None;
    };
    let names: Vec<&str> = fields
        .iter()
        .map(|f: &FieldShape| f.name.as_str())
        .collect();
    let floats = fields.iter().all(|f| f.ty == Type::F32);
    match (name.as_str(), &names[..]) {
        ("Vec2", ["x", "y"]) if floats => Some("vec2<f32>"),
        ("Vec3", ["x", "y", "z"]) if floats => Some("vec3<f32>"),
        ("Vec4", ["x", "y", "z", "w"]) if floats => Some("vec4<f32>"),
        ("Mat4", ["c0", "c1", "c2", "c3"])
            if fields
                .iter()
                .all(|f| vec_type(program, f.ty) == Some("vec4<f32>")) =>
        {
            Some("mat4x4<f32>")
        }
        _ => None,
    }
}

impl<'p> Body<'p> {
    /// A call of `std.vec`'s method `fid` on a value of one of these types,
    /// as WGSL's own operation; `None` to write it as a function.
    pub(crate) fn vec_call(
        &mut self,
        w: &mut Writer<'p>,
        fid: u32,
        args: &[Expr],
    ) -> Option<String> {
        let method = vec_op(w.program(), fid, args)?;
        let mut v = Vec::new();
        for a in args {
            v.push(self.expr(w, a)?);
        }
        let at = |k: usize| v.get(k).cloned().unwrap_or_default();
        Some(match method {
            "add" => format!("({} + {})", at(0), at(1)),
            "sub" => format!("({} - {})", at(0), at(1)),
            "neg" => format!("(-{})", at(0)),
            "mul" | "scale" | "matmul" | "product" => format!("({} * {})", at(0), at(1)),
            "div" => format!("({} / {})", at(0), at(1)),
            "dot" => format!("dot({}, {})", at(0), at(1)),
            "length_squared" => format!("dot({0}, {0})", at(0)),
            "length" => format!("length({})", at(0)),
            "distance" => format!("distance({}, {})", at(0), at(1)),
            "cross" => format!("cross({}, {})", at(0), at(1)),
            "lerp" => format!("mix({}, {}, {})", at(0), at(1), at(2)),
            "transposed" => format!("transpose({})", at(0)),
            _ => return None,
        })
    }
}
