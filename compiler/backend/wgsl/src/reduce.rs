//! Kernels for `sum`, `min` and `max` of `Int32` or `UInt32` on the GPU
//! (spec 15.2). Each workgroup combines its 256 items in their order and
//! writes one partial into `out`, three words per group; the runtime
//! combines the partials in order (`runtime/native/src/rt_gpu.rs`).
//!
//! A `sum` partial is the sum, the lowest and the highest running sum
//! from the group's start (counting the empty start, 0), so the runtime
//! can tell whether any running sum of the whole list, as the CPU adds
//! it, leaves the type. Every add is checked: one that overflows stops
//! the kernel with code 1, and the CPU adds the list again in order, so
//! a real overflow panics at the right item (decision 0031's rule).

use crate::{PRELUDE, Parts, chain_parts, load, store, wgsl_type};
use kavach_kir::{FuncId, Program, Type};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Reduce {
    Sum,
    Min,
    Max,
}

/// The kernel that reduces the list's items of type `item`, each first
/// through the chain of closure functions `stages` for a `sum`.
pub(crate) fn kernel(
    program: &Program,
    stages: Option<&[(FuncId, u32)]>,
    item: Type,
    kind: Reduce,
) -> Option<String> {
    let p = match stages {
        Some(stages) => chain_parts(program, stages)?,
        None => Parts {
            text: format!("// work 1\n{PRELUDE}"),
            setup: String::new(),
            value: load("items[i]", item),
            ret: item,
        },
    };
    let t = match p.ret {
        Type::I32 | Type::U32 => wgsl_type(p.ret)?,
        _ => return None,
    };
    let (lowest, highest, overflows) = match p.ret {
        Type::I32 => (
            "(i32(-2147483647) - 1)",
            "2147483647",
            "((a ^ r) & (b ^ r)) < 0",
        ),
        _ => ("0u", "4294967295u", "r < a"),
    };
    let zero = if p.ret == Type::I32 { "0" } else { "0u" };
    let mut text = p.text;
    text.push_str(&format!(
        "fn radd(a: {t}, b: {t}) -> {t} {{ let r = a + b; if ({overflows}) {{ halt(1u, 0u, 0u, 0u); }} return r; }}\n"
    ));
    text.push_str(&format!(
        "var<workgroup> rs: array<{t}, 256>;\nvar<workgroup> rl: array<{t}, 256>;\nvar<workgroup> rh: array<{t}, 256>;\n"
    ));
    text.push_str("@compute @workgroup_size(256)\nfn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) t: u32, @builtin(workgroup_id) wid: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {\n    let i = id.x + id.y * groups.x * 256u;\n");
    text.push_str(&p.setup);
    let empty = match kind {
        Reduce::Sum => zero,
        Reduce::Min => highest,
        Reduce::Max => lowest,
    };
    text.push_str(&format!(
        "    var v: {t} = {empty};\n    if (i < info[0]) {{ v = {}; }}\n",
        p.value
    ));
    let combine = match kind {
        Reduce::Sum => {
            text.push_str(&format!(
                "    rs[t] = v;\n    rl[t] = min({zero}, v);\n    rh[t] = max({zero}, v);\n"
            ));
            "            let s = rs[t];\n            rl[t] = min(rl[t], radd(s, rl[t + gap]));\n            rh[t] = max(rh[t], radd(s, rh[t + gap]));\n            rs[t] = radd(s, rs[t + gap]);\n"
        }
        Reduce::Min => {
            text.push_str("    rs[t] = v;\n");
            "            rs[t] = min(rs[t], rs[t + gap]);\n"
        }
        Reduce::Max => {
            text.push_str("    rs[t] = v;\n");
            "            rs[t] = max(rs[t], rs[t + gap]);\n"
        }
    };
    text.push_str("    workgroupBarrier();\n    for (var gap = 1u; gap < 256u; gap = gap * 2u) {\n        if (t % (2u * gap) == 0u) {\n");
    text.push_str(combine);
    text.push_str("        }\n        workgroupBarrier();\n    }\n");
    let word = |v: &str| store(v, p.ret);
    text.push_str(&format!(
        "    if (t == 0u) {{\n        let g = wid.x + wid.y * groups.x;\n        out[3u * g] = {};\n        out[3u * g + 1u] = {};\n        out[3u * g + 2u] = {};\n    }}\n}}\n",
        word("rs[0]"),
        word("rl[0]"),
        word("rh[0]")
    ));
    Some(text)
}
