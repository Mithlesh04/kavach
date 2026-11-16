//! Libraries loaded while running (`extern c library`, spec 10.7): a
//! library loaded by the system's own loader, `dlopen` or
//! `LoadLibraryW`, and its functions found by name. A library stays
//! loaded until the program ends, so an address found stays good. An entry
//! of its own, so only a program that loads libraries links this.

use crate::heap::release;
use crate::shapes::str_of;
use kavach_kir::{RtOp, Shape};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Each loaded library's path, by its handle, for errors.
fn paths() -> std::sync::MutexGuard<'static, HashMap<usize, String>> {
    static PATHS: OnceLock<Mutex<HashMap<usize, String>>> = OnceLock::new();
    PATHS
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Leaves `why` as a `LibraryError`, its text, for `kv_take_error`.
fn fail(why: String) -> i64 {
    let text = crate::shapes::new_text(&why, crate::heap::first_of(&Shape::Str));
    crate::rt_errors::set_error(text);
    0
}

// `CLibraryOpen`: `a` the path (owned). `CLibraryFind`: `a` the handle,
// `b` the function's name (owned).
crate::rt_data::entry!(kv_rt_clib, |op, a, b, _c, _code, _line| {
    match op {
        RtOp::CLibraryOpen => {
            let path = str_of(a);
            release(a);
            match open(&path) {
                Ok(handle) => {
                    paths().insert(handle, path);
                    handle as i64
                }
                Err(why) => fail(format!("the library {path} can't be loaded: {why}")),
            }
        }
        _ => {
            let name = str_of(b);
            release(b);
            match find(a as usize, &name) {
                Some(address) => address as i64,
                None => {
                    let path = paths().get(&(a as usize)).cloned().unwrap_or_default();
                    fail(format!("the library {path} has no function `{name}`"))
                }
            }
        }
    }
});

#[cfg(unix)]
#[allow(unsafe_code)]
fn open(path: &str) -> Result<usize, String> {
    let c = std::ffi::CString::new(path).map_err(|_| "its name has a zero byte".to_string())?;
    // SAFETY: `c` is a valid C string; loading runs the library's own
    // start-up code, which the program asked for inside `unsafe`.
    let handle = unsafe { libc::dlopen(c.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
    if !handle.is_null() {
        return Ok(handle as usize);
    }
    // SAFETY: `dlerror` gives the last error as a C string, or null.
    let why = unsafe { libc::dlerror() };
    Err(if why.is_null() {
        "the system gave no reason".into()
    } else {
        // SAFETY: a non-null `dlerror` result is a C string.
        unsafe { std::ffi::CStr::from_ptr(why) }
            .to_string_lossy()
            .into_owned()
    })
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn find(handle: usize, name: &str) -> Option<usize> {
    let c = std::ffi::CString::new(name).ok()?;
    // SAFETY: `handle` came from `dlopen` and is never closed.
    let address = unsafe { libc::dlsym(handle as *mut _, c.as_ptr()) };
    (!address.is_null()).then_some(address as usize)
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn open(path: &str) -> Result<usize, String> {
    let wide: Vec<u16> = path.encode_utf16().chain([0]).collect();
    // SAFETY: `wide` ends in a zero; loading runs the library's own
    // start-up code, which the program asked for inside `unsafe`.
    let handle = unsafe { windows_sys::Win32::System::LibraryLoader::LoadLibraryW(wide.as_ptr()) };
    if !handle.is_null() {
        return Ok(handle as usize);
    }
    // SAFETY: reads this thread's last error; no memory is touched.
    let code = unsafe { windows_sys::Win32::Foundation::GetLastError() };
    Err(std::io::Error::from_raw_os_error(code as i32).to_string())
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn find(handle: usize, name: &str) -> Option<usize> {
    let c = std::ffi::CString::new(name).ok()?;
    // SAFETY: `handle` came from `LoadLibraryW` and is never freed.
    let address = unsafe {
        windows_sys::Win32::System::LibraryLoader::GetProcAddress(handle as _, c.as_ptr().cast())
    };
    address.map(|f| f as usize)
}

#[cfg(not(any(unix, windows)))]
fn open(_: &str) -> Result<usize, String> {
    Err("this system can't load libraries yet".into())
}

#[cfg(not(any(unix, windows)))]
fn find(_: usize, _: &str) -> Option<usize> {
    None
}
