//! Every part of the machine (spec 14.25, decision 0112): each kind of
//! part as rows of text, a fixed number of fields a row, which
//! `stdlib/system/system.kav` turns into records. Each system's code is in
//! `parts_<system>.rs`; what a system doesn't give is an empty list.

#[cfg(target_os = "linux")]
#[path = "parts_linux.rs"]
mod facts;
#[cfg(windows)]
#[path = "parts_windows.rs"]
mod facts;
#[cfg(target_os = "linux")]
#[path = "parts_linux_more.rs"]
mod linux_more;
#[cfg(target_os = "linux")]
#[path = "parts_linux_procs.rs"]
mod linux_procs;
#[cfg(windows)]
#[path = "parts_windows_more.rs"]
mod windows_more;
#[cfg(windows)]
#[path = "parts_windows_net.rs"]
mod windows_net;
#[cfg(windows)]
#[path = "parts_windows_procs.rs"]
mod windows_procs;
// macOS and the others: nothing yet (spec 14.25's open questions).
#[cfg(not(any(target_os = "linux", windows)))]
mod facts {
    pub fn rows(_kind: super::Kind) -> Vec<Vec<String>> {
        Vec::new()
    }
}

/// The kinds of part, by the number `system.kav` asks with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Maker, product, version.
    Board,
    /// Maker, version, date.
    Bios,
    /// Maker, product, family.
    Model,
    /// Slot, size in bytes, kind, MHz, maker, part.
    MemoryModules,
    /// Name, model, size in bytes, kind (`Ssd`, `Hdd`, `Usb`, `Other`), removable.
    Disks,
    /// Path, label, file system, total, available, removable.
    Volumes,
    /// Name, description, kind, connected, Mbps, addresses (comma between).
    Network,
    /// Name, width, height, Hz, connection.
    Displays,
    /// Percent, charging, on power, minutes left; no row without a battery.
    Battery,
    /// Name, kind (`Temperature`, `Fan`, `Voltage`, `Power`), value.
    Sensors,
    /// Kind, name, maker, driver, driver version, connected.
    Devices,
    /// Name, version.
    Drivers,
    /// Name, version, maker.
    Apps,
    /// System UUID, board serial, MAC addresses (comma between).
    Ids,
    /// Pid, name, path, resident bytes.
    Running,
    /// Protocol, address, port, state, pid, program.
    Ports,
    /// Name, home, admin now.
    User,
    /// Name, full name, admin.
    Users,
}

const KINDS: [(Kind, usize); 18] = [
    (Kind::Board, 3),
    (Kind::Bios, 3),
    (Kind::Model, 3),
    (Kind::MemoryModules, 6),
    (Kind::Disks, 5),
    (Kind::Volumes, 6),
    (Kind::Network, 6),
    (Kind::Displays, 5),
    (Kind::Battery, 4),
    (Kind::Sensors, 3),
    (Kind::Devices, 6),
    (Kind::Drivers, 2),
    (Kind::Apps, 3),
    (Kind::Ids, 3),
    (Kind::Running, 4),
    (Kind::Ports, 6),
    (Kind::User, 3),
    (Kind::Users, 3),
];

/// The kinds only `System.critical.read` may read (spec 14.25).
pub const CRITICAL: [i64; 5] = [12, 13, 14, 15, 17];

/// Every row of kind number `which`, field after field, each row padded
/// or cut to its kind's width.
pub fn rows(which: i64) -> Vec<String> {
    let Some(&(kind, width)) = KINDS.get(which.max(0) as usize).filter(|_| which >= 0) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for mut row in facts::rows(kind) {
        row.resize(width, String::new());
        out.extend(
            row.into_iter()
                .map(|f| f.replace('\0', "").trim().to_string()),
        );
    }
    out
}

/// A yes or no as a row's field.
pub(crate) fn yes(b: bool) -> String {
    if b { "true" } else { "false" }.to_string()
}
