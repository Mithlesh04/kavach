//! Every part of the machine on Windows (spec 14.25, decision 0112): the
//! board, BIOS, model and installed applications from the registry, memory
//! modules and ids from the SMBIOS tables, volumes, the battery and
//! drivers from system calls; the rest in `parts_windows_more.rs`. Each
//! `unsafe` call has its `SAFETY:` comment.
#![allow(unsafe_code)]

use super::Kind;
use crate::parts::yes;
use crate::registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, subkeys, text, wide};
use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW, GetVolumeInformationW,
};
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows_sys::Win32::System::ProcessStatus::{EnumDeviceDrivers, GetDeviceDriverBaseNameW};
use windows_sys::Win32::System::SystemInformation::GetSystemFirmwareTable;

const BIOS: &str = r"HARDWARE\DESCRIPTION\System\BIOS";

fn bios(name: &str) -> String {
    text(HKEY_LOCAL_MACHINE, BIOS, name).unwrap_or_default()
}

pub fn rows(kind: Kind) -> Vec<Vec<String>> {
    match kind {
        Kind::Board => vec![vec![
            bios("BaseBoardManufacturer"),
            bios("BaseBoardProduct"),
            bios("BaseBoardVersion"),
        ]],
        Kind::Bios => vec![vec![
            bios("BIOSVendor"),
            bios("BIOSVersion"),
            bios("BIOSReleaseDate"),
        ]],
        Kind::Model => vec![vec![
            bios("SystemManufacturer"),
            bios("SystemProductName"),
            bios("SystemFamily"),
        ]],
        Kind::MemoryModules => crate::smbios::memory_modules(&smbios()),
        Kind::Volumes => volumes(),
        Kind::Battery => battery(),
        Kind::Drivers => drivers(),
        Kind::Apps => apps(),
        Kind::Ids => {
            let (uuid, serial) = crate::smbios::ids(&smbios());
            vec![vec![uuid, serial, super::windows_more::macs().join(",")]]
        }
        other => super::windows_more::rows(other),
    }
}

/// The SMBIOS tables, without the 8 bytes Windows puts before them.
fn smbios() -> Vec<u8> {
    const RSMB: u32 = u32::from_be_bytes(*b"RSMB");
    // SAFETY: a null buffer asks only for the size needed.
    let size = unsafe { GetSystemFirmwareTable(RSMB, 0, std::ptr::null_mut(), 0) };
    if size <= 8 {
        return Vec::new();
    }
    let mut buffer = vec![0u8; size as usize];
    // SAFETY: `buffer` has the room just asked for.
    let got = unsafe { GetSystemFirmwareTable(RSMB, 0, buffer.as_mut_ptr().cast(), size) };
    buffer.truncate(got as usize);
    buffer.get(8..).map(<[u8]>::to_vec).unwrap_or_default()
}

fn volumes() -> Vec<Vec<String>> {
    let mut list = [0u16; 512];
    // SAFETY: `list` holds the given number of characters.
    let len = unsafe { GetLogicalDriveStringsW(list.len() as u32, list.as_mut_ptr()) } as usize;
    let all = String::from_utf16_lossy(&list[..len.min(list.len())]);
    let mut out = Vec::new();
    for root in all.split('\0').filter(|r| !r.is_empty()) {
        let w = wide(root);
        // SAFETY: `w` ends with a zero.
        let kind = unsafe { GetDriveTypeW(w.as_ptr()) };
        // Removable (2) and fixed (3) drives; not network shares or discs.
        if kind != 2 && kind != 3 {
            continue;
        }
        let (mut available, mut total) = (0u64, 0u64);
        // SAFETY: the outputs are plain numbers; the free total is left out.
        let ok = unsafe {
            GetDiskFreeSpaceExW(w.as_ptr(), &mut available, &mut total, std::ptr::null_mut())
        };
        if ok == 0 {
            continue;
        }
        let (mut label, mut fs) = ([0u16; 261], [0u16; 64]);
        // SAFETY: each buffer holds the number of characters given.
        unsafe {
            GetVolumeInformationW(
                w.as_ptr(),
                label.as_mut_ptr(),
                label.len() as u32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                fs.as_mut_ptr(),
                fs.len() as u32,
            )
        };
        let text = |b: &[u16]| {
            String::from_utf16_lossy(&b[..b.iter().position(|c| *c == 0).unwrap_or(b.len())])
        };
        out.push(vec![
            root.to_string(),
            text(&label),
            text(&fs),
            total.to_string(),
            available.to_string(),
            yes(kind == 2),
        ]);
    }
    out
}

fn battery() -> Vec<Vec<String>> {
    // SAFETY: the status is a plain C struct the call fills.
    let s: SYSTEM_POWER_STATUS = unsafe {
        let mut s = std::mem::zeroed();
        if GetSystemPowerStatus(&mut s) == 0 {
            return Vec::new();
        }
        s
    };
    // 128: no battery; 255: not known.
    if s.BatteryFlag & 128 != 0 || s.BatteryFlag == 255 {
        return Vec::new();
    }
    let charging = s.BatteryFlag & 8 != 0;
    let left = match s.BatteryLifeTime {
        u32::MAX => -1,
        secs if !charging => (secs / 60) as i64,
        _ => -1,
    };
    vec![vec![
        s.BatteryLifePercent.min(100).to_string(),
        yes(charging),
        yes(s.ACLineStatus == 1),
        left.to_string(),
    ]]
}

fn drivers() -> Vec<Vec<String>> {
    let mut bases = vec![std::ptr::null_mut(); 2048];
    let mut needed = 0u32;
    let room = (bases.len() * std::mem::size_of::<*mut std::ffi::c_void>()) as u32;
    // SAFETY: `bases` has room for `room` bytes of addresses.
    if unsafe { EnumDeviceDrivers(bases.as_mut_ptr(), room, &mut needed) } == 0 {
        return Vec::new();
    }
    let count = (needed as usize / std::mem::size_of::<*mut std::ffi::c_void>()).min(bases.len());
    (bases[..count].iter())
        .filter_map(|base| {
            let mut name = [0u16; 260];
            // SAFETY: `name` holds the given number of characters.
            let len =
                unsafe { GetDeviceDriverBaseNameW(*base, name.as_mut_ptr(), name.len() as u32) };
            (len > 0).then(|| {
                vec![
                    String::from_utf16_lossy(&name[..len as usize]),
                    String::new(),
                ]
            })
        })
        .collect()
}

/// Installed applications, from the uninstall lists: the machine's (64-
/// and 32-bit) and the user's. System parts and updates are left out.
fn apps() -> Vec<Vec<String>> {
    const LISTS: [&str; 2] = [
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];
    let mut out: Vec<Vec<String>> = Vec::new();
    for (root, list) in LISTS
        .iter()
        .map(|l| (HKEY_LOCAL_MACHINE, l))
        .chain([(HKEY_CURRENT_USER, &LISTS[0])])
    {
        for key in subkeys(root, list) {
            let path = format!(r"{list}\{key}");
            let field = |n: &str| text(root, &path, n).unwrap_or_default();
            let name = field("DisplayName");
            let system = crate::registry::number(root, &path, "SystemComponent") == Some(1);
            if name.is_empty() || system || !field("ParentKeyName").is_empty() {
                continue;
            }
            if out.iter().any(|r| r[0] == name) {
                continue;
            }
            out.push(vec![name, field("DisplayVersion"), field("Publisher")]);
        }
    }
    out.sort();
    out
}
