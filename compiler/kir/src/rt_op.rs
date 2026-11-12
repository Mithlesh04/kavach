//! Built-in operations, carried out by the run-time library (`kv_rt`).

mod codes;
mod groups;

// `RtOp` and `RtOp::ALL`, made by `build.rs` from the op lists in
// `rt_op/ops/`, one file per family.
include!(concat!(env!("OUT_DIR"), "/rt_op.rs"));
