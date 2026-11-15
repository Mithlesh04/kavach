//! Changing the machine on Windows (spec 14.25): `TerminateProcess` for
//! programs, the network management calls for users, `LockWorkStation`
//! to lock. Each `unsafe` call has its `SAFETY:` comment.
#![allow(unsafe_code)]

use super::Refused;
use crate::registry::wide;

/// The refusal for Windows error `code`, about `what`.
fn refused(code: u32, what: &str) -> Refused {
    match code {
        5 => Refused::Denied(format!("{what}: this needs administrator rights")),
        87 | 2221 => Refused::NotFound(what.to_string()),
        2224 => Refused::Failed(format!("{what} already exists")),
        _ => Refused::Failed(format!(
            "{what}: {}",
            std::io::Error::from_raw_os_error(code as i32)
        )),
    }
}

pub fn terminate(pid: i64, _force: bool) -> Result<(), Refused> {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};
    let what = format!("program {pid}");
    let Ok(id) = u32::try_from(pid) else {
        return Err(Refused::NotFound(what));
    };
    // SAFETY: asks only for the right to end the process; closed below.
    let h = unsafe { OpenProcess(PROCESS_TERMINATE, 0, id) };
    if h.is_null() {
        // SAFETY: reads this thread's last error.
        return Err(refused(unsafe { GetLastError() }, &what));
    }
    // SAFETY: `h` was opened with the right to end the process.
    let ok = unsafe { TerminateProcess(h, 1) } != 0;
    // SAFETY: reads this thread's last error, before anything else runs.
    let code = unsafe { GetLastError() };
    // SAFETY: the handle opened above, closed once.
    unsafe { CloseHandle(h) };
    if ok {
        Ok(())
    } else {
        Err(refused(code, &what))
    }
}

pub fn add_user(name: &str, password: &str, admin: bool) -> Result<(), Refused> {
    use windows_sys::Win32::NetworkManagement::NetManagement::{
        LOCALGROUP_MEMBERS_INFO_3, NetLocalGroupAddMembers, NetUserAdd, UF_SCRIPT, USER_INFO_1,
        USER_PRIV_USER,
    };
    let (mut n, mut p) = (wide(name), wide(password));
    let info = USER_INFO_1 {
        usri1_name: n.as_mut_ptr(),
        usri1_password: p.as_mut_ptr(),
        usri1_password_age: 0,
        usri1_priv: USER_PRIV_USER,
        usri1_home_dir: std::ptr::null_mut(),
        usri1_comment: std::ptr::null_mut(),
        usri1_flags: UF_SCRIPT,
        usri1_script_path: std::ptr::null_mut(),
    };
    let mut bad = 0u32;
    // SAFETY: `info` points at zero-ended text that lives past the call.
    let status = unsafe {
        NetUserAdd(
            std::ptr::null(),
            1,
            (&info as *const USER_INFO_1).cast(),
            &mut bad,
        )
    };
    if status != 0 {
        return Err(refused(status, &format!("user {name}")));
    }
    if admin {
        let group =
            administrators().ok_or_else(|| Refused::Failed("no administrators' group".into()))?;
        let member = LOCALGROUP_MEMBERS_INFO_3 {
            lgrmi3_domainandname: n.as_mut_ptr(),
        };
        let g = wide(&group);
        // SAFETY: one member, its name zero-ended text that lives past the call.
        let status = unsafe {
            NetLocalGroupAddMembers(
                std::ptr::null(),
                g.as_ptr(),
                3,
                (&member as *const LOCALGROUP_MEMBERS_INFO_3).cast(),
                1,
            )
        };
        if status != 0 {
            return Err(refused(status, &format!("user {name} as an administrator")));
        }
    }
    Ok(())
}

/// The administrators' group's name, which is in the system's language:
/// from its well-known id.
fn administrators() -> Option<String> {
    use windows_sys::Win32::Security::{
        CreateWellKnownSid, LookupAccountSidW, SID_NAME_USE, WinBuiltinAdministratorsSid,
    };
    let mut sid = [0u8; 68];
    let mut size = sid.len() as u32;
    // SAFETY: `sid` has room for `size` bytes, the largest SID.
    if unsafe {
        CreateWellKnownSid(
            WinBuiltinAdministratorsSid,
            std::ptr::null_mut(),
            sid.as_mut_ptr().cast(),
            &mut size,
        )
    } == 0
    {
        return None;
    }
    let (mut name, mut domain) = ([0u16; 256], [0u16; 256]);
    let (mut n, mut d) = (name.len() as u32, domain.len() as u32);
    let mut kind: SID_NAME_USE = 0;
    // SAFETY: each buffer holds the number of characters given.
    let ok = unsafe {
        LookupAccountSidW(
            std::ptr::null(),
            sid.as_ptr().cast_mut().cast(),
            name.as_mut_ptr(),
            &mut n,
            domain.as_mut_ptr(),
            &mut d,
            &mut kind,
        )
    };
    (ok != 0).then(|| String::from_utf16_lossy(&name[..(n as usize).min(name.len())]))
}

pub fn remove_user(name: &str) -> Result<(), Refused> {
    use windows_sys::Win32::NetworkManagement::NetManagement::NetUserDel;
    let n = wide(name);
    // SAFETY: `n` is zero-ended text.
    match unsafe { NetUserDel(std::ptr::null(), n.as_ptr()) } {
        0 => Ok(()),
        code => Err(refused(code, &format!("user {name}"))),
    }
}

pub fn lock() -> Result<(), Refused> {
    use windows_sys::Win32::System::Shutdown::LockWorkStation;
    // SAFETY: takes nothing; locks this session.
    if unsafe { LockWorkStation() } != 0 {
        Ok(())
    } else {
        Err(Refused::Failed("the session couldn't be locked".into()))
    }
}
