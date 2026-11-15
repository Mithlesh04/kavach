//! `std.system` in native code (spec 14.25, decision 0110): the machine's
//! facts from `kavach_runtime_process::machine`, and its GPUs from
//! `kavach_runtime_gpu`, as the interpreter gives them
//! (`compiler/interp/src/std_system.rs`). An entry of its own, so only a
//! program that asks links it (decision 0024).

use crate::heap::{alloc_slots, new_str, shape};
use crate::rt::type_of;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_process::machine;

crate::rt_data::entry!(kv_rt_machine, |op, a, _b, _c, code, _line| {
    match op {
        RtOp::MachineText => new_str(machine::text(a), type_of(code)),
        RtOp::MachineNumber => machine::number(a),
        RtOp::MachineRows | RtOp::MachineApps | RtOp::MachineIds => {
            let item = match shape(type_of(code)) {
                Shape::List(t) => *t,
                _ => Type::I64,
            };
            let rows = match op {
                RtOp::MachineApps => kavach_runtime_process::parts::rows(12),
                RtOp::MachineIds => kavach_runtime_process::parts::rows(13),
                // The applications and ids only through their own ops.
                _ if a >= 12 => Vec::new(),
                _ => kavach_runtime_process::parts::rows(a),
            };
            let slots = rows.into_iter().map(|s| new_str(s, item) as u64).collect();
            alloc_slots(type_of(code), 0, slots)
        }
        _ => {
            let item = match shape(type_of(code)) {
                Shape::List(t) => *t,
                _ => Type::I64,
            };
            let slots = (kavach_runtime_gpu::adapters().into_iter())
                .flatten()
                .map(|s| new_str(s, item) as u64)
                .collect();
            alloc_slots(type_of(code), 0, slots)
        }
    }
});
