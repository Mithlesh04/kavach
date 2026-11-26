// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Network adapters, displays, temperatures and every device on Windows
//! (spec 14.25, decision 0112): the IP helper, the display
//! configuration, performance counters and SetupAPI. Each `unsafe` call
//! has its `SAFETY:` comment.
#![allow(unsafe_code)]

use crate::parts::yes;
use crate::registry::{HKEY_LOCAL_MACHINE, text, text_at, wide};

/// Text from a zero-ended UTF-16 buffer.
fn utf16(b: &[u16]) -> String {
    String::from_utf16_lossy(&b[..b.iter().position(|c| *c == 0).unwrap_or(b.len())])
}

/// The adapters' rows, and their MAC addresses.
pub fn network() -> (Vec<Vec<String>>, Vec<String>) {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GAA_FLAG_INCLUDE_PREFIX, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows_sys::Win32::Networking::WinSock::{
        AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6,
    };
    // ERROR_BUFFER_OVERFLOW: the buffer was too small, and `size` is the
    // size the list needs now. Hosts with many adapters need more than
    // any fixed size.
    const TOO_SMALL: u32 = 111;
    let list = super::windows_procs::filled(TOO_SMALL, |words: Option<&mut [u64]>, size| {
        let buffer = words.map_or(std::ptr::null_mut(), |w| w.as_mut_ptr().cast());
        // SAFETY: `buffer` is null or has room for `size` bytes, aligned
        // for the list (8 bytes).
        unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                GAA_FLAG_INCLUDE_PREFIX,
                std::ptr::null(),
                buffer,
                size,
            )
        }
    });
    let (mut rows, mut macs) = (Vec::new(), Vec::new());
    let Ok(buffer) = list else {
        return (rows, macs);
    };
    // An empty buffer holds no list: nothing to read.
    let mut at = if buffer.is_empty() {
        std::ptr::null()
    } else {
        buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH
    };
    while !at.is_null() {
        // SAFETY: each entry of the list the call wrote into `buffer`,
        // which lives until the end of this function.
        let a = unsafe { &*at };
        let mut addresses = Vec::new();
        let mut u = a.FirstUnicastAddress;
        while !u.is_null() {
            // SAFETY: an entry of the same list; its address is read as
            // the family it says it is.
            unsafe {
                let sa = (*u).Address.lpSockaddr;
                if !sa.is_null() {
                    match (*sa).sa_family {
                        AF_INET => {
                            let v4 = &*(sa as *const SOCKADDR_IN);
                            addresses.push(
                                std::net::Ipv4Addr::from(u32::from_be(v4.sin_addr.S_un.S_addr))
                                    .to_string(),
                            );
                        }
                        AF_INET6 => {
                            let v6 = &*(sa as *const SOCKADDR_IN6);
                            addresses
                                .push(std::net::Ipv6Addr::from(v6.sin6_addr.u.Byte).to_string());
                        }
                        _ => {}
                    }
                }
                u = (*u).Next;
            }
        }
        let kind = match a.IfType {
            6 => "Ethernet",
            71 => "WiFi",
            24 => "Loopback",
            // SAFETY: the description is zero-ended text inside the list.
            _ if (unsafe { text_at(a.Description) }).contains("Bluetooth") => "Bluetooth",
            _ => "Other",
        };
        let mac_len = (a.PhysicalAddressLength as usize).min(8);
        if mac_len == 6 {
            let mac: Vec<String> = a.PhysicalAddress[..6]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            macs.push(mac.join(":"));
        }
        // SAFETY: the names are zero-ended text inside the list.
        let (name, description) = unsafe { (text_at(a.FriendlyName), text_at(a.Description)) };
        let mbps = if a.TransmitLinkSpeed == u64::MAX {
            0
        } else {
            a.TransmitLinkSpeed / 1_000_000
        };
        rows.push(vec![
            name,
            description,
            kind.to_string(),
            yes(a.OperStatus == 1),
            mbps.to_string(),
            addresses.join(","),
        ]);
        at = a.Next;
    }
    (rows, macs)
}

