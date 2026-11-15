//! The machine's facts on macOS (spec 14.25, decision 0110), from
//! `sysctl`, each `unsafe` call with its `SAFETY:` comment.
#![allow(unsafe_code)]

use super::Cpu;
use std::ffi::CString;

/// The raw bytes of `sysctl` value `name`, or none.
fn sysctl(name: &str) -> Option<Vec<u8>> {
    let name = CString::new(name).ok()?;
    let mut len: libc::size_t = 0;
    // SAFETY: a null buffer asks only for the size needed.
    let asked = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if asked != 0 || len == 0 {
        return None;
    }
    let mut buffer = vec![0u8; len];
    // SAFETY: `buffer` has room for `len` bytes, the size just asked for;
    // the call lowers `len` to what it wrote.
    let got = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            buffer.as_mut_ptr().cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if got != 0 {
        return None;
    }
    buffer.truncate(len);
    Some(buffer)
}

fn text(name: &str) -> String {
    let bytes = sysctl(name).unwrap_or_default();
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

/// A number of 4 or 8 bytes, as the system keeps it.
fn number(name: &str) -> i64 {
    match sysctl(name).as_deref() {
        Some(&[a, b, c, d]) => i64::from(u32::from_ne_bytes([a, b, c, d])),
        Some(b) if b.len() == 8 => {
            i64::from_ne_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
        }
        _ => 0,
    }
}

pub fn os_version() -> String {
    text("kern.osproductversion")
}

pub fn cpu() -> Cpu {
    // Apple's own processors have no vendor or rated speed here.
    let vendor = text("machdep.cpu.vendor");
    Cpu {
        model: text("machdep.cpu.brand_string"),
        vendor: if vendor.is_empty() && std::env::consts::ARCH == "aarch64" {
            "Apple".to_string()
        } else {
            vendor
        },
        mhz: number("hw.cpufrequency") / 1_000_000,
    }
}

pub fn hostname() -> String {
    text("kern.hostname")
}

pub fn physical_cores() -> i64 {
    number("hw.physicalcpu")
}

pub fn memory() -> (i64, i64) {
    let free = number("vm.page_free_count") * number("hw.pagesize");
    (number("hw.memsize"), free)
}

pub fn uptime_ms() -> i64 {
    // `kern.boottime` is a `timeval`: seconds, then microseconds.
    let Some(b) = sysctl("kern.boottime") else {
        return 0;
    };
    if b.len() < 8 {
        return 0;
    }
    let booted = i64::from_ne_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    (now - booted).max(0) * 1000
}
