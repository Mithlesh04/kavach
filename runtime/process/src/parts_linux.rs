//! Every part of the machine on Linux (spec 14.25, decision 0112), from
//! `/sys` and `/proc`; volumes' sizes from `statvfs` and addresses from
//! `getifaddrs`, the only `unsafe` here, each with its `SAFETY:` comment.
#![allow(unsafe_code)]

use super::Kind;
use crate::parts::yes;
use std::fs::{read_dir, read_link, read_to_string};
use std::path::Path;

fn read(path: impl AsRef<Path>) -> String {
    read_to_string(path)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// The names in folder `dir`, sorted.
fn names(dir: &str) -> Vec<String> {
    let mut out: Vec<String> = (read_dir(dir).into_iter().flatten().flatten())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    out.sort();
    out
}

/// The last part of the link at `path`: a device's driver.
fn link_name(path: impl AsRef<Path>) -> String {
    (read_link(path).ok())
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_default()
}

fn dmi(field: &str) -> String {
    read(format!("/sys/class/dmi/id/{field}"))
}

pub fn rows(kind: Kind) -> Vec<Vec<String>> {
    match kind {
        Kind::Board => vec![vec![
            dmi("board_vendor"),
            dmi("board_name"),
            dmi("board_version"),
        ]],
        Kind::Bios => vec![vec![
            dmi("bios_vendor"),
            dmi("bios_version"),
            dmi("bios_date"),
        ]],
        Kind::Model => vec![vec![
            dmi("sys_vendor"),
            dmi("product_name"),
            dmi("product_family"),
        ]],
        // Only root may read the firmware's tables.
        Kind::MemoryModules => match std::fs::read("/sys/firmware/dmi/tables/DMI") {
            Ok(table) => crate::smbios::memory_modules(&table),
            Err(_) => Vec::new(),
        },
        Kind::Disks => disks(),
        Kind::Volumes => volumes(),
        Kind::Network => network(),
        Kind::Displays => displays(),
        Kind::Battery => battery(),
        Kind::Sensors => sensors(),
        Kind::Devices => devices(),
        Kind::Drivers => drivers(),
        Kind::Apps => apps(),
        Kind::Running => super::linux_procs::running(),
        Kind::Ports => super::linux_procs::ports(),
        Kind::User => super::linux_procs::user(),
        Kind::Users => super::linux_procs::users(),
        Kind::Ids => {
            let table = std::fs::read("/sys/firmware/dmi/tables/DMI").unwrap_or_default();
            let (mut uuid, serial) = crate::smbios::ids(&table);
            if uuid.is_empty() {
                uuid = dmi("product_uuid").to_uppercase();
            }
            let macs: Vec<String> = (names("/sys/class/net").iter())
                .filter(|n| *n != "lo")
                .map(|n| read(format!("/sys/class/net/{n}/address")))
                .filter(|a| !a.is_empty() && a != "00:00:00:00:00:00")
                .collect();
            vec![vec![uuid, serial, macs.join(",")]]
        }
    }
}

/// Real drives: not loop, RAM, compressed or mapped devices.
fn drive_names() -> Vec<String> {
    (names("/sys/block").into_iter())
        .filter(|n| {
            !["loop", "ram", "zram", "dm-", "md", "sr", "fd"]
                .iter()
                .any(|p| n.starts_with(p))
        })
        .collect()
}

fn disks() -> Vec<Vec<String>> {
    (drive_names().into_iter())
        .map(|n| {
            let base = format!("/sys/block/{n}");
            let usb = read_link(&base).is_ok_and(|p| p.to_string_lossy().contains("/usb"));
            let kind = match (usb, read(format!("{base}/queue/rotational")).as_str()) {
                (true, _) => "Usb",
                (_, "1") => "Hdd",
                (_, "0") => "Ssd",
                _ => "Other",
            };
            let sectors: u64 = read(format!("{base}/size")).parse().unwrap_or(0);
            vec![
                n.clone(),
                read(format!("{base}/device/model")),
                (sectors * 512).to_string(),
                kind.to_string(),
                yes(usb || read(format!("{base}/removable")) == "1"),
            ]
        })
        .collect()
}

/// The labels of devices, from `/dev/disk/by-label`.
fn labels() -> Vec<(String, String)> {
    (names("/dev/disk/by-label").into_iter())
        .filter_map(|label| {
            let target = std::fs::canonicalize(format!("/dev/disk/by-label/{label}")).ok()?;
            Some((
                target.to_string_lossy().to_string(),
                label.replace("\\x20", " "),
            ))
        })
        .collect()
}

/// Total and available bytes of the file system at `path`.
fn space(path: &str) -> (u64, u64) {
    let Ok(c) = std::ffi::CString::new(path) else {
        return (0, 0);
    };
    // SAFETY: `stats` is a plain C value the call fills; `c` ends with 0.
    let stats = unsafe {
        let mut stats: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c.as_ptr(), &mut stats) != 0 {
            return (0, 0);
        }
        stats
    };
    let block = stats.f_frsize as u64;
    (stats.f_blocks as u64 * block, stats.f_bavail as u64 * block)
}

