//! A shared library from LLVM-optimized code (decision 0026): the same
//! export shims the Cranelift backend builds — `kv_lib_enter_ahead`, text in
//! and out, errors as return codes, `kv_lib_leave` — written as LLVM IR
//! around the optimized functions, then linked as the Cranelift path
//! links its object.

use crate::module::{Module, llvm_type, ret_type};
use crate::{NotBuilt, c_libraries, run, tools};
use kavach_kir::{Program, Shape, Type, encode_shapes, type_code};
use std::fmt::Write;
use std::path::Path;
use std::process::Command;

/// Compiles `program` and writes the shared library `out`, as
/// `kavach_backend_cranelift::build_library` does, but through LLVM.
pub fn build_library(
    program: &Program,
    out: &Path,
    free_name: &str,
    last_error_name: &str,
) -> Result<(), NotBuilt> {
    if let Some(what) = crate::check::unsupported(program) {
        return Err(NotBuilt::Unsupported(what));
    }
    let bin = tools().ok_or_else(|| {
        NotBuilt::Unsupported("LLVM's tools (`rustup component add llvm-tools`)".into())
    })?;
    let (mut m, body) = crate::emit_body(program);
    let (path, path_len) = m.text(&program.path);
    let shapes_text = encode_shapes(&program.shapes);
    let (shapes, shapes_len) = m.text(&shapes_text);
    // Calls compute at the same time, each thread with its own stack
    // limit (`module.rs`), until their first effect (decision 0035).
    m.declare(
        "kv_lib_enter_ahead",
        "void",
        &["i64", "i64", "i64", "i64", "ptr"],
        false,
    );
    m.declare("kv_lib_leave", "void", &[], false);
    let mut shims = String::new();
    let enter = format!(
        "call void @kv_lib_enter_ahead(i64 {path}, i64 {path_len}, i64 {shapes}, i64 {shapes_len}, ptr @kv_limit_slot)"
    );
    for (id, name) in &program.exports {
        shims.push_str(&shim(&mut m, program, *id as usize, name, &enter));
    }
    m.declare("kv_lib_free", "void", &["i64"], false);
    m.declare("kv_lib_last_error", "i64", &[], false);
    let _ = writeln!(
        shims,
        "define dso_local void @{free_name}(i64 %p) {{\n  call void @kv_lib_free(i64 %p)\n  ret void\n}}\n"
    );
    let _ = writeln!(
        shims,
        "define dso_local i64 @{last_error_name}() {{\n  %r = call i64 @kv_lib_last_error()\n  ret i64 %r\n}}\n"
    );
    let ir = m.finish_library(&format!("{body}{shims}"));
    if std::env::var_os("KAVACH_LLVM_IR").is_some() {
        eprintln!("{ir}");
    }
    let dir = kavach_backend_cranelift::build_folder("kavach-llvm-lib");
    let built = compile(&bin, &dir, &ir, out, program, free_name, last_error_name);
    let _ = std::fs::remove_dir_all(&dir);
    built.map_err(NotBuilt::Failed)
}

