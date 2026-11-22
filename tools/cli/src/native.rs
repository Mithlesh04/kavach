// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

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

/// Whether `kavach build --tracy` was asked for.
static TRACY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `args` without `--tracy`. With it, `kavach run` starts Tracy's client
/// now, in this process, and `kavach build` makes a program that does.
pub(crate) fn take_tracy(mut args: Vec<String>) -> Vec<String> {
    let Some(at) = args.iter().position(|a| a == "--tracy") else {
        return args;
    };
    args.remove(at);
    match args.first().map(String::as_str) {
        Some("run") => kavach_runtime_tracy::start(),
        _ => TRACY.store(true, std::sync::atomic::Ordering::Relaxed),
    }
    args
}

/// `kavach build`: an executable file, named after the source file and
/// written to the current folder unless `-o` says otherwise (spec 10.9).
pub(crate) fn build(
    path: &str,
    out: Option<&str>,
    compress: Option<kavach_runtime_unpack::Method>,
) -> ExitCode {
    // What the program will say built it (spec 10.5).
    let tracy = TRACY.load(std::sync::atomic::Ordering::Relaxed);
    let mut options: Vec<String> = Vec::new();
    if tracy {
        options.push("--tracy".into());
    }
    match compress {
        Some(kavach_runtime_unpack::Method::Deflate) => options.push("--compress".into()),
        Some(kavach_runtime_unpack::Method::Xz) => options.push("--compress=max".into()),
        None => {}
    }
    if std::env::var("KAVACH_BACKEND").as_deref() == Ok("cranelift") {
        options.push("KAVACH_BACKEND=cranelift".into());
    }
    kavach_driver::set_build_options(&options);
    let mut program = match lowered(path) {
        Ok(p) => p,
        Err(code) => return code,
    };
    program.tracy = tracy;
    let out = match out {
        Some(o) => std::path::PathBuf::from(o),
        None => {
            let stem = std::path::Path::new(path)
                .file_stem()
                .map_or("program".into(), |s| s.to_string_lossy().to_string());
            std::path::PathBuf::from(format!("{stem}{}", std::env::consts::EXE_SUFFIX))
        }
    };
    // The Windows icon `[build]` names, checked on every system (10.5).
    let icon = kavach_driver::program_icon(std::path::Path::new(path)).and_then(|icon| {
        if let Some(png) = &icon {
            kavach_backend_cranelift::resources::check_icon(png)?;
        }
        Ok(icon)
    });
    let icon = match icon {
        Ok(icon) => icon,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let exe = kavach_backend_cranelift::Exe {
        icon: icon.as_deref(),
        // A program with no `kavach.toml` is a console program.
        console: match kavach_driver::Config::for_file(std::path::Path::new(path)) {
            Ok(config) => config.build.console,
            Err(_) => true,
        },
    };
    let built = build_with_best_backend(&program, &out, exe).and_then(|()| env::write_beside(&out));
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
    exe: kavach_backend_cranelift::Exe,
) -> Result<(), String> {
    if std::env::var("KAVACH_BACKEND").as_deref() != Ok("cranelift") {
        match kavach_backend_llvm::build(program, out, exe) {
            Ok(()) => return Ok(()),
            Err(kavach_backend_llvm::NotBuilt::Failed(e)) => return Err(e),
            Err(kavach_backend_llvm::NotBuilt::Unsupported(_)) => {}
        }
    }
    kavach_backend_cranelift::build(program, out, exe)
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

/// Gives the interpreter the shaders of the program's pipelines (spec
/// 15.3), which only the WGSL backend can write. Only a program that
/// makes one pays for lowering it.
pub(crate) fn give_shaders(c: &kavach_driver::Compiled) {
    if !c.src.contains("gpu.pipeline") {
        return;
    }
    let Ok(program) = kavach_kir::lower(c) else {
        return;
    };
    let mut shaders = std::collections::HashMap::new();
    for (key, text) in kavach_backend_wgsl::pipeline_sources(&program) {
        let defs: Vec<usize> = key.split(' ').filter_map(|d| d.parse().ok()).collect();
        if let [v, f] = defs[..] {
            shaders.insert((v, f, kavach_backend_wgsl::deep(&key)), text);
        }
    }
    kavach_interp::give_pipelines(shaders);
}
