// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Reading the Windows registry (spec 14.25, decisions 0110 and 0112):
//! text and numbers by key and name, and a key's subkeys. Each `unsafe`
//! call has its `SAFETY:` comment.
#![allow(unsafe_code)]

use windows_sys::Win32::System::Registry::{
    HKEY, KEY_READ, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegCloseKey, RegEnumKeyExW, RegGetValueW,
    RegOpenKeyExW,
};

pub use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

/// `s` as zero-ended UTF-16 text for a system call; `None` when `s` holds
/// a zero itself, which would end the text early: `"guest\0x"` would
/// name the user `guest`.
pub fn wide(s: &str) -> Option<Vec<u16>> {
    (!s.contains('\0')).then(|| s.encode_utf16().chain([0]).collect())
}

/// Text value `name` of key `key` under `root`.
pub fn text(root: HKEY, key: &str, name: &str) -> Option<String> {
    let (Some(key), Some(name)) = (wide(key), wide(name)) else {
        return None;
    };
    let mut buffer = vec![0u16; 1024];
    let mut size = (buffer.len() * 2) as u32;
    // SAFETY: the key and name end with a zero; `buffer` has room for
    // `size` bytes, which the call lowers to what it wrote.
    let status = unsafe {
        RegGetValueW(
            root,
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

/// Number value `name` of key `key` under `root`.
pub fn number(root: HKEY, key: &str, name: &str) -> Option<u32> {
    let (Some(key), Some(name)) = (wide(key), wide(name)) else {
        return None;
    };
    let mut value = 0u32;
    let mut size = 4u32;
    // SAFETY: as in `text`, with room for one 32-bit number.
    let status = unsafe {
        RegGetValueW(
            root,
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

/// The names of key `key`'s subkeys under `root`.
pub fn subkeys(root: HKEY, key: &str) -> Vec<String> {
    let Some(key) = wide(key) else {
        return Vec::new();
    };
    let mut open: HKEY = std::ptr::null_mut();
    // SAFETY: `key` ends with a zero; `open` receives a handle, closed below.
    if unsafe { RegOpenKeyExW(root, key.as_ptr(), 0, KEY_READ, &mut open) } != 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0.. {
        let mut name = [0u16; 256];
        let mut len = name.len() as u32;
        // SAFETY: `name` holds `len` characters; the other outputs are
        // left out (null).
        let status = unsafe {
            RegEnumKeyExW(
                open,
                i,
                name.as_mut_ptr(),
                &mut len,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if status != 0 {
            break;
        }
        out.push(String::from_utf16_lossy(
            &name[..(len as usize).min(name.len())],
        ));
    }
    // SAFETY: `open` is the handle opened above, closed once.
    unsafe { RegCloseKey(open) };
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_for_a_system_call_never_holds_a_zero_before_its_end() {
        assert_eq!(wide("ab"), Some(vec![97, 98, 0]));
        assert_eq!(wide(""), Some(vec![0]));
        // It would end the text early: `guest\0x` would name `guest`.
        assert_eq!(wide(&format!("guest{}x", char::from(0))), None);
        assert_eq!(wide(&char::from(0).to_string()), None);
    }
}
