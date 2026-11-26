// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Programs, ports and users on Windows (spec 14.25, decision 0112): a
//! Toolhelp snapshot of the processes, the IP helper's tables of
//! connections with their owners, the process token, and the network
//! management calls for accounts. Each `unsafe` call has its `SAFETY:`.
#![allow(unsafe_code)]

use crate::parts::yes;
use crate::registry::text_at;
use std::collections::HashMap;

fn utf16(b: &[u16]) -> String {
    String::from_utf16_lossy(&b[..b.iter().position(|c| *c == 0).unwrap_or(b.len())])
}

/// Each process's id and name.
fn processes() -> Vec<(u32, String)> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    // SAFETY: a snapshot of the processes, closed below.
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snap == INVALID_HANDLE_VALUE {
        return Vec::new();
    }
    let mut out = Vec::new();
    // SAFETY: a plain C struct with its size set, as the calls ask.
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    // SAFETY: `snap` is the snapshot above; `entry` is filled each time.
    let mut more = unsafe { Process32FirstW(snap, &mut entry) } != 0;
    while more {
        out.push((entry.th32ProcessID, utf16(&entry.szExeFile)));
        // SAFETY: as above.
        more = unsafe { Process32NextW(snap, &mut entry) } != 0;
    }
    // SAFETY: the snapshot taken above, closed once.
    unsafe { CloseHandle(snap) };
    out
}

pub fn running() -> Vec<Vec<String>> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    (processes().into_iter())
        .map(|(pid, name)| {
            let (mut path, mut memory) = (String::new(), 0u64);
            // SAFETY: asks only to read the process's details; closed below.
            let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
            if !h.is_null() {
                let mut buffer = [0u16; 1024];
                let mut len = buffer.len() as u32;
                // SAFETY: `buffer` holds `len` characters.
                if unsafe { QueryFullProcessImageNameW(h, 0, buffer.as_mut_ptr(), &mut len) } != 0 {
                    path = String::from_utf16_lossy(&buffer[..(len as usize).min(buffer.len())]);
                }
                // SAFETY: a plain C struct whose size is given.
                let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
                let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                // SAFETY: `h` may be read; `counters` is filled.
                if unsafe { GetProcessMemoryInfo(h, &mut counters, size) } != 0 {
                    memory = counters.WorkingSetSize as u64;
                }
                // SAFETY: the handle opened above, closed once.
                unsafe { CloseHandle(h) };
            }
            vec![pid.to_string(), name, path, memory.to_string()]
        })
        .collect()
}

/// A table of the IP helper, as 32-bit words: its count, then its rows;
/// or the error that kept it from being read.
fn table(tcp: bool, family: u32) -> Result<Vec<u32>, u32> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
    };
    filled(TOO_SMALL, |words: Option<&mut [u32]>, size: &mut u32| {
        let buffer = words.map_or(std::ptr::null_mut(), |w| w.as_mut_ptr().cast());
        // SAFETY: `buffer` is null or has room for `size` bytes.
        unsafe {
            if tcp {
                GetExtendedTcpTable(buffer, size, 0, family, TCP_TABLE_OWNER_PID_ALL, 0)
            } else {
                GetExtendedUdpTable(buffer, size, 0, family, UDP_TABLE_OWNER_PID, 0)
            }
        }
    })
}

/// ERROR_INSUFFICIENT_BUFFER: the buffer given was too small.
const TOO_SMALL: u32 = 122;

/// What `call` fills, as the IP helper fills a table: first with no
/// buffer, to learn the size in bytes, then with a buffer of that size.
/// The table can grow between the calls, as connections open, so while
/// the call says `too_small`, it is asked again with the size it gives
/// and some room to spare, a few times. Then, or on any other error, the
/// error: never an empty table, which would say nothing is there.
pub(super) fn filled<W: Copy + Default>(
    too_small: u32,
    mut call: impl FnMut(Option<&mut [W]>, &mut u32) -> u32,
) -> Result<Vec<W>, u32> {
    let word = std::mem::size_of::<W>();
    let mut size = 0u32;
    match call(None, &mut size) {
        0 => return Ok(Vec::new()),
        e if e != too_small => return Err(e),
        _ => {}
    }
    for _ in 0..4 {
        let bytes = size as usize + size as usize / 8 + 256;
        let mut words = vec![W::default(); bytes.div_ceil(word)];
        let mut room = (words.len() * word) as u32;
        match call(Some(&mut words), &mut room) {
            0 => return Ok(words),
            e if e != too_small => return Err(e),
            // The call wrote the size it needs now.
            _ => size = room.max(size),
        }
    }
    Err(too_small)
}

