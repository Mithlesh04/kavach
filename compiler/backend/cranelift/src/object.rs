// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `kavach build`: compiles a program to an object file with a C `main`,
//! then links it with the runtime library into an executable
//! (decision 0016).

use crate::codegen::place_text;
use crate::{compile, link, new_isa};
use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{
    DataDescription, DataId, FuncId, FuncOrDataId, Linkage, Module, default_libcall_names,
};
use cranelift_object::{ObjectBuilder, ObjectModule};
use kavach_kir::{Program, encode_shapes};
use std::path::Path;

/// Compiles `program` and writes the executable `out`, as `exe` asks.
pub fn build(program: &Program, out: &Path, exe: link::Exe) -> Result<(), String> {
    let mut clock = kavach_kir::times::Clock::start();
    // Executables on Linux and macOS are position-independent.
    let isa = new_isa(!cfg!(windows))?;
    let builder = ObjectBuilder::new(isa.clone(), "kavach_program", default_libcall_names())
        .map_err(|e| e.to_string())?;
    let mut module = ObjectModule::new(builder);
    let mut unwinds = crate::unwind::Unwinds::default();
    let entry = compile(&mut module, program, Some(&mut unwinds))?;
    let main = &program.functions[program.entry as usize];
    c_main(&mut module, program, entry, main.fails)?;
    let mut product = module.finish();
    crate::unwind::write(&mut product, &*isa, unwinds)?;
    let bytes = product.emit().map_err(|e| e.to_string())?;

    let dir = crate::link::build_folder("kavach-build");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let object = dir.join(if cfg!(windows) {
        "program.obj"
    } else {
        "program.o"
    });
    std::fs::write(&object, bytes).map_err(|e| e.to_string())?;
    let libs: Vec<String> = {
        let mut l: Vec<String> = program
            .functions
            .iter()
            .filter_map(|f| f.external.clone())
            .collect();
        l.extend(kavach_kir::system_libraries(program));
        l.sort();
        l.dedup();
        l
    };
    clock.lap("codegen");
    let linked = link::link(&object, out, &libs, exe);
    clock.lap("link");
    let _ = std::fs::remove_dir_all(&dir);
    linked
}

/// `int main(int argc, char **argv)`: hands the entry function, the file
/// path and the shape table to the runtime's `kv_main`.
fn c_main(
    module: &mut ObjectModule,
    program: &Program,
    entry: FuncId,
    fails: bool,
) -> Result<(), String> {
    let mut sig = module.make_signature();
    sig.params.push(AbiParam::new(types::I32));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I32));
    let main = module
        .declare_function("main", Linkage::Export, &sig)
        .map_err(|e| e.to_string())?;

    let mut start_sig = module.make_signature();
    for t in [
        types::I64,
        types::I8,
        types::I64,
        types::I64,
        types::I64,
        types::I64,
        types::I64,
    ] {
        start_sig.params.push(AbiParam::new(t));
    }
    start_sig.returns.push(AbiParam::new(types::I32));
    let kv_main = module
        .declare_function("kv_main", Linkage::Import, &start_sig)
        .map_err(|e| e.to_string())?;
    let path = place_text(module, &program.path);
    let shapes_text = encode_shapes(&program.shapes);
    let shapes = place_text(module, &shapes_text);
    // Each function's start, name and line, for crash reports (14.30).
    let names_text = kavach_runtime_native::crash_stack::names_text(&program.functions);
    let names = place_text(module, &names_text);
    let starts = function_table(module, program)?;
    let mut table_sig = module.make_signature();
    for _ in 0..4 {
        table_sig.params.push(AbiParam::new(types::I64));
    }
    let set_functions = module
        .declare_function("kv_set_functions", Linkage::Import, &table_sig)
        .map_err(|e| e.to_string())?;

    let mut ctx = module.make_context();
    ctx.func.signature = sig;
    let mut fctx = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
    let block = b.create_block();
    b.append_block_params_for_function_params(block);
    b.switch_to_block(block);
    let entry_ref = module.declare_func_in_func(entry, b.func);
    let entry_addr = b.ins().func_addr(types::I64, entry_ref);
    let fails = b.ins().iconst(types::I8, fails as i64);
    let set = module.declare_func_in_func(set_functions, b.func);
    // `--tracy`: Tracy's client starts first (decision 0123).
    if program.tracy {
        let tracy = module
            .declare_function("kv_tracy_start", Linkage::Import, &module.make_signature())
            .map_err(|e| e.to_string())?;
        let tracy = module.declare_func_in_func(tracy, b.func);
        b.ins().call(tracy, &[]);
    }
    let text = |id: DataId, len: usize, b: &mut FunctionBuilder| {
        let gv = module.declare_data_in_func(id, b.func);
        let ptr = b.ins().symbol_value(types::I64, gv);
        let len = b.ins().iconst(types::I64, len as i64);
        (ptr, len)
    };
    let (names_ptr, names_len) = text(names, names_text.len(), &mut b);
    let (starts_ptr, count) = text(starts, program.functions.len(), &mut b);
    b.ins()
        .call(set, &[names_ptr, names_len, starts_ptr, count]);
    let (path_ptr, path_len) = text(path, program.path.len(), &mut b);
    let (shapes_ptr, shapes_len) = text(shapes, shapes_text.len(), &mut b);
    let start = module.declare_func_in_func(kv_main, b.func);
    // No per-thread stack limit: tasks take turns (`kv_main`).
    let no_slot = b.ins().iconst(types::I64, 0);
    let call = b.ins().call(
        start,
        &[
            entry_addr, fails, path_ptr, path_len, shapes_ptr, shapes_len, no_slot,
        ],
    );
    let code = b.inst_results(call)[0];
    b.ins().return_(&[code]);
    b.seal_all_blocks();
    b.finalize();
    module
        .define_function(main, &mut ctx)
        .map_err(|e| format!("main: {e:?}"))
}

/// The start of every function, by number, as data (0 for a C function):
/// what a crash report names frames by (`crash_stack.rs`).
fn function_table(module: &mut ObjectModule, program: &Program) -> Result<DataId, String> {
    let id = module
        .declare_anonymous_data(false, false)
        .map_err(|e| e.to_string())?;
    let mut data = DataDescription::new();
    // Written data, not zeroed: relocations aren't applied in `.bss`.
    data.define(vec![0u8; program.functions.len().max(1) * 8].into_boxed_slice());
    data.set_align(8);
    for i in 0..program.functions.len() {
        if let Some(FuncOrDataId::Func(f)) = module.get_name(&format!("kv_fn_{i}")) {
            let r = module.declare_func_in_data(f, &mut data);
            data.write_function_addr((i * 8) as u32, r);
        }
    }
    module.define_data(id, &data).map_err(|e| e.to_string())?;
    Ok(id)
}
