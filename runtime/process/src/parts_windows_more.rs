//! More parts of the machine on Windows (spec 14.25, decision 0112):
//! physical disks (storage queries), network adapters (the IP helper),
//! displays (the display configuration), temperatures (performance
//! counters) and every device (SetupAPI). Each `unsafe` call has its
//! `SAFETY:` comment.
#![allow(unsafe_code)]

use super::Kind;
use crate::parts::yes;
use crate::registry::wide;

pub fn rows(kind: Kind) -> Vec<Vec<String>> {
    match kind {
        Kind::Disks => disks(),
        Kind::Network => network().0,
        Kind::Displays => displays(),
        Kind::Sensors => sensors(),
        Kind::Devices => devices(),
        _ => Vec::new(),
    }
}

pub fn macs() -> Vec<String> {
    network().1
}

fn disks() -> Vec<Vec<String>> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{
        DEVICE_SEEK_PENALTY_DESCRIPTOR, DISK_GEOMETRY_EX, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
        IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery, STORAGE_DEVICE_DESCRIPTOR,
        STORAGE_PROPERTY_QUERY, StorageDeviceProperty, StorageDeviceSeekPenaltyProperty,
    };
    let mut out = Vec::new();
    for n in 0..32 {
        let path = wide(&format!(r"\\.\PhysicalDrive{n}"));
        // SAFETY: `path` ends with a zero; no access is asked, only
        // queries, so no admin rights are needed. Closed below.
        let h = unsafe {
            CreateFileW(
                path.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };
        if h == INVALID_HANDLE_VALUE {
            continue;
        }
        let query = |id, out: &mut [u8]| -> bool {
            let q = STORAGE_PROPERTY_QUERY {
                PropertyId: id,
                QueryType: PropertyStandardQuery,
                AdditionalParameters: [0],
            };
            let mut got = 0u32;
            // SAFETY: the query is a plain struct read by size; `out` has
            // room for its length.
            unsafe {
                DeviceIoControl(
                    h,
                    IOCTL_STORAGE_QUERY_PROPERTY,
                    (&q as *const STORAGE_PROPERTY_QUERY).cast(),
                    std::mem::size_of_val(&q) as u32,
                    out.as_mut_ptr().cast(),
                    out.len() as u32,
                    &mut got,
                    std::ptr::null_mut(),
                ) != 0
            }
        };
        let mut desc = vec![0u8; 1024];
        let mut model = String::new();
        let (mut removable, mut usb) = (false, false);
        // The descriptors' fields are read as bytes at their places: their
        // flags are `bool`s here, which a driver's bytes may not be.
        let at = |bytes: &[u8], off: usize| -> u32 {
            bytes
                .get(off..off + 4)
                .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        if query(StorageDeviceProperty, &mut desc) {
            use std::mem::offset_of;
            let field = |off: u32| {
                let off = off as usize;
                if off == 0 || off >= desc.len() {
                    return String::new();
                }
                let end = desc[off..]
                    .iter()
                    .position(|b| *b == 0)
                    .map_or(desc.len(), |e| off + e);
                String::from_utf8_lossy(&desc[off..end]).trim().to_string()
            };
            let vendor = field(at(
                &desc,
                offset_of!(STORAGE_DEVICE_DESCRIPTOR, VendorIdOffset),
            ));
            let product = field(at(
                &desc,
                offset_of!(STORAGE_DEVICE_DESCRIPTOR, ProductIdOffset),
            ));
            model = format!("{vendor} {product}").trim().to_string();
            removable = desc[offset_of!(STORAGE_DEVICE_DESCRIPTOR, RemovableMedia)] != 0;
            // Bus type 7 is USB.
            usb = at(&desc, offset_of!(STORAGE_DEVICE_DESCRIPTOR, BusType)) == 7;
        }
        let mut seek = [0u8; std::mem::size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>()];
        let rotates = query(StorageDeviceSeekPenaltyProperty, &mut seek).then(|| {
            seek[std::mem::offset_of!(DEVICE_SEEK_PENALTY_DESCRIPTOR, IncursSeekPenalty)] != 0
        });
        let mut geometry = vec![0u8; 256];
        let mut got = 0u32;
        // SAFETY: `geometry` has room for the descriptor and its data.
        let sized = unsafe {
            DeviceIoControl(
                h,
                IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
                std::ptr::null(),
                0,
                geometry.as_mut_ptr().cast(),
                geometry.len() as u32,
                &mut got,
                std::ptr::null_mut(),
            ) != 0
        };
        // SAFETY: when the call worked it wrote a DISK_GEOMETRY_EX there.
        let size = if sized {
            unsafe { (*(geometry.as_ptr() as *const DISK_GEOMETRY_EX)).DiskSize }
        } else {
            0
        };
        // SAFETY: `h` was opened above and is closed once.
        unsafe { CloseHandle(h) };
        let kind = match (usb, rotates) {
            (true, _) => "Usb",
            (_, Some(true)) => "Hdd",
            (_, Some(false)) => "Ssd",
            _ => "Other",
        };
        out.push(vec![
            format!("PhysicalDrive{n}"),
            model,
            size.max(0).to_string(),
            kind.to_string(),
            yes(removable || usb),
        ]);
    }
    out
}

use super::windows_net::{devices, displays, network, sensors};
