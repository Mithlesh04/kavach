// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The functions running at a panic, for crash reports (spec 14.30,
//! decision 0118). The stack is walked with the system's own unwinder, and
//! each frame's function is found from its exact start, which the unwind
//! tables give. A frame is Kavach's when that start is in the table a
//! Cranelift build or `run --native` gives (`set_functions`), or, for an
//! LLVM build, when the start is just after LLVM `prefix` data of
//! `MAGIC` and the function's number: that takes no function's address,
//! so the optimizer is free as before. Nothing runs until a panic.

use std::sync::OnceLock;

/// What LLVM writes before each function's entry, with its number.
pub const MAGIC: u32 = 0x4356_414B;

/// Every function by number: its name and the program line it starts on;
/// and, when they are known, function starts and their numbers.
struct Table {
    names: Vec<(String, i32)>,
    starts: Vec<(usize, usize)>,
}

static TABLE: OnceLock<Table> = OnceLock::new();

/// Sets the functions: `names` has a line `name<TAB>line` for each, by
/// number; `starts` the address of each, or none (LLVM's `prefix` data).
pub fn set_functions(names: &str, starts: &[usize]) {
    let names = names
        .lines()
        .map(|l| match l.rsplit_once('\t') {
            Some((name, line)) => (name.to_string(), line.parse().unwrap_or(0)),
            None => (l.to_string(), 0),
        })
        .collect();
    let mut starts: Vec<(usize, usize)> = starts.iter().copied().zip(0..).collect();
    starts.sort_unstable();
    let _ = TABLE.set(Table { names, starts });
}

/// `set_functions` for generated code: `starts` is null or `n` addresses.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
#[expect(clippy::not_unsafe_ptr_arg_deref, reason = "generated code's ABI")]
pub extern "C-unwind" fn kv_set_functions(
    names: *const u8,
    names_len: usize,
    starts: *const usize,
    n: usize,
) {
    let starts: Vec<usize> = if starts.is_null() {
        Vec::new()
    } else {
        // SAFETY: generated code passes its table of `n` addresses, which
        // lives as long as the program; read unaligned, as data may be.
        (0..n)
            .map(|i| unsafe { starts.add(i).read_unaligned() })
            .collect()
    };
    set_functions(crate::rt::text(names, names_len), &starts);
}

/// The Kavach functions running now, innermost first, each as `name
/// (file:line)` with where it is defined.
pub fn running() -> Vec<String> {
    let Some(table) = TABLE.get() else {
        return Vec::new();
    };
    let mine = image_base(kv_set_functions as *const () as usize);
    let mut out = Vec::new();
    for ip in return_addresses() {
        // A return address is just after its call: one byte back is in
        // the calling function, even when the call ends it.
        let Some((start, base)) = function_start(ip.saturating_sub(1)) else {
            continue;
        };
        let number = match table.starts.binary_search_by_key(&start, |s| s.0) {
            Ok(i) => Some(table.starts[i].1),
            // Only this executable's functions have prefix data to read.
            Err(_) if table.starts.is_empty() && Some(base) == mine => prefix_number(start),
            Err(_) => None,
        };
        if let Some((name, line)) = number.and_then(|n| table.names.get(n)) {
            let (path, line) = crate::rt::location(*line);
            out.push(format!("{name} ({path}:{line})"));
        }
    }
    out
}

/// The number in the `prefix` data before the function at `start`, if
/// it has Kavach's.
#[allow(unsafe_code)]
fn prefix_number(start: usize) -> Option<usize> {
    // SAFETY: `start` is a function's start in this executable's code;
    // the eight bytes before it are in the same image, which is mapped
    // whole: its code, or the headers before the first function.
    let (magic, number) = unsafe {
        let p = (start - 8) as *const u32;
        (p.read_unaligned(), p.add(1).read_unaligned())
    };
    (magic == MAGIC).then_some(number as usize)
}

