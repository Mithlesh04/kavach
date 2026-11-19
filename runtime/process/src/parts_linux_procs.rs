// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Programs, ports and users on Linux (spec 14.25, decision 0112): the
//! programs and their sockets from `/proc`, the users from `/etc/passwd`
//! and `/etc/group`. The user's own id comes from `geteuid`, the only
//! `unsafe` here, with its `SAFETY:` comment.
#![allow(unsafe_code)]

use crate::parts::yes;
use std::collections::HashMap;
use std::fs::{read_dir, read_link, read_to_string};

fn read(path: &str) -> String {
    read_to_string(path)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Every process's id.
fn pids() -> Vec<u32> {
    let mut out: Vec<u32> = (read_dir("/proc").into_iter().flatten().flatten())
        .filter_map(|e| e.file_name().to_string_lossy().parse().ok())
        .collect();
    out.sort();
    out
}

pub fn running() -> Vec<Vec<String>> {
    // SAFETY: takes nothing and can't fail.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(1) as u64;
    (pids().into_iter())
        .filter_map(|pid| {
            let name = read(&format!("/proc/{pid}/comm"));
            if name.is_empty() {
                return None;
            }
            let path =
                read_link(format!("/proc/{pid}/exe")).map(|p| p.to_string_lossy().to_string());
            let resident: u64 = (read(&format!("/proc/{pid}/statm"))
                .split_whitespace()
                .nth(1))
            .and_then(|r| r.parse().ok())
            .unwrap_or(0);
            Some(vec![
                pid.to_string(),
                name,
                path.unwrap_or_default(),
                (resident * page).to_string(),
            ])
        })
        .collect()
}

/// Which process holds each socket, by its inode: from the links in
/// each process's `fd` folder (another user's only for root).
fn holders() -> HashMap<u64, u32> {
    let mut out = HashMap::new();
    for pid in pids() {
        for fd in read_dir(format!("/proc/{pid}/fd"))
            .into_iter()
            .flatten()
            .flatten()
        {
            let Ok(link) = read_link(fd.path()) else {
                continue;
            };
            let link = link.to_string_lossy();
            if let Some(inode) = link
                .strip_prefix("socket:[")
                .and_then(|l| l.strip_suffix(']'))
            {
                if let Ok(inode) = inode.parse() {
                    out.insert(inode, pid);
                }
            }
        }
    }
    out
}

/// An address of `/proc/net/tcp`: the IP in the kernel's byte order, in
/// hex, then the port.
fn address(hex: &str) -> (String, u16) {
    let (ip, port) = hex.split_once(':').unwrap_or(("", "0"));
    let port = u16::from_str_radix(port, 16).unwrap_or(0);
    let bytes: Vec<u8> = (0..ip.len() / 2)
        .filter_map(|i| u8::from_str_radix(&ip[2 * i..2 * i + 2], 16).ok())
        .collect();
    let ip = match bytes.len() {
        4 => std::net::Ipv4Addr::new(bytes[3], bytes[2], bytes[1], bytes[0]).to_string(),
        16 => {
            // Four 32-bit words, each in the kernel's byte order.
            let mut b = [0u8; 16];
            for (w, chunk) in bytes.chunks(4).enumerate() {
                for (i, v) in chunk.iter().rev().enumerate() {
                    b[w * 4 + i] = *v;
                }
            }
            std::net::Ipv6Addr::from(b).to_string()
        }
        _ => String::new(),
    };
    (ip, port)
}

fn state(code: &str) -> &'static str {
    match code {
        "01" => "established",
        "02" => "syn_sent",
        "03" => "syn_received",
        "04" => "fin_wait1",
        "05" => "fin_wait2",
        "06" => "time_wait",
        "07" => "close",
        "08" => "close_wait",
        "09" => "last_ack",
        "0A" => "listen",
        "0B" => "closing",
        _ => "",
    }
}

pub fn ports() -> Vec<Vec<String>> {
    let holders = holders();
    let names: HashMap<u32, String> = (pids().into_iter())
        .map(|p| (p, read(&format!("/proc/{p}/comm"))))
        .collect();
    let mut out = Vec::new();
    for (file, protocol) in [
        ("tcp", "tcp"),
        ("tcp6", "tcp"),
        ("udp", "udp"),
        ("udp6", "udp"),
    ] {
        for line in read(&format!("/proc/net/{file}")).lines().skip(1) {
            let f: Vec<&str> = line.split_whitespace().collect();
            let (Some(local), Some(st), Some(inode)) = (f.get(1), f.get(3), f.get(9)) else {
                continue;
            };
            let (ip, port) = address(local);
            let pid = inode
                .parse()
                .ok()
                .and_then(|i: u64| holders.get(&i).copied());
            out.push(vec![
                protocol.to_string(),
                ip,
                port.to_string(),
                if protocol == "tcp" {
                    state(st).to_string()
                } else {
                    String::new()
                },
                pid.map_or(String::new(), |p| p.to_string()),
                pid.and_then(|p| names.get(&p).cloned()).unwrap_or_default(),
            ]);
        }
    }
    out
}

/// The members of each group in `/etc/group`.
fn groups() -> HashMap<String, Vec<String>> {
    (read("/etc/group").lines())
        .filter_map(|l| {
            let f: Vec<&str> = l.split(':').collect();
            Some((
                f.first()?.to_string(),
                f.get(3)?
                    .split(',')
                    .filter(|m| !m.is_empty())
                    .map(str::to_string)
                    .collect(),
            ))
        })
        .collect()
}

fn is_admin(name: &str, uid: u32, groups: &HashMap<String, Vec<String>>) -> bool {
    uid == 0
        || ["sudo", "wheel", "admin"]
            .iter()
            .any(|g| groups.get(*g).is_some_and(|m| m.iter().any(|m| m == name)))
}

pub fn user() -> Vec<Vec<String>> {
    // SAFETY: takes nothing and can't fail.
    let uid = unsafe { libc::geteuid() };
    let entry = (read("/etc/passwd").lines())
        .find(|l| l.split(':').nth(2) == Some(&uid.to_string()))
        .map(str::to_string);
    let f: Vec<String> = entry
        .unwrap_or_default()
        .split(':')
        .map(str::to_string)
        .collect();
    let name = f
        .first()
        .cloned()
        .filter(|n| !n.is_empty())
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_default();
    let home = f
        .get(5)
        .cloned()
        .filter(|h| !h.is_empty())
        .or_else(|| std::env::var("HOME").ok())
        .unwrap_or_default();
    // Administrator rights now: root, not membership of `sudo`.
    vec![vec![name, home, yes(uid == 0)]]
}

/// The accounts people log in with: root, and the users the system made
/// for people (ids from 1000), with a shell that lets them in.
pub fn users() -> Vec<Vec<String>> {
    let groups = groups();
    (read("/etc/passwd").lines())
        .filter_map(|l| {
            let f: Vec<&str> = l.split(':').collect();
            let (name, uid, gecos, shell) = (
                *f.first()?,
                f.get(2)?.parse::<u32>().ok()?,
                *f.get(4)?,
                *f.get(6)?,
            );
            let person = uid == 0 || (1000..65534).contains(&uid);
            let logs_in = !shell.ends_with("nologin") && !shell.ends_with("false");
            (person && logs_in).then(|| {
                let full = gecos.split(',').next().unwrap_or("").to_string();
                vec![name.to_string(), full, yes(is_admin(name, uid, &groups))]
            })
        })
        .collect()
}