fn volumes() -> Vec<Vec<String>> {
    let labels = labels();
    let mut out = Vec::new();
    for line in read("/proc/mounts").lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let [device, path, fs, ..] = f[..] else {
            continue;
        };
        // Disks, and `/` whatever holds it (an overlay in a container).
        if !(device.starts_with("/dev/") || path == "/")
            || out.iter().any(|r: &Vec<String>| r[0] == path)
        {
            continue;
        }
        // A file bind-mounted on its own (`/etc/hosts` in a container) is
        // no volume.
        if !std::path::Path::new(path).is_dir() {
            continue;
        }
        let path = path.replace("\\040", " ");
        let (total, available) = space(&path);
        let real = std::fs::canonicalize(device).map(|p| p.to_string_lossy().to_string());
        let real = real.unwrap_or_else(|_| device.to_string());
        let label = (labels.iter().find(|(d, _)| *d == real)).map(|(_, l)| l.clone());
        // A partition's drive says whether it can be taken out.
        let drive = (drive_names().into_iter())
            .find(|d| real.trim_start_matches("/dev/").starts_with(d.as_str()));
        let removable = drive.is_some_and(|d| {
            read(format!("/sys/block/{d}/removable")) == "1"
                || read_link(format!("/sys/block/{d}"))
                    .is_ok_and(|p| p.to_string_lossy().contains("/usb"))
        });
        out.push(vec![
            path,
            label.unwrap_or_default(),
            fs.to_string(),
            total.to_string(),
            available.to_string(),
            yes(removable),
        ]);
    }
    out
}

/// Each interface's IP addresses, from `getifaddrs`.
fn addresses() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: `getifaddrs` fills `list` with a linked list it owns, which
    // is walked while it lives and freed once with `freeifaddrs`; each
    // address is read as the family it says it is.
    unsafe {
        if libc::getifaddrs(&mut list) != 0 {
            return out;
        }
        let mut at = list;
        while !at.is_null() {
            let a = &*at;
            if !a.ifa_addr.is_null() {
                let name = std::ffi::CStr::from_ptr(a.ifa_name)
                    .to_string_lossy()
                    .to_string();
                match (*a.ifa_addr).sa_family as i32 {
                    libc::AF_INET => {
                        let v4 = &*(a.ifa_addr as *const libc::sockaddr_in);
                        let ip = std::net::Ipv4Addr::from(u32::from_be(v4.sin_addr.s_addr));
                        out.push((name, ip.to_string()));
                    }
                    libc::AF_INET6 => {
                        let v6 = &*(a.ifa_addr as *const libc::sockaddr_in6);
                        out.push((
                            name,
                            std::net::Ipv6Addr::from(v6.sin6_addr.s6_addr).to_string(),
                        ));
                    }
                    _ => {}
                }
            }
            at = a.ifa_next;
        }
        libc::freeifaddrs(list);
    }
    out
}

fn network() -> Vec<Vec<String>> {
    let addresses = addresses();
    (names("/sys/class/net").into_iter())
        .map(|n| {
            let base = format!("/sys/class/net/{n}");
            let kind = if n == "lo" {
                "Loopback"
            } else if Path::new(&format!("{base}/wireless")).exists() {
                "WiFi"
            } else if n.starts_with("bn") || read(format!("{base}/type")) == "32" {
                "Bluetooth"
            } else if read(format!("{base}/type")) == "1" {
                "Ethernet"
            } else {
                "Other"
            };
            let mine: Vec<&str> = (addresses.iter().filter(|(a, _)| *a == n))
                .map(|(_, ip)| ip.as_str())
                .collect();
            let speed: i64 = read(format!("{base}/speed")).parse().unwrap_or(0);
            vec![
                n.clone(),
                link_name(format!("{base}/device/driver")),
                kind.to_string(),
                yes(read(format!("{base}/operstate")) == "up" || n == "lo"),
                speed.max(0).to_string(),
                mine.join(","),
            ]
        })
        .collect()
}

use super::linux_more::{apps, battery, devices, displays, drivers, sensors};
