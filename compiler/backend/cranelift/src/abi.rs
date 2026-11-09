//! The calling convention between KIR and Cranelift: machine types,
//! function signatures, and the runtime functions native code imports.

use cranelift_codegen::ir::{AbiParam, Signature, types};
use cranelift_module::{FuncId as CFuncId, Linkage, Module};
use kavach_kir::Type;
use kavach_runtime_native::{heap, rt, rt_data, rt_errors, rt_maps, rt_numbers, text_items};

/// The runtime functions native code calls.
pub(crate) struct Runtime {
    pub(crate) fns: Vec<(&'static str, CFuncId)>,
}

impl Runtime {
    pub(crate) fn id(&self, name: &str) -> CFuncId {
        self.fns
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, id)| *id)
            .expect("a runtime function")
    }
}

/// The Cranelift type that stores a KIR type.
pub(crate) fn cl_type(t: Type) -> Option<types::Type> {
    Some(match t {
        Type::I8 | Type::U8 | Type::Bool => types::I8,
        Type::I16 | Type::U16 => types::I16,
        Type::I32 | Type::U32 | Type::Char => types::I32,
        Type::I64 | Type::U64 | Type::Ref(_) => types::I64,
        Type::F64 => types::F64,
        Type::F32 => types::F32,
        Type::Unit => return None,
    })
}

/// A function's signature. A function that can fail also returns its
/// error: a pointer that is 0 on success (spec 5).
pub(crate) fn signature(module: &dyn Module, params: &[Type], ret: Type, fails: bool) -> Signature {
    let mut sig = module.make_signature();
    sig.params
        .extend(params.iter().filter_map(|t| cl_type(*t)).map(AbiParam::new));
    sig.returns.extend(cl_type(ret).map(AbiParam::new));
    if fails {
        sig.returns.push(AbiParam::new(types::I64));
    }
    sig
}

/// A C function's signature (spec 10.7): as C passes them, small integers
/// and booleans widened by the caller, signed or not as their type is.
pub(crate) fn c_signature(module: &dyn Module, params: &[Type], ret: Type) -> Signature {
    let param = |t: Type| {
        let p = AbiParam::new(cl_type(t)?);
        Some(match t {
            Type::I8 | Type::I16 | Type::I32 => p.sext(),
            Type::U8 | Type::U16 | Type::U32 | Type::Bool => p.uext(),
            _ => p,
        })
    };
    let mut sig = module.make_signature();
    sig.params.extend(params.iter().filter_map(|t| param(*t)));
    sig.returns.extend(param(ret));
    sig
}

/// A runtime function: name, address, parameter types, result types.
pub(crate) type RuntimeFn = (
    &'static str,
    *const u8,
    &'static [types::Type],
    &'static [types::Type],
);