/// The return addresses on this thread's stack, innermost first: walked
/// with the unwind tables, the program's own and those the JIT registered
/// (`unwind_tables.rs`), as an exception would unwind.
#[cfg(all(windows, target_arch = "x86_64"))]
#[allow(unsafe_code)]
fn return_addresses() -> Vec<usize> {
    use windows_sys::Win32::System::Diagnostics::Debug::{
        CONTEXT, RtlCaptureContext, RtlLookupFunctionEntry, RtlVirtualUnwind,
    };
    // SAFETY: `CONTEXT` is plain data, all zeros a valid value.
    let mut ctx: CONTEXT = unsafe { std::mem::zeroed() };
    // SAFETY: fills `ctx` with this point's registers.
    unsafe { RtlCaptureContext(&mut ctx) };
    let mut out = Vec::new();
    while ctx.Rip != 0 && out.len() < 128 {
        out.push(ctx.Rip as usize);
        let mut base = 0u64;
        // SAFETY: looks the address up in the unwind tables; reads only.
        let entry = unsafe { RtlLookupFunctionEntry(ctx.Rip, &mut base, std::ptr::null_mut()) };
        if entry.is_null() {
            // A leaf function: its return address is on top of the stack.
            // SAFETY: `Rsp` is this thread's stack, which holds it.
            ctx.Rip = unsafe { (ctx.Rsp as *const u64).read() };
            ctx.Rsp += 8;
            continue;
        }
        let (mut data, mut frame) = (std::ptr::null_mut(), 0u64);
        // SAFETY: undoes one frame of `ctx` as its unwind codes say; the
        // stack it reads is this thread's, below the current frame.
        unsafe {
            RtlVirtualUnwind(
                0,
                base,
                ctx.Rip,
                entry,
                &mut ctx,
                &mut data,
                &mut frame,
                std::ptr::null_mut(),
            )
        };
    }
    out
}

#[cfg(all(windows, not(target_arch = "x86_64")))]
fn return_addresses() -> Vec<usize> {
    Vec::new()
}

/// The base of the image holding code address `pc`: this executable's,
/// for the runtime's own functions.
#[cfg(windows)]
fn image_base(pc: usize) -> Option<usize> {
    function_start(pc).map(|(_, base)| base)
}

/// The start of the function holding `pc`, and its image's base.
#[cfg(windows)]
#[allow(unsafe_code)]
fn function_start(pc: usize) -> Option<(usize, usize)> {
    let mut base = 0u64;
    // SAFETY: looks `pc` up in the loaded images' unwind tables; reads only.
    let entry = unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::RtlLookupFunctionEntry(
            pc as u64,
            &mut base,
            std::ptr::null_mut(),
        )
    };
    if entry.is_null() {
        return None;
    }
    // SAFETY: a non-null entry points into an image's function table.
    let begin = unsafe { (*entry).BeginAddress };
    Some((base as usize + begin as usize, base as usize))
}

#[cfg(all(unix, any(target_env = "gnu", target_os = "macos")))]
unsafe extern "C" {
    /// The unwinder's: the start of the function holding `pc`, from its
    /// unwind tables.
    fn _Unwind_FindEnclosingFunction(pc: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

#[cfg(all(unix, any(target_env = "gnu", target_os = "macos")))]
#[allow(unsafe_code)]
fn return_addresses() -> Vec<usize> {
    let mut frames = [std::ptr::null_mut(); 128];
    // SAFETY: `frames` holds the 128 entries the count says.
    let n = unsafe { libc::backtrace(frames.as_mut_ptr(), 128) };
    frames[..n.max(0) as usize]
        .iter()
        .map(|p| *p as usize)
        .collect()
}

#[cfg(all(unix, any(target_env = "gnu", target_os = "macos")))]
#[allow(unsafe_code)]
fn function_start(pc: usize) -> Option<(usize, usize)> {
    // SAFETY: looks `pc` up in the unwind tables; reads only.
    let start = unsafe { _Unwind_FindEnclosingFunction(pc as *mut _) } as usize;
    (start != 0).then(|| (start, image_base(start).unwrap_or(0)))
}

/// The base of the image holding code address `pc`, from the dynamic
/// loader (the unwinder doesn't find every function's own start).
#[cfg(all(unix, any(target_env = "gnu", target_os = "macos")))]
#[allow(unsafe_code)]
fn image_base(pc: usize) -> Option<usize> {
    // SAFETY: `Dl_info` is plain data, all zeros a valid value.
    let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
    // SAFETY: `dladdr` fills `info` for an address in a loaded image.
    let found = unsafe { libc::dladdr(pc as *const _, &mut info) };
    (found != 0).then_some(info.dli_fbase as usize)
}

#[cfg(not(any(windows, all(unix, any(target_env = "gnu", target_os = "macos")))))]
fn return_addresses() -> Vec<usize> {
    Vec::new()
}

#[cfg(not(any(windows, all(unix, any(target_env = "gnu", target_os = "macos")))))]
fn function_start(_: usize) -> Option<(usize, usize)> {
    None
}

#[cfg(not(any(windows, all(unix, any(target_env = "gnu", target_os = "macos")))))]
fn image_base(_: usize) -> Option<usize> {
    None
}

/// The name table `set_functions` takes, for `functions`: each one's own
/// name (a method's without its type, as the interpreter knows it) and
/// the program line it starts on.
pub fn names_text(functions: &[kavach_kir::Function]) -> String {
    functions
        .iter()
        .map(|f| {
            let last = f.name.rsplit(' ').next().unwrap_or(&f.name);
            let own = last.rsplit('.').next().unwrap_or(last);
            format!("{own}\t{}\n", f.line)
        })
        .collect()
}
