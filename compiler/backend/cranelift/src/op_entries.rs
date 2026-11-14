//! The runtime's op entries (`RtOp::entry`, `compiler/kir/src/rt_op/groups.rs`):
//! each takes an op's code, three operands, the code of the result's type
//! and the line, and gives one value, so they share one signature.

use crate::abi::RuntimeFn;
use cranelift_codegen::ir::types::{I32, I64};
use kavach_runtime_native::rt_data;

/// Every op entry, with the signature they share.
pub(crate) fn op_entries() -> Vec<RuntimeFn> {
    const PARAMS: &[cranelift_codegen::ir::types::Type] = &[I64, I64, I64, I64, I64, I32];
    const RETURNS: &[cranelift_codegen::ir::types::Type] = &[I64];
    let entries: &[(&'static str, *const u8)] = &[
        ("kv_rt", rt_data::kv_rt as *const u8),
        (
            "kv_rt_regex",
            kavach_runtime_native::rt_regex::kv_rt_regex as *const u8,
        ),
        (
            "kv_rt_json",
            kavach_runtime_native::rt_json::kv_rt_json as *const u8,
        ),
        ("kv_rt_net", kavach_runtime_native::kv_rt_net as *const u8),
        ("kv_rt_gpu", kavach_runtime_native::kv_rt_gpu as *const u8),
        (
            "kv_rt_drops",
            kavach_runtime_native::kv_rt_drops as *const u8,
        ),
        (
            "kv_rt_tasks",
            kavach_runtime_native::kv_rt_tasks as *const u8,
        ),
        (
            "kv_rt_agents",
            kavach_runtime_native::kv_rt_agents as *const u8,
        ),
        (
            "kv_rt_journal",
            kavach_runtime_native::kv_rt_journal as *const u8,
        ),
        ("kv_rt_otlp", kavach_runtime_native::kv_rt_otlp as *const u8),
        (
            "kv_rt_random",
            kavach_runtime_native::kv_rt_random as *const u8,
        ),
        ("kv_rt_zone", kavach_runtime_native::kv_rt_zone as *const u8),
        (
            "kv_rt_clipboard",
            kavach_runtime_native::kv_rt_clipboard as *const u8,
        ),
        (
            "kv_rt_crypto",
            kavach_runtime_native::kv_rt_crypto as *const u8,
        ),
        (
            "kv_rt_base64",
            kavach_runtime_native::kv_rt_base64 as *const u8,
        ),
        (
            "kv_rt_compress",
            kavach_runtime_native::kv_rt_compress as *const u8,
        ),
        (
            "kv_rt_window",
            kavach_runtime_native::kv_rt_window as *const u8,
        ),
        ("kv_rt_run", kavach_runtime_native::kv_rt_run as *const u8),
        (
            "kv_rt_writers",
            kavach_runtime_native::kv_rt_writers as *const u8,
        ),
        (
            "kv_rt_scopes",
            kavach_runtime_native::kv_rt_scopes as *const u8,
        ),
        ("kv_rt_draw", kavach_runtime_native::kv_rt_draw as *const u8),
        (
            "kv_rt_normalize",
            kavach_runtime_native::kv_rt_normalize as *const u8,
        ),
        (
            "kv_rt_file_info",
            kavach_runtime_native::kv_rt_file_info as *const u8,
        ),
        (
            "kv_rt_version",
            kavach_runtime_native::kv_rt_version as *const u8,
        ),
        ("kv_rt_weak", kavach_runtime_native::kv_rt_weak as *const u8),
        (
            "kv_rt_process",
            kavach_runtime_native::kv_rt_process as *const u8,
        ),
        (
            "kv_rt_serial",
            kavach_runtime_native::kv_rt_serial as *const u8,
        ),
        ("kv_rt_log", kavach_runtime_native::kv_rt_log as *const u8),
        ("kv_rt_ai", kavach_runtime_native::kv_rt_ai as *const u8),
    ];
    entries
        .iter()
        .map(|&(name, address)| (name, address, PARAMS, RETURNS))
        .collect()
}