/// The exported C function `name` in front of program function `i`:
/// the lock and runtime setup, text conversions, and errors as codes.
fn shim(m: &mut Module, program: &Program, i: usize, name: &str, enter: &str) -> String {
    let f = &program.functions[i];
    let is_text = |t: Type| matches!(t, Type::Ref(id) if program.shapes.get(id as usize) == Some(&Shape::Str));
    let params: Vec<Type> = f.params.iter().map(|p| f.locals[*p as usize]).collect();
    let mut args: Vec<String> = Vec::new();
    let mut sig: Vec<String> = Vec::new();
    for (k, t) in params.iter().enumerate() {
        sig.push(format!("{} %a{k}", llvm_type(*t)));
        args.push(format!("%a{k}"));
    }
    let result_at = f.fails && f.ret != Type::Unit;
    if result_at {
        sig.push("i64 %at".to_string());
    }
    let outer_ret = if f.fails {
        "i32".to_string()
    } else {
        llvm_type(f.ret).to_string()
    };
    let mut out = String::new();
    let _ = writeln!(
        out,
        "define dso_local {outer_ret} @{name}({}) {{",
        sig.join(", ")
    );
    let _ = writeln!(out, "  {enter}");
    // C text in: a new Kavach text each, which the function owns.
    m.declare("kv_lib_text_in", "i64", &["i64", "i64"], false);
    for (k, t) in params.iter().enumerate() {
        if is_text(*t) {
            let _ = writeln!(
                out,
                "  %t{k} = call i64 @kv_lib_text_in(i64 %a{k}, i64 {})",
                type_code(*t)
            );
            args[k] = format!("%t{k}");
        }
    }
    let inner_ret = ret_type(f.ret, f.fails);
    let called: Vec<String> = params
        .iter()
        .zip(&args)
        .map(|(t, a)| format!("{} {a}", llvm_type(*t)))
        .collect();
    let call = format!("call {inner_ret} @kv_fn_{i}({})", called.join(", "));
    let mut value = String::new();
    if f.fails {
        if f.ret == Type::Unit {
            let _ = writeln!(out, "  %err = {call}");
        } else {
            let _ = writeln!(out, "  %pair = {call}");
            let _ = writeln!(out, "  %val = extractvalue {inner_ret} %pair, 0");
            let _ = writeln!(out, "  %err = extractvalue {inner_ret} %pair, 1");
            value = "%val".to_string();
        }
        m.declare("kv_lib_error", "i64", &["i64"], false);
        let _ = writeln!(out, "  %bad = icmp ne i64 %err, 0");
        let _ = writeln!(out, "  br i1 %bad, label %failed, label %ok");
        let _ = writeln!(out, "failed:");
        let _ = writeln!(out, "  %code = call i64 @kv_lib_error(i64 %err)");
        let _ = writeln!(out, "  %code32 = trunc i64 %code to i32");
        let _ = writeln!(out, "  call void @kv_lib_leave()");
        let _ = writeln!(out, "  ret i32 %code32");
        let _ = writeln!(out, "ok:");
    } else if f.ret == Type::Unit {
        let _ = writeln!(out, "  {call}");
    } else {
        let _ = writeln!(out, "  %val = {call}");
        value = "%val".to_string();
    }
    if is_text(f.ret) {
        m.declare("kv_lib_text_out", "i64", &["i64"], false);
        let _ = writeln!(out, "  %text = call i64 @kv_lib_text_out(i64 {value})");
        value = "%text".to_string();
    }
    if result_at {
        // The value goes where `at` points, unless it is null; text
        // nobody takes is freed.
        m.declare("kv_lib_free", "void", &["i64"], false);
        let _ = writeln!(out, "  %want = icmp ne i64 %at, 0");
        let _ = writeln!(out, "  br i1 %want, label %store, label %skip");
        let _ = writeln!(out, "store:");
        let _ = writeln!(out, "  %slot = inttoptr i64 %at to ptr");
        let t = llvm_type(f.ret);
        let _ = writeln!(out, "  store {t} {value}, ptr %slot");
        let _ = writeln!(out, "  br label %done");
        let _ = writeln!(out, "skip:");
        if is_text(f.ret) {
            let _ = writeln!(out, "  call void @kv_lib_free(i64 {value})");
        }
        let _ = writeln!(out, "  br label %done");
        let _ = writeln!(out, "done:");
    }
    let _ = writeln!(out, "  call void @kv_lib_leave()");
    if f.fails {
        let _ = writeln!(out, "  ret i32 0");
    } else if f.ret == Type::Unit {
        let _ = writeln!(out, "  ret void");
    } else {
        let _ = writeln!(out, "  ret {} {value}", llvm_type(f.ret));
    }
    let _ = writeln!(out, "}}\n");
    out
}

/// Optimizes, compiles and links the shared library.
#[allow(clippy::too_many_arguments)]
fn compile(
    bin: &Path,
    dir: &Path,
    ir: &str,
    out: &Path,
    program: &Program,
    free_name: &str,
    last_error_name: &str,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let (ll, bc) = (dir.join("library.ll"), dir.join("library.bc"));
    let object = dir.join(if cfg!(windows) {
        "library.obj"
    } else {
        "library.o"
    });
    std::fs::write(&ll, ir).map_err(|e| e.to_string())?;
    run(Command::new(bin.join("opt"))
        .arg("-O3")
        .arg(&ll)
        .arg("-o")
        .arg(&bc))?;
    let mut llc = Command::new(bin.join("llc"));
    llc.args(["-O3", "-filetype=obj"]);
    if !cfg!(windows) {
        // A shared library on Linux and macOS is position-independent.
        llc.arg("-relocation-model=pic");
    }
    run(llc.arg(&bc).arg("-o").arg(&object))?;
    let mut names: Vec<&str> = program.exports.iter().map(|(_, n)| n.as_str()).collect();
    names.push(free_name);
    names.push(last_error_name);
    kavach_backend_cranelift::link_library(&object, out, &names, &c_libraries(program))
}
