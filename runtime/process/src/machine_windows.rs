//! The machine's facts on Windows (spec 14.25, decision 0110): the
//! system's version and the processor from the registry, the rest from
//! system calls, each `unsafe` with its `SAFETY:` comment.

#![allow(unsafe_code)]

use super::Cpu;
use windows_sys::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
};
use windows_sys::Win32::System::SystemInformation::{
    ComputerNameDnsHostname, GetComputerNameExW, GetLogicalProcessorInformation, GetTickCount64,
    GlobalMemoryStatusEx, MEMORYSTATUSEX, RelationProcessorCore,
    SYSTEM_LOGICAL_PROCESSOR_INFORMATION,
};

const VERSION: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
const PROCESSOR: &str = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// Text value `name` of registry key `key` under `HKEY_LOCAL_MACHINE`.
fn registry_text(key: &str, name: &str) -> Option<String> {
    let (key, name) = (wide(key), wide(name));
    let mut buffer = [0u16; 256];
    let mut size = std::mem::size_of_val(&buffer) as u32;
    // SAFETY: the key and name end with a zero; `buffer` has room for
    // `size` bytes, which the call lowers to what it wrote.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if status != 0 {
        return None;
    }
    let len = (size as usize / 2).min(buffer.len());
    let text = String::from_utf16_lossy(&buffer[..len]);
    Some(text.trim_end_matches('\0').trim().to_string())
}

/// Number value `name` of registry key `key` under `HKEY_LOCAL_MACHINE`.
fn registry_number(key: &str, name: &str) -> Option<u32> {
    let (key, name) = (wide(key), wide(name));
    let mut value = 0u32;
    let mut size = 4u32;
    // SAFETY: as in `registry_text`, with room for one 32-bit number.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut value as *mut u32).cast(),
            &mut size,
        )
    };
    (status == 0).then_some(value)
}

pub fn os_version() -> String {
    let major = registry_number(VERSION, "CurrentMajorVersionNumber");
    let minor = registry_number(VERSION, "CurrentMinorVersionNumber");
    let build = registry_text(VERSION, "CurrentBuild");
    match (major, minor, build) {
        (Some(a), Some(b), Some(c)) => format!("{a}.{b}.{c}"),
        _ => String::new(),
    }
}

pub fn cpu() -> Cpu {
    Cpu {
        model: registry_text(PROCESSOR, "ProcessorNameString").unwrap_or_default(),
        vendor: registry_text(PROCESSOR, "VendorIdentifier").unwrap_or_default(),
        mhz: registry_number(PROCESSOR, "~MHz").map_or(0, i64::from),
    }
}

pub fn hostname() -> String {
    let mut buffer = [0u16; 256];
    let mut len = buffer.len() as u32;
    // SAFETY: `buffer` holds `len` characters; the call writes at most
    // that many and sets `len` to the name's length.
    let ok = unsafe { GetComputerNameExW(ComputerNameDnsHostname, buffer.as_mut_ptr(), &mut len) };
    if ok == 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..(len as usize).min(buffer.len())])
}

pub fn physical_cores() -> i64 {
    let item = std::mem::size_of::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION>();
    let mut bytes = 0u32;
    // SAFETY: a null buffer asks only for the size needed.
    unsafe { GetLogicalProcessorInformation(std::ptr::null_mut(), &mut bytes) };
    let count = bytes as usize / item;
    if count == 0 {
        return 0;
    }
    // SAFETY: the info is a plain C struct, so zeroes are a valid value.
    let zero: SYSTEM_LOGICAL_PROCESSOR_INFORMATION = unsafe { std::mem::zeroed() };
    let mut items = vec![zero; count];
    // SAFETY: `items` has room for `bytes` bytes, the size just asked for.
    let ok = unsafe { GetLogicalProcessorInformation(items.as_mut_ptr(), &mut bytes) };
    if ok == 0 {
        return 0;
    }
    let filled = (bytes as usize / item).min(count);
    (items[..filled].iter())
        .filter(|i| i.Relationship == RelationProcessorCore)
        .count() as i64
}

pub fn memory() -> (i64, i64) {
    // SAFETY: the status is a plain C struct whose size is given in it.
    unsafe {
        let mut status: MEMORYSTATUSEX = std::mem::zeroed();
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut status) == 0 {
            return (0, 0);
        }
        (status.ullTotalPhys as i64, status.ullAvailPhys as i64)
    }
}

pub fn uptime_ms() -> i64 {
    // SAFETY: takes nothing, reads the system's tick count.
    unsafe { GetTickCount64() as i64 }
}
