//! `kavach build --compress` and `--compress=max` (spec 10.5, decision
//! 0081): the built executable, packed behind the starter of
//! `runtime/unpack`.

use kavach_runtime_unpack::{Method, packed};
use std::path::Path;

/// Takes `--compress` or `--compress=max` out of `build`'s arguments
/// (they come before the file, as `--lib` does): the arguments left, and
/// the method asked for.
pub(crate) fn take_flag(mut args: Vec<String>) -> (Vec<String>, Option<Method>) {
    if args.first().map(String::as_str) != Some("build") {
        return (args, None);
    }
    let Some(at) = args
        .iter()
        .skip(1)
        .take_while(|a| a.starts_with("--"))
        .position(|a| a == "--compress" || a == "--compress=max")
    else {
        return (args, None);
    };
    let flag = args.remove(at + 1);
    let method = if flag == "--compress=max" {
        Method::Xz
    } else {
        Method::Deflate
    };
    (args, Some(method))
}

/// Packs the executable at `out` in place; what it says about the sizes.
pub(crate) fn pack(out: &Path, method: Method) -> Result<String, String> {
    let program = std::fs::read(out).map_err(|e| format!("can't read {}: {e}", out.display()))?;
    let file = packed(&program, method).map_err(|e| format!("can't pack: {e}"))?;
    std::fs::write(out, &file).map_err(|e| format!("can't write {}: {e}", out.display()))?;
    let kb = |n: usize| n.div_ceil(1024);
    Ok(format!(
        "packed: {} KB, {}% of {} KB",
        kb(file.len()),
        file.len() * 100 / program.len().max(1),
        kb(program.len())
    ))
}

/// What `kavach version` reads: a packed program's own bytes, unpacked.
pub(crate) fn readable(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    match kavach_runtime_unpack::trailer(&bytes) {
        Some(_) => kavach_runtime_unpack::unpack(&bytes),
        None => Ok(bytes),
    }
}
