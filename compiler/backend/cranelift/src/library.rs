//! `kavach build --lib` (decision 0026): the program's functions in an
//! object file with one exported C function per export, each in front of
//! the Kavach function: it sets up the runtime (`kv_lib_enter`), turns C
//! text into Kavach text, calls it, turns a text result into C text, and
//! writes out what it printed (`kv_lib_leave`). A function that can fail
//! returns its error's number instead, its value through one more
//! parameter (`kv_lib_error`). With `free` and `last_error`, the object
//! is linked into a shared library.

use crate::abi::signature;
use crate::codegen::place_text;
use crate::{compile, link, new_isa};
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{DataId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use kavach_kir::{Program, Shape, Type, encode_shapes};
use kavach_runtime_native::rt::type_code;
use std::path::Path;

/// Compiles `program` and writes the shared library `out`, exporting
/// `program.exports` under their C names, `free_name`, which frees text
/// the library gave, and `last_error_name`, which gives the last error.
pub fn build_library(
    program: &Program,
    out: &Path,
    free_name: &str,
    last_error_name: &str,
) -> Result<(), String> {
    // A shared library on Linux and macOS is position-independent.
    let isa = new_isa(!cfg!(windows))?;
    let builder = ObjectBuilder::new(isa, "kavach_library", default_libcall_names())
        .map_err(|e| e.to_string())?;
    let mut module = ObjectModule::new(builder);
    compile(&mut module, program, None)?;
    let path = place_text(&mut module, &program.path);
    let shapes_text = encode_shapes(&program.shapes);
    let shapes = place_text(&mut module, &shapes_text);
    let texts = [(path, program.path.len()), (shapes, shapes_text.len())];
    for (id, name) in &program.exports {
        export(&mut module, program, *id as usize, name, texts)?;
    }
    pass_on(&mut module, free_name, "kv_lib_free", (1, false))?;
    pass_on(&mut module, last_error_name, "kv_lib_last_error", (0, true))?;
    let bytes = module.finish().emit().map_err(|e| e.to_string())?;

    let dir = link::build_folder("kavach-lib");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let object = dir.join(if cfg!(windows) {
        "library.obj"
    } else {
        "library.o"
    });
    std::fs::write(&object, bytes).map_err(|e| e.to_string())?;
    let mut libs: Vec<String> = program
        .functions
        .iter()
        .filter_map(|f| f.external.clone())
        .collect();
    libs.sort();
    libs.dedup();
    let mut names: Vec<&str> = program.exports.iter().map(|(_, n)| n.as_str()).collect();
    names.push(free_name);
    names.push(last_error_name);
    let linked = link::link_library(&object, out, &names, &libs);
    let _ = std::fs::remove_dir_all(&dir);
    linked
}

/// The exported C function `name` in front of program function `i`.
fn export(
    module: &mut ObjectModule,
    program: &Program,
    i: usize,
    name: &str,
    texts: [(DataId, usize); 2],
) -> Result<(), String> {
    let f = &program.functions[i];
    let params: Vec<_> = f.params.iter().map(|p| f.locals[*p as usize]).collect();
    // The Kavach function follows the C calling convention already. One
    // that can fail also returns its error; the C function returns the
    // error's number instead, and the value through one more parameter.
    let inner_sig = signature(module, &params, f.ret, f.fails);
    let inner = module
        .declare_function(&format!("kv_fn_{i}"), Linkage::Local, &inner_sig)
        .map_err(|e| e.to_string())?;
    let mut sig = signature(module, &params, f.ret, false);
    if f.fails {
        if f.ret != Type::Unit {
            sig.params.push(AbiParam::new(types::I64));
        }
        sig.returns = vec![AbiParam::new(types::I32)];
    }
    let outer = module
        .declare_function(name, Linkage::Export, &sig)
        .map_err(|e| e.to_string())?;
    let mut enter_sig = module.make_signature();
    enter_sig.params.extend([AbiParam::new(types::I64); 4]);
    let enter = module
        .declare_function("kv_lib_enter", Linkage::Import, &enter_sig)
        .map_err(|e| e.to_string())?;
    let leave_sig = module.make_signature();
    let leave = module
        .declare_function("kv_lib_leave", Linkage::Import, &leave_sig)
        .map_err(|e| e.to_string())?;

    let mut ctx = module.make_context();
    ctx.func.signature = sig;
    let mut fctx = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
    let block = b.create_block();
    b.append_block_params_for_function_params(block);
    b.switch_to_block(block);
    let mut args = b.block_params(block).to_vec();
    let result_at = (f.fails && f.ret != Type::Unit)
        .then(|| args.pop())
        .flatten();
    let mut setup = Vec::new();
    for (id, len) in texts {
        let gv = module.declare_data_in_func(id, b.func);
        setup.push(b.ins().symbol_value(types::I64, gv));
        setup.push(b.ins().iconst(types::I64, len as i64));
    }
    let enter = module.declare_func_in_func(enter, b.func);
    b.ins().call(enter, &setup);
    let is_text = |t: Type| matches!(t, Type::Ref(id) if program.shapes.get(id as usize) == Some(&Shape::Str));
    // C text in: a new Kavach text each, which the function owns.
    for (arg, t) in args.iter_mut().zip(&params) {
        if is_text(*t) {
            let text_in = runtime(module, "kv_lib_text_in", 2, true)?;
            let text_in = module.declare_func_in_func(text_in, b.func);
            let code = b.ins().iconst(types::I64, type_code(*t));
            let call = b.ins().call(text_in, &[*arg, code]);
            *arg = b.inst_results(call)[0];
        }
    }
    let inner = module.declare_func_in_func(inner, b.func);
    let call = b.ins().call(inner, &args);
    let mut results = b.inst_results(call).to_vec();
    let leave = module.declare_func_in_func(leave, b.func);
    let error = if f.fails { results.pop() } else { None };
    if let Some(error) = error {
        // Failed: the error's number, from the runtime, which keeps its
        // text for `last_error`.
        let (failed, done) = (b.create_block(), b.create_block());
        b.ins().brif(error, failed, &[], done, &[]);
        b.switch_to_block(failed);
        let to_code = runtime(module, "kv_lib_error", 1, true)?;
        let to_code = module.declare_func_in_func(to_code, b.func);
        let call = b.ins().call(to_code, &[error]);
        let code = b.inst_results(call)[0];
        let code = b.ins().ireduce(types::I32, code);
        b.ins().call(leave, &[]);
        b.ins().return_(&[code]);
        b.switch_to_block(done);
    }
    if is_text(f.ret) {
        let text_out = runtime(module, "kv_lib_text_out", 1, true)?;
        let text_out = module.declare_func_in_func(text_out, b.func);
        let call = b.ins().call(text_out, &results);
        results = b.inst_results(call).to_vec();
    }
    if let (Some(at), Some(value)) = (result_at, results.first().copied()) {
        // The value goes where `result` points, unless it is null; text
        // nobody takes is freed.
        let (store, skip, done) = (b.create_block(), b.create_block(), b.create_block());
        b.ins().brif(at, store, &[], skip, &[]);
        b.switch_to_block(store);
        b.ins().store(MemFlags::new(), value, at, 0);
        b.ins().jump(done, &[]);
        b.switch_to_block(skip);
        if is_text(f.ret) {
            let free = runtime(module, "kv_lib_free", 1, false)?;
            let free = module.declare_func_in_func(free, b.func);
            b.ins().call(free, &[value]);
        }
        b.ins().jump(done, &[]);
        b.switch_to_block(done);
    }
    b.ins().call(leave, &[]);
    if f.fails {
        let ok = b.ins().iconst(types::I32, 0);
        results = vec![ok];
    }
    b.ins().return_(&results);
    b.seal_all_blocks();
    b.finalize();
    module
        .define_function(outer, &mut ctx)
        .map_err(|e| format!("{name}: {e:?}"))
}

/// The runtime function `name` with `params` 64-bit parameters, giving a
/// 64-bit result or nothing.
fn runtime(
    module: &mut ObjectModule,
    name: &str,
    params: usize,
    gives: bool,
) -> Result<cranelift_module::FuncId, String> {
    let mut sig = module.make_signature();
    sig.params.extend(vec![AbiParam::new(types::I64); params]);
    if gives {
        sig.returns.push(AbiParam::new(types::I64));
    }
    module
        .declare_function(name, Linkage::Import, &sig)
        .map_err(|e| e.to_string())
}

/// The exported `name`, which calls runtime function `inner`, with
/// `shape`: how many 64-bit parameters, and whether it gives a result.
fn pass_on(
    module: &mut ObjectModule,
    name: &str,
    inner: &str,
    (params, gives): (usize, bool),
) -> Result<(), String> {
    let inner = runtime(module, inner, params, gives)?;
    let mut sig = module.make_signature();
    sig.params.extend(vec![AbiParam::new(types::I64); params]);
    if gives {
        sig.returns.push(AbiParam::new(types::I64));
    }
    let outer = module
        .declare_function(name, Linkage::Export, &sig)
        .map_err(|e| e.to_string())?;
    let mut ctx = module.make_context();
    ctx.func.signature = sig;
    let mut fctx = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
    let block = b.create_block();
    b.append_block_params_for_function_params(block);
    b.switch_to_block(block);
    let args = b.block_params(block).to_vec();
    let inner = module.declare_func_in_func(inner, b.func);
    let call = b.ins().call(inner, &args);
    let results = b.inst_results(call).to_vec();
    b.ins().return_(&results);
    b.seal_all_blocks();
    b.finalize();
    module
        .define_function(outer, &mut ctx)
        .map_err(|e| format!("{name}: {e:?}"))
}
