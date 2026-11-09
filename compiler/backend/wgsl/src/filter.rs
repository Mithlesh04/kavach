//! The kernel of `list.filter(f)` on the GPU (spec 15.2), for items of
//! one word. It runs twice, told which by the word after the captured
//! values in `info`. First each workgroup counts the items it keeps;
//! the runtime adds the counts up into where each group's items start,
//! after the mode word. Then each kept item is written at its group's
//! start plus how many the group kept before it, so the items keep their
//! order.

use crate::{chain_parts, store};
use kavach_kir::{FuncId, Program, Type};

/// The kernel for predicate function `fid`, whose closure has `fields`
/// captured values (`info[1]` to `info[fields]`).
pub(crate) fn kernel(program: &Program, fid: FuncId, fields: u32) -> Option<String> {
    let p = chain_parts(program, &[(fid, 0)])?;
    if p.ret != Type::Bool {
        return None;
    }
    let mode = fields + 1;
    let mut text = p.text;
    text.push_str("var<workgroup> kept: array<u32, 256>;\n");
    text.push_str("@compute @workgroup_size(256)\nfn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) t: u32, @builtin(workgroup_id) wid: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {\n    let i = id.x + id.y * groups.x * 256u;\n    let g = wid.x + wid.y * groups.x;\n");
    text.push_str(&p.setup);
    text.push_str(&format!(
        "    var keep = 0u;\n    if (i < info[0]) {{ keep = {}; }}\n",
        store(&p.value, Type::Bool)
    ));
    // How many of the group's items up to each one it keeps.
    text.push_str("    kept[t] = keep;\n    workgroupBarrier();\n    for (var gap = 1u; gap < 256u; gap = gap * 2u) {\n        var before = 0u;\n        if (t >= gap) { before = kept[t - gap]; }\n        workgroupBarrier();\n        kept[t] = kept[t] + before;\n        workgroupBarrier();\n    }\n");
    text.push_str(&format!(
        "    if (info[{mode}u] == 0u) {{\n        if (t == 255u) {{ out[g] = kept[255]; }}\n    }} else if (keep == 1u) {{\n        out[info[{mode}u + 1u + g] + kept[t] - 1u] = items[i];\n    }}\n}}\n"
    ));
    Some(text)
}
