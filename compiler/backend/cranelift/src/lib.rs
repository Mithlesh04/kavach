//! Native code generation with Cranelift (decision 0005): KIR → machine
//! code, either compiled in memory and run (JIT) for `kavach run --native`,
//! or written to an executable for `kavach build` (decision 0016).
//!
//! The run-time library the code calls is `runtime/native`.

mod abi;
mod alloc;
mod calls;
mod codegen;
mod data;
mod errors;
mod flat;
mod inline;
mod jit_unwind;
mod library;
mod link;
pub use link::link_library;
mod loops;
mod object;
mod op_entries;
mod ops;
mod places;
mod pointers;
mod text_lists;
mod unwind;
mod winx64;

use abi::{cl_type, declare_runtime, runtime_functions, signature};
use codegen::FnGen;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::isa::OwnedTargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use kavach_kir::Program;
use kavach_runtime_native as runtime;
use std::collections::HashMap;

pub use library::build_library;
pub use link::{build_folder, link};
pub use object::build;

/// Compiles `program` to machine code and runs it. Returns the exit code:
/// 0, or 1 when `main` returns an error; a panic exits the process
/// directly with 101 (spec 5.8).
pub fn run(program: &Program) -> Result<i32, String> {
    run_with_args(program, &[])
}

/// Like [`run`], with the program's command-line arguments (spec 14.6).
pub fn run_with_args(program: &Program, args: &[String]) -> Result<i32, String> {
    runtime::rt_std::set_args(args.to_vec());
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(runtime::STACK_SIZE)
            .spawn_scoped(scope, || run_here(program))
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| "native code crashed".to_string())?
    })
}

fn run_here(program: &Program) -> Result<i32, String> {
    runtime::start(&program.path, &program.shapes);
    let isa = new_isa(false)?;
    let mut builder = JITBuilder::with_isa(isa, default_libcall_names());
    for (name, ptr, _, _) in runtime_functions() {
        builder.symbol(name, ptr);
    }
    builder.symbol(
        "kv_stack_limit",
        std::ptr::addr_of!(runtime::rt::kv_stack_limit) as *const u8,
    );
    builder.symbol(
        "kv_pool",
        std::ptr::addr_of!(runtime::pool::kv_pool) as *const u8,
    );
    builder.symbol(
        "kv_live",
        std::ptr::addr_of!(runtime::heap::kv_live) as *const u8,
    );
    builder.symbol(
        "kv_free_at_exit",
        std::ptr::addr_of!(runtime::rt::kv_free_at_exit) as *const u8,
    );
    let mut module = JITModule::new(builder);
    // Unwind tables, so a panic stops only its task (decision 0065).
    let mut unwinds = unwind::Unwinds::default();
    let entry = compile(&mut module, program, Some(&mut unwinds))?;
    let prepared = jit_unwind::prepare(&mut module, &unwinds)?;
    module.finalize_definitions().map_err(|e| e.to_string())?;
    jit_unwind::register(&module, module.isa(), unwinds, prepared);
    let code = module.get_finalized_function(entry);
    let main = &program.functions[program.entry as usize];
    Ok(runtime::run_entry(code, main.fails))
}

/// The machine this compiler runs on, as a code generation target.
pub(crate) fn new_isa(pic: bool) -> Result<OwnedTargetIsa, String> {
    let mut flags = settings::builder();
    let pic = if pic { "true" } else { "false" };
    for (key, value) in [
        ("use_colocated_libcalls", "false"),
        ("is_pic", pic),
        ("opt_level", "speed"),
    ] {
        flags.set(key, value).map_err(|e| e.to_string())?;
    }
    cranelift_native::builder()
        .map_err(|e| e.to_string())?
        .finish(settings::Flags::new(flags))
        .map_err(|e| e.to_string())
}

