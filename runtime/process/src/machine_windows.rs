// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The machine's facts on Windows (spec 14.25, decision 0110): the
//! system's version and the processor from the registry (`registry.rs`),
//! the rest from system calls, each `unsafe` with its `SAFETY:` comment.

#![allow(unsafe_code)]

use super::Cpu;
use windows_sys::Win32::System::SystemInformation::{
    ComputerNameDnsHostname, GetComputerNameExW, GetLogicalProcessorInformation, GetTickCount64,
    GlobalMemoryStatusEx, MEMORYSTATUSEX, RelationProcessorCore,
    SYSTEM_LOGICAL_PROCESSOR_INFORMATION,
};

const VERSION: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
const PROCESSOR: &str = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";

fn registry_text(key: &str, name: &str) -> Option<String> {
    crate::registry::text(crate::registry::HKEY_LOCAL_MACHINE, key, name)
}

fn registry_number(key: &str, name: &str) -> Option<u32> {
    crate::registry::number(crate::registry::HKEY_LOCAL_MACHINE, key, name)
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