/// Every runtime function, in one table used both to register their
/// addresses with the JIT and to declare their signatures.
pub(crate) fn runtime_functions() -> Vec<RuntimeFn> {
    use types::{F64, I8, I32, I64};
    vec![
        (
            "kv_print_str",
            rt::kv_print_str as *const u8,
            &[I64, I64],
            &[],
        ),
        ("kv_print_i64", rt::kv_print_i64 as *const u8, &[I64], &[]),
        ("kv_print_u64", rt::kv_print_u64 as *const u8, &[I64], &[]),
        ("kv_print_f64", rt::kv_print_f64 as *const u8, &[F64], &[]),
        ("kv_print_bool", rt::kv_print_bool as *const u8, &[I8], &[]),
        ("kv_print_end", rt::kv_print_end as *const u8, &[], &[]),
        ("kv_panic", rt::kv_panic as *const u8, &[I64, I64, I32], &[]),
        (
            "kv_overflow",
            rt::kv_overflow as *const u8,
            &[I32, I64, I64, I32, I32],
            &[],
        ),
        (
            "kv_neg_overflow",
            rt::kv_neg_overflow as *const u8,
            &[I64, I32, I32],
            &[],
        ),
        ("kv_div_zero", rt::kv_div_zero as *const u8, &[I32], &[]),
        ("kv_fmod", rt::kv_fmod as *const u8, &[F64, F64], &[F64]),
        (
            "kv_check_int",
            rt::kv_check_int as *const u8,
            &[I64, I32, I32, I32],
            &[I64],
        ),
        (
            "kv_float_to_int",
            rt::kv_float_to_int as *const u8,
            &[F64, I32, I32],
            &[I64],
        ),
        (
            "kv_stack_overflow",
            rt::kv_stack_overflow as *const u8,
            &[I32],
            &[],
        ),
        ("kv_free", heap::kv_free as *const u8, &[I64], &[]),
        (
            "kv_str_len",
            rt_data::kv_str_len as *const u8,
            &[I64],
            &[I64],
        ),
        (
            "kv_text_box",
            text_items::kv_text_box as *const u8,
            &[I64, I64],
            &[I64],
        ),
        (
            "kv_text_store",
            text_items::kv_text_store as *const u8,
            &[I64, I64],
            &[],
        ),
        (
            "kv_text_len",
            text_items::kv_text_len as *const u8,
            &[I64],
            &[I64],
        ),
        (
            "kv_map_get_int",
            rt_maps::kv_map_get_int as *const u8,
            &[I64, I64, I64],
            &[I64],
        ),
        (
            "kv_map_insert",
            rt_maps::kv_map_insert as *const u8,
            &[I64, I64, I64],
            &[I64],
        ),
        (
            "kv_format_text",
            rt_data::kv_format_text as *const u8,
            &[I64, I64, I64, I64],
            &[I64],
        ),
        (
            "kv_bad_index",
            rt_data::kv_bad_index as *const u8,
            &[I64, I64, I32],
            &[],
        ),
        ("kv_retain", rt_data::kv_retain as *const u8, &[I64], &[]),
        ("kv_release", rt_data::kv_release as *const u8, &[I64], &[]),
        (
            "kv_str_new",
            rt_data::kv_str_new as *const u8,
            &[I64, I64, I64],
            &[I64],
        ),
        (
            "kv_format",
            rt_data::kv_format as *const u8,
            &[I64, I64, I64, I64],
            &[I64],
        ),
        (
            "kv_print_value",
            rt_data::kv_print_value as *const u8,
            &[I64, I64],
            &[],
        ),
        (
            "kv_eq",
            rt_data::kv_eq as *const u8,
            &[I64, I64, I64],
            &[I8],
        ),
        ("kv_cmp", rt_data::kv_cmp as *const u8, &[I64, I64], &[I64]),
        (
            "kv_concat",
            rt_data::kv_concat as *const u8,
            &[I64, I64],
            &[I64],
        ),
        (
            "kv_record",
            rt_data::kv_record as *const u8,
            &[I64, I64, I64],
            &[I64],
        ),
        (
            "kv_list",
            rt_data::kv_list as *const u8,
            &[I64, I64],
            &[I64],
        ),
        (
            "kv_init",
            rt_data::kv_init as *const u8,
            &[I64, I64, I64],
            &[],
        ),
        ("kv_get", rt_data::kv_get as *const u8, &[I64, I64], &[I64]),
        (
            "kv_set",
            rt_data::kv_set as *const u8,
            &[I64, I64, I64],
            &[I64],
        ),
        (
            "kv_has_variant",
            rt_data::kv_has_variant as *const u8,
            &[I64, I64],
            &[I8],
        ),
        (
            "kv_index",
            rt_data::kv_index as *const u8,
            &[I64, I64, I32],
            &[I64],
        ),
        (
            "kv_some",
            rt_data::kv_some as *const u8,
            &[I64, I64],
            &[I64],
        ),
        (
            "kv_is_some",
            rt_data::kv_is_some as *const u8,
            &[I64],
            &[I8],
        ),
        (
            "kv_unwrap",
            rt_data::kv_unwrap as *const u8,
            &[I64, I64, I32],
            &[I64],
        ),
        (
            "kv_len_raw",
            rt_data::kv_len_raw as *const u8,
            &[I64],
            &[I64],
        ),
        (
            "kv_item_raw",
            rt_data::kv_item_raw as *const u8,
            &[I64, I64],
            &[I64],
        ),
        (
            "kv_rt",
            rt_data::kv_rt as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_regex",
            kavach_runtime_native::rt_regex::kv_rt_regex as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_json",
            kavach_runtime_native::rt_json::kv_rt_json as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_net",
            kavach_runtime_native::kv_rt_net as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_gpu",
            kavach_runtime_native::kv_rt_gpu as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_drops",
            kavach_runtime_native::kv_rt_drops as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_tasks",
            kavach_runtime_native::kv_rt_tasks as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_agents",
            kavach_runtime_native::kv_rt_agents as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_journal",
            kavach_runtime_native::kv_rt_journal as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_otlp",
            kavach_runtime_native::kv_rt_otlp as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_random",
            kavach_runtime_native::kv_rt_random as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_zone",
            kavach_runtime_native::kv_rt_zone as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_clipboard",
            kavach_runtime_native::kv_rt_clipboard as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_version",
            kavach_runtime_native::kv_rt_version as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_weak",
            kavach_runtime_native::kv_rt_weak as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_process",
            kavach_runtime_native::kv_rt_process as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_serial",
            kavach_runtime_native::kv_rt_serial as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_log",
            kavach_runtime_native::kv_rt_log as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_rt_ai",
            kavach_runtime_native::kv_rt_ai as *const u8,
            &[I64, I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_map_new",
            rt_maps::kv_map_new as *const u8,
            &[I64],
            &[I64],
        ),
        (
            "kv_context",
            rt_errors::kv_context as *const u8,
            &[I64, I64, I64],
            &[I64],
        ),
        (
            "kv_take_error",
            rt_errors::kv_take_error as *const u8,
            &[],
            &[I64],
        ),
        (
            "kv_parse",
            rt_numbers::kv_parse as *const u8,
            &[I64, I64, I64],
            &[I64],
        ),
        (
            "kv_dec_new",
            rt_numbers::kv_dec_new as *const u8,
            &[I64, I64, I64],
            &[I64],
        ),
        (
            "kv_dec_arith",
            rt_numbers::kv_dec_arith as *const u8,
            &[I64, I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_dec_neg",
            rt_numbers::kv_dec_neg as *const u8,
            &[I64, I64],
            &[I64],
        ),
        (
            "kv_dec_from",
            rt_numbers::kv_dec_from as *const u8,
            &[I64, I64, I64, I32],
            &[I64],
        ),
        (
            "kv_dec_to",
            rt_numbers::kv_dec_to as *const u8,
            &[I64, I64, I32],
            &[I64],
        ),
    ]
}

/// Declares the runtime functions in `module`.
pub(crate) fn declare_runtime(module: &mut dyn Module) -> Runtime {
    let fns = runtime_functions()
        .into_iter()
        .map(|(name, _, params, returns)| {
            let mut sig = module.make_signature();
            sig.params.extend(params.iter().map(|t| AbiParam::new(*t)));
            sig.returns
                .extend(returns.iter().map(|t| AbiParam::new(*t)));
            (
                name,
                module
                    .declare_function(name, Linkage::Import, &sig)
                    .expect("declare a runtime function"),
            )
        })
        .collect();
    Runtime { fns }
}