/// Compiles every function; returns the entry function's id. With
/// `unwinds`, each function's unwind description is kept for its object
/// file (`unwind.rs`).
pub(crate) fn compile(
    module: &mut dyn Module,
    program: &Program,
    mut unwinds: Option<&mut unwind::Unwinds>,
) -> Result<cranelift_module::FuncId, String> {
    let rt = declare_runtime(module);
    let mut funcs = Vec::new();
    for (i, f) in program.functions.iter().enumerate() {
        let params: Vec<_> = f.params.iter().map(|p| f.locals[*p as usize]).collect();
        // A C function (spec 10.7): the library's symbol of its name.
        let declared = if f.external.is_some() {
            let sig = crate::abi::c_signature(module, &params, f.ret);
            module.declare_function(&f.name, Linkage::Import, &sig)
        } else {
            let sig = signature(module, &params, f.ret, f.fails);
            module.declare_function(&format!("kv_fn_{i}"), Linkage::Local, &sig)
        };
        funcs.push(declared.map_err(|e| e.to_string())?);
    }
    let fails: Vec<bool> = program.functions.iter().map(|f| f.fails).collect();
    let closures = kavach_kir::borrows::closure_functions(&program.functions);
    let mut texts = HashMap::new();
    // The runtime's stack limit. Native code runs on one thread for now;
    // the task scheduler will make it per thread.
    let stack_limit = module
        .declare_data("kv_stack_limit", Linkage::Import, true, false)
        .map_err(|e| e.to_string())?;
    // The object pool and the live-object counter (`alloc.rs`).
    let pool = module
        .declare_data("kv_pool", Linkage::Import, true, false)
        .map_err(|e| e.to_string())?;
    let live = module
        .declare_data("kv_live", Linkage::Import, true, false)
        .map_err(|e| e.to_string())?;
    let free_at_exit = module
        .declare_data("kv_free_at_exit", Linkage::Import, true, false)
        .map_err(|e| e.to_string())?;
    let mut fctx = FunctionBuilderContext::new();
    for (i, f) in program.functions.iter().enumerate() {
        if f.external.is_some() {
            continue;
        }
        let mut ctx = module.make_context();
        let params: Vec<_> = f.params.iter().map(|p| f.locals[*p as usize]).collect();
        ctx.func.signature = signature(module, &params, f.ret, f.fails);
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        // Unit locals get an I8 slot that is never read.
        let vars = f
            .locals
            .iter()
            .map(|t| b.declare_var(cl_type(*t).unwrap_or(cranelift_codegen::ir::types::I8)))
            .collect();
        let mut g = FnGen {
            b,
            module,
            texts: &mut texts,
            stack_limit,
            pool,
            live,
            free_at_exit: (i == program.entry as usize).then_some(free_at_exit),
            funcs: &funcs,
            fails: &fails,
            closures: &closures,
            this_fails: f.fails,
            ret: f.ret,
            rt: &rt,
            shapes: &program.shapes,
            vars,
            types: f.locals.clone(),
            loops: Vec::new(),
            borrowed: vec![false; f.locals.len()],
            steady: vec![None; f.locals.len()],
            unchecked: Vec::new(),
            text_views: vec![false; f.locals.len()],
        };
        // A closure's function borrows its closure and the captures it
        // never changes (`kavach_kir::borrows`).
        if closures[i] {
            for l in kavach_kir::borrows::borrowing_locals(f) {
                g.borrowed[l as usize] = true;
            }
        }
        // Heap locals start as null, so storing into them can release the old value.
        for (i, t) in f.locals.iter().enumerate() {
            if t.is_ref() && !f.params.contains(&(i as u32)) {
                let null = g.int(cranelift_codegen::ir::types::I64, 0);
                g.b.def_var(g.vars[i], null);
            }
        }
        let args = g.b.block_params(entry).to_vec();
        for (p, v) in f.params.iter().zip(args) {
            g.b.def_var(g.vars[*p as usize], v);
        }
        let value = g.block(&f.body);
        let value = if cl_type(f.ret).is_some() {
            value.or_else(|| g.zero(f.ret))
        } else {
            None
        };
        g.leave_and_return(value, None);
        // The block left open after the final return is unreachable.
        g.b.ins()
            .trap(cranelift_codegen::ir::TrapCode::user(1).expect("a trap code"));
        g.b.seal_all_blocks();
        g.b.finalize();
        // For work on the compiler: `KAVACH_CLIF=1` prints each function's
        // Cranelift IR before optimization.
        if std::env::var_os("KAVACH_CLIF").is_some() {
            eprintln!("; {}\n{}", f.name, ctx.func.display());
        }
        module
            .define_function(funcs[i], &mut ctx)
            .map_err(|e| format!("{}: {e:?}", f.name))?;
        if let (Some(u), Some(code)) = (unwinds.as_deref_mut(), ctx.compiled_code())
            && let Some(frame) = unwind::Frame::of(code, module.isa())
        {
            u.0.push((funcs[i], frame, code.code_info().total_size));
        }
        module.clear_context(&mut ctx);
    }
    Ok(funcs[program.entry as usize])
}
