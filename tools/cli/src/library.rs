//! `kavach build --lib` (spec 10.10, decision 0026): a library package
//! as a shared library with a C header, for other languages.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The shared library's file name for package `name`, as each system
/// names them.
fn file_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{name}.dylib")
    } else {
        format!("lib{name}.so")
    }
}

/// Builds the library whose own file is `path` into the folder `dir`
/// (the current folder without `-o`).
pub(crate) fn build_lib(path: &str, dir: Option<&str>) -> ExitCode {
    let compiled = match crate::load(path) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let stem = || {
        Path::new(path)
            .file_stem()
            .map_or("library".into(), |s| s.to_string_lossy().to_string())
    };
    let package = kavach_driver::Config::for_file(Path::new(path))
        .ok()
        .map(|c| c.package.name)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(stem);
    let lib = kavach_kir::library(&compiled, &package);
    for (name, why) in &lib.skipped {
        eprintln!("not exported: `{name}`: {why}");
    }
    if lib.exports.is_empty() {
        eprintln!(
            "error: {path} has no `pub fn` that C can call (spec 10.10): numbers, `Bool` and `Ptr` in and out, and no `!`"
        );
        return ExitCode::from(1);
    }
    let names: Vec<&str> = lib.exports.iter().map(|(k, _)| k.as_str()).collect();
    let mut program = match kavach_kir::lower_with(&compiled, &names) {
        Ok(p) => p,
        Err(u) => return crate::native::unsupported(&compiled, path, &u),
    };
    kavach_backend_wgsl::attach(&mut program);
    // Exported under their C names.
    for (export, (_, c_name)) in program.exports.iter_mut().zip(&lib.exports) {
        export.1 = c_name.clone();
    }
    let dir = dir.map_or_else(|| PathBuf::from("."), PathBuf::from);
    let _ = std::fs::create_dir_all(&dir);
    let out = dir.join(file_name(&package));
    let header = dir.join(format!("{package}.h"));
    if let Err(e) = std::fs::write(&header, &lib.header) {
        eprintln!("error: can't write {}: {e}", header.display());
        return ExitCode::from(3);
    }
    // LLVM-optimized when it can be (decision 0026), Cranelift otherwise.
    match kavach_backend_llvm::library::build_library(&program, &out, &lib.free, &lib.last_error) {
        Ok(()) => {
            eprintln!(
                "built {} and {}",
                kavach_driver::relative_path(&out),
                kavach_driver::relative_path(&header)
            );
            return ExitCode::SUCCESS;
        }
        Err(kavach_backend_llvm::NotBuilt::Unsupported(_)) => {}
        Err(kavach_backend_llvm::NotBuilt::Failed(e)) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    }
    match kavach_backend_cranelift::build_library(&program, &out, &lib.free, &lib.last_error) {
        Ok(()) => {
            eprintln!(
                "built {} and {}",
                kavach_driver::relative_path(&out),
                kavach_driver::relative_path(&header)
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(3)
        }
    }
}
