//! Native code (spec 10.5, 10.9): `kavach build` makes an executable
//! with LLVM when it can, Cranelift otherwise, and `kavach run --native`
//! runs the program in memory.

use crate::{env, load, pack};
use std::process::ExitCode;

/// A checked file lowered to KIR, or the exit code after reporting why not.
pub(crate) fn lowered(path: &str) -> Result<kavach_kir::Program, ExitCode> {
    let compiled = load(path)?;
    let mut program = kavach_kir::lower(&compiled).map_err(|u| unsupported(&compiled, path, &u))?;
    // The kernels of `map`s that may run on the GPU (spec 15.2).
    kavach_backend_wgsl::attach(&mut program);
    Ok(program)
}

/// Reports what the native compiler doesn't support yet, where.
pub(crate) fn unsupported(
    compiled: &kavach_driver::Compiled,
    path: &str,
    u: &kavach_kir::Unsupported,
) -> ExitCode {
    // `u.line` counts lines of all the program's files together.
    let file = compiled
        .files
        .iter()
        .rev()
        .find(|f| f.first_line <= u.line as usize)
        .map_or((path, u.line as usize), |f| {
            (f.path.as_str(), u.line as usize - f.first_line + 1)
        });
    eprintln!(
        "error: the native compiler doesn't support {} yet
 --> {}:{}
help: run it with `kavach run {path}` (the interpreter) for now",
        u.feature, file.0, file.1
    );
    ExitCode::from(3)
}

/// `kavach build`: an executable file, named after the source file and
/// written to the current folder unless `-o` says otherwise (spec 10.9).
pub(crate) fn build(
    path: &str,
    out: Option<&str>,
    compress: Option<kavach_runtime_unpack::Method>,
) -> ExitCode {
    let program = match lowered(path) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let out = match out {
        Some(o) => std::path::PathBuf::from(o),
        None => {
            let stem = std::path::Path::new(path)
                .file_stem()
                .map_or("program".into(), |s| s.to_string_lossy().to_string());
            std::path::PathBuf::from(format!("{stem}{}", std::env::consts::EXE_SUFFIX))
        }
    };
    let built = build_with_best_backend(&program, &out).and_then(|()| env::write_beside(&out));
    let packed = built.and_then(|()| compress.map(|m| pack::pack(&out, m)).transpose());
    match packed {
        Ok(sizes) => {
            eprintln!("built {}", kavach_driver::relative_path(&out));
            if let Some(sizes) = sizes {
                eprintln!("{sizes}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(3)
        }
    }
}

/// Builds with LLVM when it can (optimized code, decision 0018), otherwise
/// with Cranelift. `KAVACH_BACKEND=cranelift` always uses Cranelift.
pub(crate) fn build_with_best_backend(
    program: &kavach_kir::Program,
    out: &std::path::Path,
) -> Result<(), String> {
    if std::env::var("KAVACH_BACKEND").as_deref() != Ok("cranelift") {
        match kavach_backend_llvm::build(program, out) {
            Ok(()) => return Ok(()),
            Err(kavach_backend_llvm::NotBuilt::Failed(e)) => return Err(e),
            Err(kavach_backend_llvm::NotBuilt::Unsupported(_)) => {}
        }
    }
    kavach_backend_cranelift::build(program, out)
}

/// `kavach run --native`: KIR → Cranelift → machine code, run in memory.
pub(crate) fn native(path: &str, args: &[&str]) -> ExitCode {
    let program = match lowered(path) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    match kavach_backend_cranelift::run_with_args(&program, &args) {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(e) => {
            eprintln!("error: native code generation failed: {e}");
            ExitCode::from(3)
        }
    }
}