pub fn displays() -> Vec<Vec<String>> {
    use windows_sys::Win32::Devices::Display::{
        DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_MODE_INFO,
        DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo,
        GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
    };
    let (mut paths_n, mut modes_n) = (0u32, 0u32);
    // SAFETY: two plain numbers the call fills.
    if unsafe { GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut paths_n, &mut modes_n) }
        != 0
    {
        return Vec::new();
    }
    // SAFETY: plain C structs, valid as zeroes.
    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> =
        vec![unsafe { std::mem::zeroed() }; paths_n as usize];
    let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> =
        vec![unsafe { std::mem::zeroed() }; modes_n as usize];
    // SAFETY: the lists have room for the counts given.
    let status = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut paths_n,
            paths.as_mut_ptr(),
            &mut modes_n,
            modes.as_mut_ptr(),
            std::ptr::null_mut(),
        )
    };
    if status != 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for p in &paths[..paths_n as usize] {
        let t = &p.targetInfo;
        // SAFETY: a plain C struct, valid as zeroes; its header says what
        // to fill.
        let mut name: DISPLAYCONFIG_TARGET_DEVICE_NAME = unsafe { std::mem::zeroed() };
        name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
        name.header.size = std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
        name.header.adapterId = t.adapterId;
        name.header.id = t.id;
        // SAFETY: the header is the start of `name`, sized above.
        unsafe { DisplayConfigGetDeviceInfo(&mut name.header) };
        // SAFETY: the source's mode index is the union's plain number.
        let source = unsafe { p.sourceInfo.Anonymous.modeInfoIdx } as usize;
        let (w, h) = match modes.get(source) {
            // SAFETY: a source mode's union holds its source fields.
            Some(m) => unsafe { (m.Anonymous.sourceMode.width, m.Anonymous.sourceMode.height) },
            None => (0, 0),
        };
        let hz = (t.refreshRate.Numerator)
            .checked_div(t.refreshRate.Denominator)
            .unwrap_or(0);
        let connection = match t.outputTechnology {
            0 => "VGA",
            4 => "DVI",
            5 => "HDMI",
            10 => "DisplayPort",
            11 | i32::MIN => "internal",
            _ => "",
        };
        out.push(vec![
            utf16(&name.monitorFriendlyDeviceName),
            w.to_string(),
            h.to_string(),
            hz.to_string(),
            connection.to_string(),
        ]);
    }
    out
}

pub fn sensors() -> Vec<Vec<String>> {
    use windows_sys::Win32::System::Performance::{
        PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PdhAddEnglishCounterW, PdhCloseQuery,
        PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    };
    let mut query = 0isize;
    let mut counter = 0isize;
    let Some(path) = wide(r"\Thermal Zone Information(*)\Temperature") else {
        return Vec::new();
    };
    // SAFETY: plain handles the calls fill; `path` ends with a zero.
    unsafe {
        if PdhOpenQueryW(std::ptr::null(), 0, &mut query as *mut isize as _) != 0 {
            return Vec::new();
        }
        if PdhAddEnglishCounterW(
            query as _,
            path.as_ptr(),
            0,
            &mut counter as *mut isize as _,
        ) != 0
            || PdhCollectQueryData(query as _) != 0
        {
            PdhCloseQuery(query as _);
            return Vec::new();
        }
    }
    let (mut size, mut count) = (0u32, 0u32);
    // SAFETY: a null buffer asks only for the size needed.
    unsafe {
        PdhGetFormattedCounterArrayW(
            counter as _,
            PDH_FMT_DOUBLE,
            &mut size,
            &mut count,
            std::ptr::null_mut(),
        )
    };
    let mut buffer = vec![0u64; (size as usize).div_ceil(8) + 1];
    let mut out = Vec::new();
    // SAFETY: `buffer` has room for `size` bytes, aligned for the items.
    let ok = unsafe {
        PdhGetFormattedCounterArrayW(
            counter as _,
            PDH_FMT_DOUBLE,
            &mut size,
            &mut count,
            buffer.as_mut_ptr().cast(),
        )
    } == 0;
    if ok {
        let items = buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W;
        for i in 0..count as usize {
            // SAFETY: the call wrote `count` items, their names inside `buffer`.
            let (name, kelvin) = unsafe {
                (
                    (text_at((*items.add(i)).szName)),
                    (*items.add(i)).FmtValue.Anonymous.doubleValue,
                )
            };
            out.push(vec![
                name,
                "Temperature".to_string(),
                format!("{:.1}", kelvin - 273.15),
            ]);
        }
    }
    // SAFETY: the query opened above, closed once.
    unsafe { PdhCloseQuery(query as _) };
    out
}

