//! Unwind tables for code the JIT makes (`kavach run --native`; decision
//! 0065, step 3), as `unwind.rs` writes them into object files: so a
//! panic unwinds through compiled Kavach code to the start of its task.
//! The JIT knows each function's address only once its code is placed,
//! so the tables are built then, and the runtime hands them to the system
//! (`runtime/native/src/unwind_tables.rs`).
//!
//! On Windows a function table holds 32-bit offsets from one base, to the
//! code and to its unwind codes, so the codes go into a data object the
//! JIT places beside the code. If the two land too far apart, nothing is
//! registered, and a panic in that program ends it, as before.

use crate::unwind::Unwinds;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::isa::unwind::UnwindInfo;
use cranelift_jit::JITModule;
use cranelift_module::{DataDescription, DataId, Linkage, Module};
use gimli::write::{Address, EhFrame, EndianVec, FrameTable};
use kavach_runtime_native::unwind_tables::{self, RuntimeFunction};

/// What is made before the code is placed: on Windows, the data object
/// holding every function's unwind codes, where each one starts, and its
/// length.
pub(crate) struct Prepared(Option<(DataId, Vec<u32>, u64)>);

/// Before `finalize_definitions`: the unwind codes of Windows functions,
/// in one data object of `module`.
pub(crate) fn prepare(module: &mut JITModule, unwinds: &Unwinds) -> Result<Prepared, String> {
    let mut codes = Vec::new();
    let mut starts = Vec::new();
    for (_, info, _) in &unwinds.0 {
        let UnwindInfo::WindowsX64(w) = info else {
            return Ok(Prepared(None));
        };
        while codes.len() % 4 != 0 {
            codes.push(0);
        }
        starts.push(codes.len() as u32);
        let mut bytes = vec![0; w.emit_size()];
        w.emit(&mut bytes);
        codes.extend(bytes);
    }
    if starts.is_empty() {
        return Ok(Prepared(None));
    }
    let length = codes.len() as u64;
    let id = (module.declare_data("kv_unwind_codes", Linkage::Local, false, false))
        .map_err(|e| e.to_string())?;
    let mut data = DataDescription::new();
    data.define(codes.into_boxed_slice());
    data.set_align(4);
    module.define_data(id, &data).map_err(|e| e.to_string())?;
    Ok(Prepared(Some((id, starts, length))))
}

/// After `finalize_definitions`: registers the tables of every function.
pub(crate) fn register(
    module: &JITModule,
    isa: &dyn TargetIsa,
    unwinds: Unwinds,
    prepared: Prepared,
) {
    let placed: Vec<(u64, UnwindInfo, u32)> = (unwinds.0.into_iter())
        .map(|(id, info, size)| (module.get_finalized_function(id) as u64, info, size))
        .collect();
    match prepared.0 {
        Some((data, starts, length)) => windows(module, (data, length), &starts, &placed),
        None => system_v(isa, &placed),
    }
}

/// `.eh_frame` with each function's absolute address, ended by a zero word.
fn system_v(isa: &dyn TargetIsa, placed: &[(u64, UnwindInfo, u32)]) {
    let Some(cie) = isa.create_systemv_cie() else {
        return;
    };
    let mut table = FrameTable::default();
    let cie = table.add_cie(cie);
    for (at, info, _) in placed {
        if let UnwindInfo::SystemV(u) = info {
            table.add_fde(cie, u.to_fde(Address::Constant(*at)));
        }
    }
    let mut eh = EhFrame(EndianVec::new(gimli::LittleEndian));
    if table.write_eh_frame(&mut eh).is_err() {
        return;
    }
    let mut section = eh.0.into_vec();
    section.extend_from_slice(&[0; 4]);
    unwind_tables::register_eh_frame(section);
}

/// A Windows function table: each function's start, end and unwind codes,
/// relative to the lowest of them.
fn windows(
    module: &JITModule,
    (data, length): (DataId, u64),
    starts: &[u32],
    placed: &[(u64, UnwindInfo, u32)],
) {
    let codes = module.get_finalized_data(data).0 as u64;
    let lowest = placed
        .iter()
        .map(|(at, ..)| *at)
        .chain([codes])
        .min()
        .unwrap_or(0);
    let highest = (placed.iter().map(|(at, _, size)| at + u64::from(*size)))
        .chain([codes + length])
        .max()
        .unwrap_or(0);
    if highest - lowest > u64::from(u32::MAX) {
        return;
    }
    let offset = |at: u64| (at - lowest) as u32;
    let mut table: Vec<RuntimeFunction> = (placed.iter().zip(starts))
        .map(|((at, _, size), start)| RuntimeFunction {
            begin: offset(*at),
            end: offset(at + u64::from(*size)),
            unwind: offset(codes) + start,
        })
        .collect();
    table.sort_by_key(|f| f.begin);
    unwind_tables::add_function_table(table, lowest);
}