fn tcp_state(s: u32) -> &'static str {
    match s {
        1 => "close",
        2 => "listen",
        3 => "syn_sent",
        4 => "syn_received",
        5 => "established",
        6 => "fin_wait1",
        7 => "fin_wait2",
        8 => "close_wait",
        9 => "closing",
        10 => "last_ack",
        11 => "time_wait",
        _ => "",
    }
}

/// A port as the tables keep it: in network order, in a 32-bit word.
fn port(w: u32) -> u16 {
    u16::from_be(w as u16)
}

/// Sixteen bytes of an IPv6 address from four words.
fn v6(w: &[u32]) -> std::net::Ipv6Addr {
    let mut b = [0u8; 16];
    for (i, word) in w.iter().take(4).enumerate() {
        b[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    std::net::Ipv6Addr::from(b)
}

/// The ports' rows, or why a table couldn't be read.
pub fn ports() -> Result<Vec<Vec<String>>, String> {
    use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
    let names: HashMap<u32, String> = processes().into_iter().collect();
    let mut out = Vec::new();
    let mut push = |protocol: &str, ip: String, p: u16, state: &str, pid: u32| {
        let program = names.get(&pid).cloned().unwrap_or_default();
        out.push(vec![
            protocol.to_string(),
            ip,
            p.to_string(),
            state.to_string(),
            pid.to_string(),
            program,
        ]);
    };
    // Each table's rows, by their width in words (the `MIB_*_OWNER_PID` rows).
    for (tcp, family, width) in [
        (true, AF_INET, 6),
        (true, AF_INET6, 14),
        (false, AF_INET, 3),
        (false, AF_INET6, 7),
    ] {
        let t = table(tcp, family as u32).map_err(|e| {
            let e = std::io::Error::from_raw_os_error(e as i32);
            format!("the table of ports couldn't be read: {e}")
        })?;
        let count = t.first().copied().unwrap_or(0) as usize;
        for r in (0..count).filter_map(|i| t.get(1 + i * width..1 + (i + 1) * width)) {
            match (tcp, family == AF_INET) {
                (true, true) => push(
                    "tcp",
                    std::net::Ipv4Addr::from(u32::from_be(r[1])).to_string(),
                    port(r[2]),
                    tcp_state(r[0]),
                    r[5],
                ),
                (true, false) => push(
                    "tcp",
                    v6(&r[0..4]).to_string(),
                    port(r[5]),
                    tcp_state(r[12]),
                    r[13],
                ),
                (false, true) => push(
                    "udp",
                    std::net::Ipv4Addr::from(u32::from_be(r[0])).to_string(),
                    port(r[1]),
                    "",
                    r[2],
                ),
                (false, false) => push("udp", v6(&r[0..4]).to_string(), port(r[5]), "", r[6]),
            }
        }
    }
    Ok(out)
}

/// Whether this process runs elevated: with administrator rights now.
fn elevated() -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    let mut token = std::ptr::null_mut();
    // SAFETY: the current process's token, opened to read; closed below.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return false;
    }
    let mut e = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut got = 0u32;
    let size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
    // SAFETY: `e` is the struct this class fills, sized above.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            (&mut e as *mut TOKEN_ELEVATION).cast(),
            size,
            &mut got,
        )
    } != 0;
    // SAFETY: the token opened above, closed once.
    unsafe { CloseHandle(token) };
    ok && e.TokenIsElevated != 0
}

pub fn user() -> Vec<Vec<String>> {
    use windows_sys::Win32::System::WindowsProgramming::GetUserNameW;
    let mut name = [0u16; 257];
    let mut len = name.len() as u32;
    // SAFETY: `name` holds `len` characters.
    let ok = unsafe { GetUserNameW(name.as_mut_ptr(), &mut len) } != 0;
    let name = if ok {
        utf16(&name)
    } else {
        std::env::var("USERNAME").unwrap_or_default()
    };
    vec![vec![
        name,
        std::env::var("USERPROFILE").unwrap_or_default(),
        yes(elevated()),
    ]]
}