/// A device class's kind (spec 14.25).
fn kind_of(class: &str) -> &'static str {
    match class.to_ascii_lowercase().as_str() {
        "bluetooth" => "bluetooth",
        "net" => "network",
        "media" | "audioendpoint" => "audio",
        "monitor" => "display",
        "display" => "gpu",
        "keyboard" => "keyboard",
        "mouse" => "mouse",
        "diskdrive" | "cdrom" | "scsiadapter" | "hdc" => "storage",
        "usb" => "usb",
        "camera" | "image" => "camera",
        "printer" | "printqueue" => "printer",
        "processor" => "processor",
        _ => "other",
    }
}

pub fn devices() -> Vec<Vec<String>> {
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        DIGCF_ALLCLASSES, DIGCF_PRESENT, SP_DEVINFO_DATA, SPDRP_CLASS, SPDRP_DEVICEDESC,
        SPDRP_DRIVER, SPDRP_FRIENDLYNAME, SPDRP_MFG, SPDRP_SERVICE, SetupDiDestroyDeviceInfoList,
        SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceRegistryPropertyW,
    };
    // SAFETY: no class, enumerator or window: every device present.
    let set = unsafe {
        SetupDiGetClassDevsW(
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
            DIGCF_ALLCLASSES | DIGCF_PRESENT,
        )
    };
    if set == -1isize as _ {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0.. {
        // SAFETY: a plain C struct with its size set, as the call asks.
        let mut data: SP_DEVINFO_DATA = unsafe { std::mem::zeroed() };
        data.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
        // SAFETY: `set` is the list made above; `data` is filled.
        if unsafe { SetupDiEnumDeviceInfo(set, i, &mut data) } == 0 {
            break;
        }
        let property = |p| {
            let mut buffer = [0u16; 512];
            // SAFETY: `buffer` has room for the bytes given.
            let ok = unsafe {
                SetupDiGetDeviceRegistryPropertyW(
                    set,
                    &data,
                    p,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr().cast(),
                    (buffer.len() * 2) as u32,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                String::new()
            } else {
                utf16(&buffer)
            }
        };
        let name = Some(property(SPDRP_FRIENDLYNAME))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| property(SPDRP_DEVICEDESC));
        let key = property(SPDRP_DRIVER);
        let version = if key.is_empty() {
            String::new()
        } else {
            text(
                HKEY_LOCAL_MACHINE,
                &format!(r"SYSTEM\CurrentControlSet\Control\Class\{key}"),
                "DriverVersion",
            )
            .unwrap_or_default()
        };
        out.push(vec![
            kind_of(&property(SPDRP_CLASS)).to_string(),
            name,
            property(SPDRP_MFG),
            property(SPDRP_SERVICE),
            version,
            yes(true),
        ]);
    }
    // SAFETY: the list made above, freed once.
    unsafe { SetupDiDestroyDeviceInfoList(set) };
    out
}
