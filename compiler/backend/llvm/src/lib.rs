//! The LLVM backend (decision 0018): KIR → LLVM IR text → `opt -O3` →
//! `llc` → object file → linked with the runtime library like the
//! Cranelift backend's output (decision 0016).
//!
//! It supports part of KIR so far (`check.rs`); `build` says when a
//! program needs something else, and the caller uses Cranelift instead.
//! `library.rs` builds shared libraries the same way (decision 0026).

mod branches;
mod calls;
mod check;
mod data;
mod expr;
mod func;
mod heap;
pub mod library;
mod lists;
mod module;
mod ops;
mod pointers;
mod records;
mod stmt;
mod text;

use kavach_kir::Program;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Why `build` didn't make an executable.
pub enum NotBuilt {
    /// The program uses something this backend doesn't support yet, or
    /// LLVM's tools aren't installed: build it another way.
    Unsupported(String),
    /// Building failed.
    Failed(String),
}

/// Compiles `program` and writes the executable `out`.
pub fn build(program: &Program, out: &Path) -> Result<(), NotBuilt> {
    if let Some(what) = check::unsupported(program) {
        // `KAVACH_LLVM_IR=1` (below) also says why LLVM isn't used.
        if std::env::var_os("KAVACH_LLVM_IR").is_some() {
            eprintln!("LLVM backend: not for {what}");
        }
        return Err(NotBuilt::Unsupported(what));
    }
    let bin = tools().ok_or_else(|| {
        NotBuilt::Unsupported("LLVM's tools (`rustup component add llvm-tools`)".into())
    })?;
    let ir = emit(program);
    // For work on the compiler: `KAVACH_LLVM_IR=1` prints the IR before
    // LLVM optimizes it (like `KAVACH_CLIF` for Cranelift).
    if std::env::var_os("KAVACH_LLVM_IR").is_some() {
        eprintln!("{ir}");
    }
    let dir = kavach_backend_cranelift::build_folder("kavach-llvm");
    let built = compile(&bin, &dir, &ir, out, &c_libraries(program));
    let _ = std::fs::remove_dir_all(&dir);
    built.map_err(NotBuilt::Failed)
}

/// The whole program as LLVM IR text.
pub fn emit(program: &Program) -> String {
    let (m, body) = emit_body(program);
    m.finish(program, &body, &func::fn_name(program.entry as usize))
}

/// Every function as LLVM IR, and the module collecting declarations.
pub(crate) fn emit_body(program: &Program) -> (module::Module, String) {
    let mut m = module::Module::default();
    m.fails = program.functions.iter().map(|f| f.fails).collect();
    m.closures = kavach_kir::borrows::closure_functions(&program.functions);
    m.c_names = program
        .functions
        .iter()
        .map(|f| f.external.as_ref().map(|_| f.name.clone()))
        .collect();
    let mut body = String::new();
    for (i, f) in program.functions.iter().enumerate() {
        if f.external.is_some() {
            // A C function (spec 10.7): declared as C has it.
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| c_type(f.locals[*p as usize], false))
                .collect();
            let params: Vec<&str> = params.iter().map(String::as_str).collect();
            m.declare(&f.name, &c_type(f.ret, true), &params, false);
            continue;
        }
        let is_main = i == program.entry as usize;
        let quiet = i >= program.quiet_from as usize;
        let closure = m.closures[i];
        let emit = func::FnEmit::new(&mut m, f, &program.shapes, is_main, quiet).borrowing(closure);
        body.push_str(&emit.emit(&func::fn_name(i)));
    }
    (m, body)
}

/// A C type in LLVM IR: small integers and booleans widened by the
/// caller, signed or not as their type is (`signext i8`; for a result,
/// the attribute comes first).
fn c_type(t: kavach_kir::Type, result: bool) -> String {
    use kavach_kir::Type;
    let ty = module::llvm_type(t);
    let ext = match t {
        Type::I8 | Type::I16 | Type::I32 => "signext",
        Type::U8 | Type::U16 | Type::U32 | Type::Bool => "zeroext",
        _ => return ty.to_string(),
    };
    if result {
        format!("{ext} {ty}")
    } else {
        format!("{ty} {ext}")
    }
}

/// The C libraries the program's C functions come from (spec 10.7).
pub fn c_libraries(program: &Program) -> Vec<String> {
    let mut libs: Vec<String> = program
        .functions
        .iter()
        .filter_map(|f| f.external.clone())
        .collect();
    libs.sort();
    libs.dedup();
    libs
}

/// Writes `ir` in `dir`, optimizes and compiles it, and links `out`.
fn compile(bin: &Path, dir: &Path, ir: &str, out: &Path, libs: &[String]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let (ll, bc) = (dir.join("program.ll"), dir.join("program.bc"));
    let object = dir.join(if cfg!(windows) {
        "program.obj"
    } else {
        "program.o"
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
        // Executables on Linux and macOS are position-independent.
        llc.arg("-relocation-model=pic");
    }
    run(llc.arg(&bc).arg("-o").arg(&object))?;
    kavach_backend_cranelift::link(&object, out, libs)
}

pub(crate) fn run(cmd: &mut Command) -> Result<(), String> {
    let output = cmd.output().map_err(|e| format!("{cmd:?}: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{:?} failed: {}",
        cmd.get_program(),
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// The folder with `opt` and `llc`: `KAVACH_LLVM_BIN`, an installed
/// Kavach's own `llvm/` folder, or rustup's `llvm-tools` for the Rust
/// toolchain in use.
pub(crate) fn tools() -> Option<PathBuf> {
    let has_tools = |dir: &Path| dir.join(exe("opt")).is_file() && dir.join(exe("llc")).is_file();
    if let Some(dir) = std::env::var_os("KAVACH_LLVM_BIN") {
        return Some(PathBuf::from(dir)).filter(|d| has_tools(d));
    }
    // An installed Kavach carries the tools itself (decision 0080):
    // `<root>/bin/kavach` with `<root>/llvm/`, or `llvm/` next to `kavach`.
    if let Ok(kavach) = std::env::current_exe()
        && let Some(bin) = kavach.parent()
    {
        let roots = [Some(bin), bin.parent()];
        if let Some(dir) = roots
            .into_iter()
            .flatten()
            .map(|root| root.join("llvm"))
            .find(|d| has_tools(d))
        {
            return Some(dir);
        }
    }
    // Asking `rustc` (through rustup) takes about a second, longer than
    // the whole build of a small program: the answer is kept in a file.
    let cache = std::env::temp_dir().join("kavach-llvm-bin.txt");
    if let Ok(dir) = std::fs::read_to_string(&cache) {
        let dir = PathBuf::from(dir.trim());
        if has_tools(&dir) {
            return Some(dir);
        }
    }
    let found = tools_from_rustc()?;
    let _ = std::fs::write(&cache, found.to_string_lossy().as_bytes());
    Some(found)
}

/// The tools of the toolchain `rustc` belongs to.
fn tools_from_rustc() -> Option<PathBuf> {
    let has_tools = |dir: &Path| dir.join(exe("opt")).is_file() && dir.join(exe("llc")).is_file();
    let sysroot = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .ok()?;
    let sysroot = String::from_utf8(sysroot.stdout).ok()?;
    let host = if cfg!(windows) {
        "x86_64-pc-windows-msvc"
    } else {
        "x86_64-unknown-linux-gnu"
    };
    let dir = Path::new(sysroot.trim())
        .join("lib/rustlib")
        .join(host)
        .join("bin");
    has_tools(&dir).then_some(dir)
}

fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}