/// Each enabled account's name and full name (`NetUserEnum` at level
/// 20), and the names of the administrators (level 1).
pub fn users() -> Vec<Vec<String>> {
    use windows_sys::Win32::NetworkManagement::NetManagement::{
        FILTER_NORMAL_ACCOUNT, MAX_PREFERRED_LENGTH, NetApiBufferFree, NetUserEnum,
        UF_ACCOUNTDISABLE, USER_INFO_1, USER_INFO_20, USER_PRIV_ADMIN,
    };
    /// An account's details at one level of `NetUserEnum`: only the pairs
    /// below, so `each` can't read one level's entries as another's.
    trait Level {
        const LEVEL: u32;
    }
    impl Level for USER_INFO_20 {
        const LEVEL: u32 = 20;
    }
    impl Level for USER_INFO_1 {
        const LEVEL: u32 = 1;
    }
    /// Calls `f` with the `read` entries of `T`'s level, then frees them.
    fn each<T: Level>(mut f: impl FnMut(&T)) {
        let mut buffer: *mut u8 = std::ptr::null_mut();
        let (mut read, mut total, mut resume) = (0u32, 0u32, 0u32);
        // SAFETY: the call allocates `buffer` with `read` entries of
        // `T::LEVEL`, which `T` is (`Level` ties the two); they are read
        // only before `NetApiBufferFree` frees it, whatever the status.
        unsafe {
            let status = NetUserEnum(
                std::ptr::null(),
                T::LEVEL,
                FILTER_NORMAL_ACCOUNT,
                &mut buffer,
                MAX_PREFERRED_LENGTH,
                &mut read,
                &mut total,
                &mut resume,
            );
            if buffer.is_null() {
                return;
            }
            if status == 0 {
                std::slice::from_raw_parts(buffer as *const T, read as usize)
                    .iter()
                    .for_each(&mut f);
            }
            NetApiBufferFree(buffer.cast());
        }
    }
    let mut rows = Vec::new();
    each(|u: &USER_INFO_20| {
        if u.usri20_flags & UF_ACCOUNTDISABLE == 0 {
            // SAFETY: the names are zero-ended text inside the buffer.
            rows.push(unsafe { (text_at(u.usri20_name), text_at(u.usri20_full_name)) });
        }
    });
    let mut admins = Vec::new();
    each(|u: &USER_INFO_1| {
        if u.usri1_priv == USER_PRIV_ADMIN {
            // SAFETY: as above.
            admins.push(unsafe { text_at(u.usri1_name) });
        }
    });
    (rows.into_iter())
        .map(|(name, full)| {
            let admin = admins.contains(&name);
            vec![name, full, yes(admin)]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake IP helper: its table has `rows` one-word rows after the
    /// count. Each time it says the buffer is too small, the next number
    /// of `opened` rows arrive, as connections open between the calls.
    fn helper(
        mut rows: u32,
        opened: &[u32],
    ) -> impl FnMut(Option<&mut [u32]>, &mut u32) -> u32 + '_ {
        let mut opened = opened.iter();
        move |words, size| {
            let need = (rows + 1) * 4;
            match words {
                Some(w) if *size >= need => {
                    w[0] = rows;
                    0
                }
                _ => {
                    *size = need;
                    rows += opened.next().copied().unwrap_or(0);
                    TOO_SMALL
                }
            }
        }
    }

    #[test]
    fn a_table_that_grows_between_the_calls_is_asked_for_again() {
        let count = |opened| filled(TOO_SMALL, helper(10, opened)).map(|t| t.first().copied());
        assert_eq!(count(&[]), Ok(Some(10)));
        // A few connections open after the size is asked for.
        assert_eq!(count(&[20]), Ok(Some(30)));
        // Many open: the call says the size it needs, and is asked again.
        assert_eq!(count(&[1000]), Ok(Some(1010)));
        assert_eq!(count(&[1000, 2000]), Ok(Some(3010)));
    }

    #[test]
    fn a_table_that_never_fits_is_an_error_not_an_empty_table() {
        // Each try finds many more rows: the tries run out.
        assert_eq!(
            filled(TOO_SMALL, helper(10, &[100_000; 16])),
            Err(TOO_SMALL)
        );
        // Any other error is given at once.
        assert_eq!(filled::<u32>(TOO_SMALL, |_, _| 50), Err(50));
    }

    #[test]
    fn the_adapter_list_is_asked_for_again_too() {
        // `GetAdaptersAddresses`: 64-bit words, ERROR_BUFFER_OVERFLOW (111).
        let mut need = 32 * 1024 + 8;
        let list = filled(111, |words: Option<&mut [u64]>, size| match words {
            Some(w) if *size >= need => {
                w[0] = 7;
                0
            }
            _ => {
                *size = need;
                need += 4096;
                111
            }
        });
        assert_eq!(list.map(|l| l[0]), Ok(7));
    }

    #[test]
    fn this_machines_ports_are_read() {
        let ports = ports().expect("every table read");
        assert!(ports.iter().all(|r| r.len() == 6));
    }
}
